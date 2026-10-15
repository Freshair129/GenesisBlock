//! Process-level recovery checks for the standalone REST executable.
//!
//! This target requires Cargo feature `bins`; the server binary is built from
//! the same source/profile as the integration test and is started as a child
//! process against a test-owned temporary database.

use anyhow::{anyhow, Context, Result};
use genesis_block_native::{
    EdgeInput, HybridSearchInput, NodeInput, OpenOptions, QueryInput, Storage,
};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

const NODE_COUNT: usize = 4;
const COLLECTION: &str = "restart-fixture";
const RELATION: &str = "GENERATED";

fn props(index: usize) -> Value {
    json!({
        "owner": format!("node-{index}"),
        "text": format!("เพลงทดสอบ-{index}"),
        "seed": 100 + index,
    })
}

fn edge_props() -> Value {
    json!({"owner": "node-0", "text": "เพลงทดสอบ-edge"})
}

fn free_loopback_port() -> Result<u16> {
    Ok(TcpListener::bind(("127.0.0.1", 0))?.local_addr()?.port())
}

fn http_request(
    address: &str,
    method: &str,
    path: &str,
    body: Option<&Value>,
    api_key: Option<&str>,
) -> Result<(u16, Value)> {
    let encoded_body = body
        .map(serde_json::to_vec)
        .transpose()?
        .unwrap_or_default();
    let mut stream = TcpStream::connect(address)
        .with_context(|| format!("connect to standalone server at {address}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(8)))?;
    stream.set_write_timeout(Some(Duration::from_secs(8)))?;

    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nContent-Length: {}\r\n",
        encoded_body.len()
    );
    if body.is_some() {
        request.push_str("Content-Type: application/json; charset=utf-8\r\n");
    }
    if let Some(api_key) = api_key {
        request.push_str(&format!("Authorization: Bearer {api_key}\r\n"));
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes())?;
    stream.write_all(&encoded_body)?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| anyhow!("HTTP response has no header terminator"))?;
    let headers = String::from_utf8_lossy(&response[..header_end]);
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| anyhow!("HTTP response has no status code: {headers}"))?
        .parse::<u16>()?;
    let body = &response[header_end + 4..];
    let body = if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(body)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(body).into_owned()))
    };
    Ok((status, body))
}

fn post_json(address: &str, path: &str, api_key: &str, body: &Value) -> Result<Value> {
    let (status, response) = http_request(address, "POST", path, Some(body), Some(api_key))?;
    if status != 200 {
        return Err(anyhow!("POST {path} returned HTTP {status}: {response}"));
    }
    Ok(response)
}

struct ServerChild {
    child: Child,
    stopped: bool,
}

impl ServerChild {
    fn hard_stop(&mut self) -> Result<()> {
        if self.child.try_wait()?.is_none() {
            self.child
                .kill()
                .context("force-stop test-owned server child")?;
        }
        self.child.wait().context("wait for stopped server child")?;
        self.stopped = true;
        Ok(())
    }
}

impl Drop for ServerChild {
    fn drop(&mut self) {
        if !self.stopped && self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn start_server(data_dir: &std::path::Path, port: u16, api_key: &str) -> Result<ServerChild> {
    let binary = env!("CARGO_BIN_EXE_genesis-db-server");
    let mut command = Command::new(binary);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let child = command
        .env("GENESIS_DATA_DIR", data_dir)
        .env("GENESIS_HOST", "127.0.0.1")
        .env("GENESIS_PORT", port.to_string())
        .env("GENESIS_API_KEY", api_key)
        .env_remove("GENESIS_RETENTION")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("start standalone server executable {binary}"))?;
    Ok(ServerChild {
        child,
        stopped: false,
    })
}

fn wait_until_ready(server: &mut ServerChild, address: &str, api_key: &str) -> Result<()> {
    for _ in 0..150 {
        if let Some(status) = server.child.try_wait()? {
            return Err(anyhow!("server exited before readiness with {status}"));
        }
        if let Ok((status, _)) = http_request(address, "GET", "/v1/status", None, Some(api_key)) {
            if status == 200 {
                return Ok(());
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err(anyhow!("server did not become ready at {address}"))
}

fn search_one(address: &str, api_key: &str, index: usize) -> Result<Value> {
    let vector: Vec<u8> = (0..NODE_COUNT)
        .map(|axis| u8::from(axis == index))
        .collect();
    let query_vector: Vec<f64> = vector.into_iter().map(f64::from).collect();
    let body = json!({
        "query_vector": query_vector,
        "k": 1,
        "alpha": 0,
        "collection": COLLECTION,
    });

    for _ in 0..80 {
        let rows = post_json(address, "/v1/search/hybrid", api_key, &body)?;
        if rows.as_array().is_some_and(|rows| !rows.is_empty()) {
            return Ok(rows);
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err(anyhow!("vector search remained empty for node-{index}"))
}

fn verify_rest_fixture(address: &str, api_key: &str) -> Result<()> {
    for index in 0..NODE_COUNT {
        let rows = search_one(address, api_key, index)?;
        let row = rows
            .as_array()
            .and_then(|rows| rows.first())
            .ok_or_else(|| anyhow!("search for node-{index} returned no first row"))?;
        assert_eq!(row["node"]["id"], format!("node-{index}"));
        assert_eq!(row["node"]["props"], props(index));
    }

    let query = json!({"from": "node-0", "rel": RELATION});
    let edges = post_json(address, "/v1/query", api_key, &query)?;
    let edge = edges
        .as_array()
        .and_then(|edges| edges.iter().find(|edge| edge["id"] == "lineage-0-1"))
        .ok_or_else(|| anyhow!("lineage-0-1 was not returned: {edges}"))?;
    assert_eq!(edge["from"], "node-0");
    assert_eq!(edge["to"], "node-1");
    assert_eq!(edge["rel"], RELATION);
    assert_eq!(edge["props"], edge_props());
    Ok(())
}

fn open_core(path: &std::path::Path) -> Result<Storage> {
    Ok(Storage::open(OpenOptions {
        path: path.to_string_lossy().into_owned(),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(NODE_COUNT as u32),
        retention: None,
    })?)
}

fn add_core_fixture(storage: &Storage) -> Result<()> {
    for index in 0..NODE_COUNT {
        let mut embedding = vec![0.0; NODE_COUNT];
        embedding[index] = 1.0;
        storage.add_node(NodeInput {
            id: Some(format!("node-{index}")),
            labels: vec!["Asset".to_string()],
            props: Some(props(index)),
            embedding: Some(embedding),
            lang: Some("th".to_string()),
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })?;
    }
    storage.add_edge(EdgeInput {
        id: Some("lineage-0-1".to_string()),
        from: "node-0".to_string(),
        to: "node-1".to_string(),
        rel: RELATION.to_string(),
        props: Some(edge_props()),
        valid_from: None,
        supersede: None,
        impact: None,
        caused_by: None,
    })?;
    Ok(())
}

fn verify_core_fixture(storage: &Storage) -> Result<()> {
    for index in 0..NODE_COUNT {
        let node = storage
            .node_view(&format!("node-{index}"))
            .ok_or_else(|| anyhow!("node-{index} was not present after reopening"))?;
        assert_eq!(node.props, props(index));

        let mut query_vector = vec![0.0; NODE_COUNT];
        query_vector[index] = 1.0;
        let rows = storage.hybrid_search(HybridSearchInput {
            query_vector,
            k: 1,
            alpha: Some(0.0),
            lang: None,
            as_of: None,
            collection: None,
            ef_search: None,
            oversample: None,
        })?;
        assert_eq!(
            rows.first().map(|row| row.node.id.as_str()),
            Some(format!("node-{index}").as_str())
        );
        assert_eq!(rows[0].node.props, props(index));
    }

    let edges = storage.query(QueryInput {
        from: Some("node-0".to_string()),
        to: Some("node-1".to_string()),
        rel: Some(RELATION.to_string()),
        as_of: None,
        include_invalid: None,
        limit: None,
    })?;
    let edge = edges
        .iter()
        .find(|edge| edge.id == "lineage-0-1")
        .ok_or_else(|| anyhow!("lineage-0-1 was not present after reopening"))?;
    assert_eq!(edge.from, "node-0");
    assert_eq!(edge.to, "node-1");
    assert_eq!(edge.rel, RELATION);
    assert_eq!(edge.props, edge_props());
    Ok(())
}

#[test]
fn graceful_storage_close_and_reopen_preserves_nodes_vectors_and_edges() -> Result<()> {
    let database = TempDir::new()?;
    {
        let storage = open_core(database.path())?;
        add_core_fixture(&storage)?;
        storage.flush_index();
    }

    let reopened = open_core(database.path())?;
    reopened.flush_index();
    verify_core_fixture(&reopened)
}

#[test]
fn standalone_rest_data_survives_forced_server_process_restart() -> Result<()> {
    const FRESH_FIXTURES: usize = 3;
    for fixture_index in 0..FRESH_FIXTURES {
        verify_forced_restart_fixture(fixture_index)?;
    }
    Ok(())
}

fn verify_forced_restart_fixture(fixture_index: usize) -> Result<()> {
    let fixture = TempDir::new()?;
    let database = fixture.path().join("database");
    std::fs::create_dir_all(&database)?;
    let port = free_loopback_port()?;
    let address = format!("127.0.0.1:{port}");
    let api_key = format!("restart-test-{}-{fixture_index}", std::process::id());

    let mut first = start_server(&database, port, &api_key)?;
    wait_until_ready(&mut first, &address, &api_key)?;
    let (unauthorized_status, _) = http_request(&address, "GET", "/v1/status", None, None)?;
    assert_eq!(unauthorized_status, 401);

    let collection = post_json(
        &address,
        "/v1/collection/create",
        &api_key,
        &json!({
            "name": COLLECTION,
            "model": "synthetic-fixture",
            "dim": NODE_COUNT,
            "metric": "cosine",
        }),
    )?;
    assert_eq!(collection["ok"], true);

    for index in 0..NODE_COUNT {
        let mut embedding = vec![0; NODE_COUNT];
        embedding[index] = 1;
        let created = post_json(
            &address,
            "/v1/node/add",
            &api_key,
            &json!({
                "id": format!("node-{index}"),
                "labels": ["Asset"],
                "props": props(index),
                "embedding": embedding,
                "lang": "th",
                "collection": COLLECTION,
            }),
        )?;
        assert_eq!(created["id"], format!("node-{index}"));
        assert_eq!(created["props"], props(index));
    }

    let edge = post_json(
        &address,
        "/v1/edge/add",
        &api_key,
        &json!({
            "id": "lineage-0-1",
            "from": "node-0",
            "to": "node-1",
            "rel": RELATION,
            "props": edge_props(),
        }),
    )?;
    assert_eq!(edge["id"], "lineage-0-1");
    assert_eq!(edge["from"], "node-0");
    assert_eq!(edge["to"], "node-1");
    assert_eq!(edge["rel"], RELATION);
    assert_eq!(edge["props"], edge_props());

    verify_rest_fixture(&address, &api_key)?;
    first.hard_stop()?;

    let mut restarted = start_server(&database, port, &api_key)?;
    wait_until_ready(&mut restarted, &address, &api_key)?;
    verify_rest_fixture(&address, &api_key)?;
    restarted.hard_stop()?;
    Ok(())
}
