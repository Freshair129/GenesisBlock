use std::time::Duration;

use genesis_block_native::{NamedQueryRequest, STUDIO_SCENE_NODE_CEILING};
use reqwest::{
    header::{HeaderValue, AUTHORIZATION},
    redirect::Policy,
    Client, Method, Url,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

const MAX_REMOTE_ENTITY_ID_BYTES: usize = 4096;
const MAX_REMOTE_TOKEN_BYTES: usize = 4096;
const MAX_REMOTE_JSON_RESPONSE_BYTES: usize = 32 * 1024 * 1024;
const MAX_REMOTE_ERROR_RESPONSE_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub struct RemoteConnection {
    base_url: Url,
    authorization: Arc<Mutex<Option<HeaderValue>>>,
    client: Client,
    cancellation: CancellationToken,
}

#[derive(Default)]
pub struct RemoteState {
    inner: Mutex<RemoteSlot>,
}

#[derive(Default)]
struct RemoteSlot {
    generation: u64,
    connection: Option<RemoteConnection>,
}

impl RemoteState {
    pub fn begin_reconfigure(&self) -> Result<u64, String> {
        let mut slot = self.lock()?;
        slot.generation = slot
            .generation
            .checked_add(1)
            .ok_or_else(|| "Studio remote configuration generation overflowed".to_string())?;
        if let Some(connection) = slot.connection.take() {
            connection.cancel();
        }
        Ok(slot.generation)
    }

    pub fn install(&self, generation: u64, connection: RemoteConnection) -> Result<(), String> {
        let mut slot = self.lock()?;
        if slot.generation != generation {
            connection.cancel();
            return Err("Remote configuration was superseded before it completed".to_string());
        }
        if let Some(previous) = slot.connection.replace(connection) {
            previous.cancel();
        }
        Ok(())
    }

    pub fn connection(&self) -> Result<RemoteConnection, String> {
        self.lock()?
            .connection
            .clone()
            .ok_or_else(|| "No remote GenesisBlockDB is configured".to_string())
    }

    pub fn close(&self) -> Result<(), String> {
        let mut slot = self.lock()?;
        slot.generation = slot
            .generation
            .checked_add(1)
            .ok_or_else(|| "Studio remote configuration generation overflowed".to_string())?;
        if let Some(connection) = slot.connection.take() {
            connection.cancel();
        }
        Ok(())
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, RemoteSlot>, String> {
        self.inner
            .lock()
            .map_err(|_| "Studio remote state is poisoned".to_string())
    }
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum StudioRemoteRequest {
    Capabilities,
    Status,
    Frontier,
    Collections,
    RelationalSchemas,
    Graph {
        limit: usize,
        seed: Option<String>,
        direction: Option<String>,
    },
    Entity {
        entity_id: String,
    },
    ReadOnlyHql {
        query: String,
    },
    NamedQuery {
        request: NamedQueryRequest,
    },
}

struct PreparedRequest {
    method: Method,
    url: Url,
    body: Option<Value>,
}

#[derive(Serialize)]
struct ReadOnlyHqlBody<'a> {
    query: &'a str,
}

impl RemoteConnection {
    pub fn new(base_url: &str, token: &str) -> Result<Self, String> {
        let base_url = validate_remote_base_url(base_url)?;
        let authorization = if token.is_empty() {
            None
        } else {
            if token.len() > MAX_REMOTE_TOKEN_BYTES {
                return Err("The remote bearer token exceeds the Studio limit".to_string());
            }
            if !valid_bearer_token(token) {
                return Err("The remote bearer token contains invalid characters".to_string());
            }
            let mut value = HeaderValue::from_str(&format!("Bearer {token}")).map_err(|_| {
                "The remote bearer token contains invalid header characters".to_string()
            })?;
            value.set_sensitive(true);
            Some(value)
        };
        let builder = Client::builder()
            .redirect(Policy::none())
            .timeout(Duration::from_secs(15));
        let client = apply_proxy_policy(builder, &base_url)
            .build()
            .map_err(|error| {
                format!(
                    "Could not create the remote Studio client: {}",
                    error.without_url()
                )
            })?;

        Ok(Self {
            base_url,
            authorization: Arc::new(Mutex::new(authorization)),
            client,
            cancellation: CancellationToken::new(),
        })
    }

    pub fn cancel(&self) {
        self.cancellation.cancel();
        if let Ok(mut authorization) = self.authorization.lock() {
            *authorization = None;
        }
    }

    async fn send_request(
        &self,
        request: StudioRemoteRequest,
    ) -> Result<reqwest::Response, String> {
        if self.cancellation.is_cancelled() {
            return Err(remote_connection_closed_error());
        }
        let prepared = prepare_request(&self.base_url, request)?;
        if prepared.url.origin().ascii_serialization()
            != self.base_url.origin().ascii_serialization()
        {
            return Err("The remote Studio request escaped its configured origin".to_string());
        }

        let mut builder = self.client.request(prepared.method, prepared.url);
        builder = builder.header("accept", "application/json");
        let authorization = self
            .authorization
            .lock()
            .map_err(|_| "Studio remote authorization state is poisoned".to_string())?
            .clone();
        if let Some(authorization) = authorization {
            builder = builder.header(AUTHORIZATION, authorization);
        }
        if let Some(body) = prepared.body {
            builder = builder.json(&body);
        }

        let response = tokio::select! {
            biased;
            _ = self.cancellation.cancelled() => return Err(remote_connection_closed_error()),
            response = builder.send() => response.map_err(|error| {
                format!("Remote Studio request failed: {}", error.without_url())
            })?,
        };
        if self.cancellation.is_cancelled() {
            return Err(remote_connection_closed_error());
        }
        Ok(response)
    }

    pub async fn request(&self, request: StudioRemoteRequest) -> Result<Value, String> {
        let response = self.send_request(request).await?;
        let status = response.status();
        if status.is_redirection() {
            if self.cancellation.is_cancelled() {
                return Err(remote_connection_closed_error());
            }
            return Err(format!(
                "Genesis server redirected the request with {status}; redirects are disabled"
            ));
        }
        if !status.is_success() {
            let (body, truncated) = read_bounded_response_body(
                response,
                MAX_REMOTE_ERROR_RESPONSE_BYTES,
                &self.cancellation,
                || {},
            )
            .await?;
            if self.cancellation.is_cancelled() {
                return Err(remote_connection_closed_error());
            }
            let detail = String::from_utf8_lossy(&body);
            let detail = if truncated {
                if detail.trim().is_empty() {
                    format!(
                        "error body omitted after reaching the {} byte limit",
                        MAX_REMOTE_ERROR_RESPONSE_BYTES
                    )
                } else {
                    format!("{} [truncated]", detail.trim())
                }
            } else {
                detail.trim().to_string()
            };
            let error = format!(
                "Genesis server {status}: {}",
                if detail.is_empty() {
                    status.canonical_reason().unwrap_or("request failed")
                } else {
                    detail.as_str()
                }
            );
            if self.cancellation.is_cancelled() {
                return Err(remote_connection_closed_error());
            }
            return Err(error);
        }
        let (body, truncated) = read_bounded_response_body(
            response,
            MAX_REMOTE_JSON_RESPONSE_BYTES,
            &self.cancellation,
            || {},
        )
        .await?;
        if self.cancellation.is_cancelled() {
            return Err(remote_connection_closed_error());
        }
        if truncated {
            if self.cancellation.is_cancelled() {
                return Err(remote_connection_closed_error());
            }
            return Err(format!(
                "Genesis server response exceeded the Studio limit of {} bytes",
                MAX_REMOTE_JSON_RESPONSE_BYTES
            ));
        }
        let result = serde_json::from_slice::<Value>(&body)
            .map_err(|error| format!("Genesis server returned invalid JSON: {error}"));
        if self.cancellation.is_cancelled() {
            return Err(remote_connection_closed_error());
        }
        result
    }
}

fn apply_proxy_policy(builder: reqwest::ClientBuilder, base_url: &Url) -> reqwest::ClientBuilder {
    if base_url.scheme() == "http" {
        // Plain HTTP is only accepted for loopback. Keep its bearer token off
        // environment-configured proxies, which would otherwise see it in cleartext.
        builder.no_proxy()
    } else {
        builder
    }
}

async fn read_bounded_response_body(
    mut response: reqwest::Response,
    max_bytes: usize,
    cancellation: &CancellationToken,
    mut after_chunk: impl FnMut(),
) -> Result<(Vec<u8>, bool), String> {
    let declared_length = response.content_length();
    if declared_length.is_some_and(|length| length > max_bytes as u64) {
        return Ok((Vec::new(), true));
    }
    let capacity = declared_length.unwrap_or_default().min(max_bytes as u64) as usize;
    let mut body = Vec::with_capacity(capacity);
    loop {
        let chunk = tokio::select! {
            biased;
            _ = cancellation.cancelled() => return Err(remote_connection_closed_error()),
            chunk = response.chunk() => chunk.map_err(|error| {
                format!(
                    "Could not read Genesis server response: {}",
                    error.without_url()
                )
            })?,
        };
        let Some(chunk) = chunk else {
            return Ok((body, false));
        };
        let remaining = max_bytes.saturating_sub(body.len());
        if chunk.len() > remaining {
            body.extend_from_slice(&chunk[..remaining]);
            after_chunk();
            return Ok((body, true));
        }
        body.extend_from_slice(&chunk);
        after_chunk();
        if body.len() == max_bytes {
            // For chunked responses, treat an exact-limit body as truncated
            // rather than fetching and allocating a possibly unbounded chunk.
            let truncated = declared_length.map_or(true, |length| length > max_bytes as u64);
            return Ok((body, truncated));
        }
    }
}

fn remote_connection_closed_error() -> String {
    "Remote GenesisBlockDB connection was closed or replaced".to_string()
}

fn validate_remote_base_url(input: &str) -> Result<Url, String> {
    let input = input.trim();
    if input.is_empty() || input.contains('\\') || input.chars().any(char::is_control) {
        return Err("Enter a valid remote server URL".to_string());
    }
    let (_scheme, remainder) = input
        .split_once("://")
        .ok_or_else(|| "Remote server URL must include an http or https scheme".to_string())?;
    let authority_end = remainder.find(['/', '?', '#']).unwrap_or(remainder.len());
    let authority = &remainder[..authority_end];
    if authority.is_empty() || authority.contains('@') {
        return Err(
            "Remote server URL must include a host and must not contain credentials".to_string(),
        );
    }
    validate_explicit_port(authority)?;
    validate_raw_path(&remainder[authority_end..])?;

    let url = Url::parse(input).map_err(|_| "Enter a valid remote server URL".to_string())?;
    if url.host_str().map_or(true, str::is_empty) {
        return Err("Remote server URL must include a host".to_string());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("Remote server URL must not include a query or fragment".to_string());
    }
    if url.port() == Some(0) {
        return Err("Remote server URL port must be between 1 and 65535".to_string());
    }
    match url.scheme() {
        "https" => Ok(url),
        "http" if is_loopback_host(&url) => Ok(url),
        "http" => Err(
            "Remote HTTP is allowed only for a loopback server; use HTTPS otherwise".to_string(),
        ),
        _ => Err("Remote server URL must use HTTPS, or HTTP on loopback".to_string()),
    }
}

fn validate_explicit_port(authority: &str) -> Result<(), String> {
    let port = if authority.starts_with('[') {
        let close = authority
            .find(']')
            .ok_or_else(|| "Remote server URL has an invalid IPv6 host".to_string())?;
        match &authority[close + 1..] {
            "" => None,
            suffix if suffix.starts_with(':') => Some(&suffix[1..]),
            _ => return Err("Remote server URL has an invalid authority".to_string()),
        }
    } else if authority.contains(':') {
        Some(
            authority
                .rsplit_once(':')
                .map(|(_, port)| port)
                .unwrap_or_default(),
        )
    } else {
        None
    };

    if let Some(port) = port {
        if port.is_empty() || !port.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err("Remote server URL port must be a number between 1 and 65535".to_string());
        }
        let port = port
            .parse::<u16>()
            .map_err(|_| "Remote server URL port must be between 1 and 65535".to_string())?;
        if port == 0 {
            return Err("Remote server URL port must be between 1 and 65535".to_string());
        }
    }
    Ok(())
}

fn validate_raw_path(remainder: &str) -> Result<(), String> {
    let Some(raw_path) = remainder.strip_prefix('/') else {
        return Ok(());
    };
    let raw_path = raw_path.split(['?', '#']).next().unwrap_or_default();
    for segment in raw_path.split('/') {
        if has_unsafe_path_segment(segment) {
            return Err(
                "Remote server URL path must not contain traversal or encoded separators"
                    .to_string(),
            );
        }
    }
    Ok(())
}

fn has_unsafe_path_segment(segment: &str) -> bool {
    let mut current = segment.as_bytes().to_vec();
    loop {
        if std::str::from_utf8(&current).is_err()
            || matches!(current.as_slice(), b"." | b"..")
            || current
                .iter()
                .any(|byte| matches!(byte, b'/' | b'\\' | b'?' | b'#') || byte.is_ascii_control())
        {
            return true;
        }
        let decoded = decode_percent_escapes_once(&current);
        if decoded == current {
            return false;
        }
        current = decoded;
    }
}

fn decode_percent_escapes_once(value: &[u8]) -> Vec<u8> {
    fn hex_value(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }

    let mut decoded = Vec::with_capacity(value.len());
    let mut index = 0;
    while index < value.len() {
        if value[index] == b'%' && index + 2 < value.len() {
            if let (Some(high), Some(low)) =
                (hex_value(value[index + 1]), hex_value(value[index + 2]))
            {
                decoded.push((high << 4) | low);
                index += 3;
                continue;
            }
        }
        decoded.push(value[index]);
        index += 1;
    }
    decoded
}

fn is_loopback_host(url: &Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    let address = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    address
        .parse::<std::net::IpAddr>()
        .is_ok_and(|address| address.is_loopback())
}

fn valid_bearer_token(token: &str) -> bool {
    let unpadded = token.trim_end_matches('=');
    !unpadded.is_empty()
        && unpadded.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'+' | b'/')
        })
}

fn prepare_request(
    base_url: &Url,
    request: StudioRemoteRequest,
) -> Result<PreparedRequest, String> {
    let (method, path, query, body) = match request {
        StudioRemoteRequest::Capabilities => (
            Method::GET,
            route(&["v1", "studio", "capabilities"]),
            vec![],
            None,
        ),
        StudioRemoteRequest::Status => (Method::GET, route(&["v1", "status"]), vec![], None),
        StudioRemoteRequest::Frontier => (Method::GET, route(&["v1", "frontier"]), vec![], None),
        StudioRemoteRequest::Collections => {
            (Method::GET, route(&["v1", "collections"]), vec![], None)
        }
        StudioRemoteRequest::RelationalSchemas => (
            Method::GET,
            route(&["v1", "studio", "relational", "schemas"]),
            vec![],
            None,
        ),
        StudioRemoteRequest::Graph {
            limit,
            seed,
            direction,
        } => {
            if limit == 0 || limit > STUDIO_SCENE_NODE_CEILING {
                return Err(format!(
                    "Graph limit must be between 1 and {STUDIO_SCENE_NODE_CEILING}"
                ));
            }
            if let Some(seed) = &seed {
                if seed.is_empty() || seed.len() > MAX_REMOTE_ENTITY_ID_BYTES {
                    return Err(
                        "Graph seed must be non-empty and within the Studio ID limit".to_string(),
                    );
                }
            }
            if let Some(direction) = &direction {
                if !matches!(direction.as_str(), "out" | "in" | "both") {
                    return Err("Graph direction must be out, in, or both".to_string());
                }
            }
            let mut query = vec![("limit".to_string(), limit.to_string())];
            if let Some(seed) = seed {
                query.push(("seed".to_string(), seed));
            }
            if let Some(direction) = direction {
                query.push(("direction".to_string(), direction));
            }
            (Method::GET, route(&["v1", "studio", "graph"]), query, None)
        }
        StudioRemoteRequest::Entity { entity_id } => {
            if entity_id.is_empty() || entity_id.len() > MAX_REMOTE_ENTITY_ID_BYTES {
                return Err(
                    "Entity ID must be non-empty and within the Studio ID limit".to_string()
                );
            }
            if entity_id.chars().any(char::is_control) || has_unsafe_path_segment(&entity_id) {
                return Err(
                    "Entity ID must not contain path traversal or path separators".to_string(),
                );
            }
            let mut path = route(&["v1", "studio", "entity"]);
            path.push(entity_id);
            (Method::GET, path, vec![], None)
        }
        StudioRemoteRequest::ReadOnlyHql { query } => (
            Method::POST,
            route(&["v1", "studio", "query", "read"]),
            vec![],
            Some(
                serde_json::to_value(ReadOnlyHqlBody { query: &query })
                    .map_err(|error| error.to_string())?,
            ),
        ),
        StudioRemoteRequest::NamedQuery { request } => (
            Method::POST,
            route(&["v1", "relational", "query"]),
            vec![],
            Some(serde_json::to_value(request).map_err(|error| error.to_string())?),
        ),
    };

    let mut url = base_url.clone();
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| "Remote server URL cannot be used as a hierarchical base".to_string())?;
        segments.pop_if_empty();
        segments.extend(path.iter().map(String::as_str));
    }
    if !query.is_empty() {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in query {
            pairs.append_pair(&key, &value);
        }
    }
    Ok(PreparedRequest { method, url, body })
}

fn route(segments: &[&str]) -> Vec<String> {
    segments
        .iter()
        .map(|segment| (*segment).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        apply_proxy_policy, prepare_request, read_bounded_response_body, validate_remote_base_url,
        CancellationToken, RemoteConnection, RemoteState, StudioRemoteRequest,
        MAX_REMOTE_ENTITY_ID_BYTES,
    };
    use reqwest::{Client, Method, Proxy, Url};
    use serde_json::{json, Value};
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::Arc,
        thread,
        time::Duration,
    };

    #[test]
    fn accepts_https_and_loopback_http_but_rejects_unsafe_url_components() {
        for url in [
            "https://db.example",
            "https://db.example:443/genesis",
            "http://127.0.0.1:3000",
            "http://[::1]:3000",
            "http://localhost:3000",
        ] {
            assert!(
                validate_remote_base_url(url).is_ok(),
                "expected {url} to be accepted"
            );
        }

        for url in [
            "",
            "ftp://db.example",
            "file:///tmp/db",
            "http://db.example",
            "http://192.168.1.20:3000",
            "http://localhost.evil.example:3000",
            "https://user:secret@db.example",
            "https://@db.example",
            "https://db.example?token=secret",
            "https://db.example/#fragment",
            "https://db.example:0",
            "https://db.example:",
            "https://db.example:65536",
            "https://db.example/../other",
            "https://db.example/%2e%2e/other",
            "https://db.example/%252e%252e/admin",
            "https://db.example/a%252fb",
            "https://db.example/a%253fb",
            "https://db.example/a%2fb",
            "https://db.example/a%5cb",
            "https://db.example/%c0%af",
        ] {
            assert!(
                validate_remote_base_url(url).is_err(),
                "expected {url} to be rejected"
            );
        }
        assert!(RemoteConnection::new("https://db.example", "token with spaces").is_err());
        assert!(RemoteConnection::new("https://db.example", "token\r\nInjected: yes").is_err());
    }

    #[test]
    fn allowlisted_routes_keep_the_configured_origin_and_encode_values() {
        let base = validate_remote_base_url("https://db.example:8443/tenant/studio/").unwrap();
        let graph = prepare_request(
            &base,
            StudioRemoteRequest::Graph {
                limit: 250,
                seed: Some("node/雪".to_string()),
                direction: Some("both".to_string()),
            },
        )
        .unwrap();
        assert_eq!(graph.method, Method::GET);
        assert_eq!(
            graph.url.origin().ascii_serialization(),
            base.origin().ascii_serialization()
        );
        assert_eq!(graph.url.path(), "/tenant/studio/v1/studio/graph");
        assert_eq!(
            graph.url.query_pairs().collect::<Vec<_>>(),
            vec![
                ("limit".into(), "250".into()),
                ("seed".into(), "node/雪".into()),
                ("direction".into(), "both".into()),
            ]
        );

        let entity = prepare_request(
            &base,
            StudioRemoteRequest::Entity {
                entity_id: "node:雪".to_string(),
            },
        )
        .unwrap();
        assert_eq!(entity.method, Method::GET);
        assert_eq!(
            entity.url.origin().ascii_serialization(),
            base.origin().ascii_serialization()
        );
        assert!(entity.url.path().ends_with("/node:%E9%9B%AA"));
    }

    #[test]
    fn routes_reject_invalid_limits_directions_and_ids() {
        let base = Url::parse("https://db.example").unwrap();
        for request in [
            StudioRemoteRequest::Graph {
                limit: 0,
                seed: None,
                direction: None,
            },
            StudioRemoteRequest::Graph {
                limit: 1001,
                seed: None,
                direction: None,
            },
            StudioRemoteRequest::Graph {
                limit: 1,
                seed: None,
                direction: Some("sideways".to_string()),
            },
            StudioRemoteRequest::Graph {
                limit: 1,
                seed: Some(String::new()),
                direction: None,
            },
            StudioRemoteRequest::Entity {
                entity_id: String::new(),
            },
            StudioRemoteRequest::Entity {
                entity_id: ".".to_string(),
            },
            StudioRemoteRequest::Entity {
                entity_id: "..".to_string(),
            },
            StudioRemoteRequest::Entity {
                entity_id: "%2e%2e".to_string(),
            },
            StudioRemoteRequest::Entity {
                entity_id: "%252e%252e".to_string(),
            },
            StudioRemoteRequest::Entity {
                entity_id: "node/other".to_string(),
            },
            StudioRemoteRequest::Entity {
                entity_id: "node%252fother".to_string(),
            },
            StudioRemoteRequest::Entity {
                entity_id: "node%253fother".to_string(),
            },
            StudioRemoteRequest::Entity {
                entity_id: "node%2523other".to_string(),
            },
            StudioRemoteRequest::Entity {
                entity_id: "x".repeat(MAX_REMOTE_ENTITY_ID_BYTES + 1),
            },
        ] {
            assert!(prepare_request(&base, request).is_err());
        }

        assert!(serde_json::from_value::<StudioRemoteRequest>(json!({
            "kind": "graph", "limit": 5, "path": "/v1/status", "method": "POST"
        }))
        .is_err());
        assert!(serde_json::from_value::<StudioRemoteRequest>(json!({
            "kind": "arbitrary", "url": "https://elsewhere.example"
        }))
        .is_err());
        assert!(serde_json::from_value::<StudioRemoteRequest>(json!({
            "kind": "graph", "limit": 5, "query": {"unexpected": "value"}
        }))
        .is_err());
    }

    #[test]
    fn read_only_post_operations_use_only_their_fixed_routes_and_bodies() {
        let base = Url::parse("https://db.example").unwrap();
        let hql = prepare_request(
            &base,
            StudioRemoteRequest::ReadOnlyHql {
                query: "SEARCH nodes LIMIT 1".to_string(),
            },
        )
        .unwrap();
        assert_eq!(hql.method, Method::POST);
        assert_eq!(hql.url.path(), "/v1/studio/query/read");
        assert_eq!(hql.body, Some(json!({"query": "SEARCH nodes LIMIT 1"})));

        let named_query = prepare_request(
            &base,
            StudioRemoteRequest::NamedQuery {
                request: genesis_block_native::NamedQueryRequest {
                    namespace: "notes".to_string(),
                    schema_version: 3,
                    query_name: "note_by_id".to_string(),
                    parameters: json!({"note_id": "n1"}),
                    limit: Some(1),
                },
            },
        )
        .unwrap();
        assert_eq!(named_query.method, Method::POST);
        assert_eq!(named_query.url.path(), "/v1/relational/query");
        assert_eq!(
            named_query.body,
            Some(json!({
                "namespace": "notes",
                "schema_version": 3,
                "query_name": "note_by_id",
                "parameters": {"note_id": "n1"},
                "limit": 1,
            }))
        );

        let capabilities = prepare_request(&base, StudioRemoteRequest::Capabilities).unwrap();
        assert_eq!(capabilities.method, Method::GET);
        assert_eq!(capabilities.url.path(), "/v1/studio/capabilities");
        assert!(capabilities.body.is_none());
    }

    #[test]
    fn forwards_token_only_to_configured_origin_and_does_not_follow_redirects() {
        let redirect_target = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        redirect_target.set_nonblocking(true).unwrap();
        let redirect_target_url = format!(
            "http://{}/exfiltrate",
            redirect_target.local_addr().unwrap()
        );
        let (configured_url, configured_request, configured_server) = one_shot_server(
            move |stream| {
                let response = format!(
                "HTTP/1.1 302 Found\r\nLocation: {redirect_target_url}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
                stream.write_all(response.as_bytes()).unwrap();
            },
        );

        let connection = RemoteConnection::new(&configured_url, "studio-secret").unwrap();
        let response = tauri::async_runtime::block_on(async {
            connection.request(StudioRemoteRequest::Status).await
        });
        assert!(response.unwrap_err().contains("redirects are disabled"));
        let request = configured_request.join().unwrap();
        configured_server.join().unwrap();
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer studio-secret"));

        match redirect_target.accept() {
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Ok(_) => panic!("redirect target received a request"),
            Err(error) => panic!("could not check redirect target: {error}"),
        }
    }

    #[test]
    fn response_body_reader_stops_at_its_configured_limit() {
        let (url, request, server) = one_shot_server(|stream| {
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\nA\r\n0123456789\r\n0\r\n\r\n",
                )
                .unwrap();
        });
        let result = tauri::async_runtime::block_on(async {
            let response = reqwest::Client::new().get(url).send().await.unwrap();
            read_bounded_response_body(response, 8, &CancellationToken::new(), || {}).await
        })
        .unwrap();
        request.join().unwrap();
        server.join().unwrap();

        assert_eq!(result.0, b"01234567");
        assert!(result.1);
    }

    #[test]
    fn close_and_reconfigure_cancel_cloned_queued_requests_before_server_send() {
        for reconfigure in [false, true] {
            let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
            listener.set_nonblocking(true).unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap());
            let state = RemoteState::default();
            let generation = state.begin_reconfigure().unwrap();
            state
                .install(
                    generation,
                    RemoteConnection::new(&url, "queued-secret").unwrap(),
                )
                .unwrap();

            // This clone models a request command that captured state and is
            // queued while the user disconnects or replaces the connection.
            let queued_request = state.connection().unwrap();
            if reconfigure {
                state.begin_reconfigure().unwrap();
            } else {
                state.close().unwrap();
            }
            let result = tauri::async_runtime::block_on(async {
                queued_request.request(StudioRemoteRequest::Status).await
            });

            assert!(result
                .unwrap_err()
                .contains("connection was closed or replaced"));
            match listener.accept() {
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Ok(_) => panic!("queued request reached the server after invalidation"),
                Err(error) => panic!("could not check queued request listener: {error}"),
            }
        }
    }

    #[test]
    fn close_cancels_an_inflight_response_body_read() {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let (first_chunk_sender, first_chunk_receiver) = std::sync::mpsc::channel();
        let (release_sender, release_receiver) = std::sync::mpsc::channel();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            loop {
                let bytes = stream.read(&mut buffer).unwrap_or(0);
                if bytes == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..bytes]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: keep-alive\r\n\r\nx",
                )
                .unwrap();
            stream.flush().unwrap();
            let _ = release_receiver.recv_timeout(Duration::from_secs(3));
        });

        let state = Arc::new(RemoteState::default());
        let generation = state.begin_reconfigure().unwrap();
        state
            .install(
                generation,
                RemoteConnection::new(&format!("http://{address}"), "inflight-secret").unwrap(),
            )
            .unwrap();
        let request_state = Arc::clone(&state);
        let (result_sender, result_receiver) = std::sync::mpsc::channel();
        let request = thread::spawn(move || {
            let connection = request_state.connection().unwrap();
            let result = tauri::async_runtime::block_on(async {
                let response = connection.send_request(StudioRemoteRequest::Status).await?;
                read_bounded_response_body(response, 100, &connection.cancellation, || {
                    let _ = first_chunk_sender.send(());
                })
                .await
            });
            result_sender.send(result).unwrap();
        });

        first_chunk_receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("the response reader did not consume the first body chunk");
        state.close().unwrap();
        let result = result_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("disconnect did not cancel the body read waiting for more data");
        release_sender.send(()).unwrap();
        request.join().unwrap();
        server.join().unwrap();

        assert!(result
            .unwrap_err()
            .contains("connection was closed or replaced"));
    }

    #[test]
    fn close_cancels_a_request_while_waiting_for_response_headers() {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let (started_sender, started_receiver) = std::sync::mpsc::channel();
        let (release_sender, release_receiver) = std::sync::mpsc::channel();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            loop {
                let bytes = stream.read(&mut buffer).unwrap_or(0);
                if bytes == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..bytes]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            started_sender.send(()).unwrap();
            // Deliberately withhold response headers until the caller cancels.
            let _ = release_receiver.recv_timeout(Duration::from_secs(3));
        });

        let state = Arc::new(RemoteState::default());
        let generation = state.begin_reconfigure().unwrap();
        state
            .install(
                generation,
                RemoteConnection::new(&format!("http://{address}"), "headers-secret").unwrap(),
            )
            .unwrap();
        let request_state = Arc::clone(&state);
        let (result_sender, result_receiver) = std::sync::mpsc::channel();
        let request = thread::spawn(move || {
            let connection = request_state.connection().unwrap();
            let result = tauri::async_runtime::block_on(async {
                connection.request(StudioRemoteRequest::Status).await
            });
            result_sender.send(result).unwrap();
        });

        started_receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("the test server did not accept the request");
        state.close().unwrap();
        let result = result_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("disconnect did not cancel the request waiting for headers");
        release_sender.send(()).unwrap();
        request.join().unwrap();
        server.join().unwrap();

        assert!(result
            .unwrap_err()
            .contains("connection was closed or replaced"));
    }

    #[test]
    fn stale_generation_install_cancels_the_unpublished_connection() {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let state = RemoteState::default();
        let stale_generation = state.begin_reconfigure().unwrap();
        state.begin_reconfigure().unwrap();
        let stale_connection = RemoteConnection::new(&url, "stale-secret").unwrap();
        let stale_clone = stale_connection.clone();

        assert!(state.install(stale_generation, stale_connection).is_err());
        let result = tauri::async_runtime::block_on(async {
            stale_clone.request(StudioRemoteRequest::Status).await
        });

        assert!(result
            .unwrap_err()
            .contains("connection was closed or replaced"));
        match listener.accept() {
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Ok(_) => panic!("stale unpublished connection reached its server"),
            Err(error) => panic!("could not check stale connection listener: {error}"),
        }
    }

    #[test]
    fn loopback_http_bypasses_proxies_while_https_keeps_proxy_support() {
        let (target_url, target_request, target_server) = one_shot_server(|stream| {
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
                .unwrap();
        });
        let proxy_listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        proxy_listener.set_nonblocking(true).unwrap();
        let proxy_url = format!("http://{}", proxy_listener.local_addr().unwrap());
        let loopback = Url::parse(&target_url).unwrap();
        let client = apply_proxy_policy(
            Client::builder().proxy(Proxy::all(&proxy_url).unwrap()),
            &loopback,
        )
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
        let response = tauri::async_runtime::block_on(async {
            client
                .get(&target_url)
                .header("authorization", "Bearer loopback-test-token")
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        });
        assert_eq!(response, "ok");
        let request = target_request.join().unwrap();
        target_server.join().unwrap();
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer loopback-test-token"));
        match proxy_listener.accept() {
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Ok(_) => panic!("loopback HTTP request was sent through a proxy"),
            Err(error) => panic!("could not check loopback proxy listener: {error}"),
        }

        let https_proxy = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        https_proxy.set_nonblocking(true).unwrap();
        let https_proxy_url = format!("http://{}", https_proxy.local_addr().unwrap());
        let remote = Url::parse("https://example.com").unwrap();
        let https_client = apply_proxy_policy(
            Client::builder().proxy(Proxy::all(&https_proxy_url).unwrap()),
            &remote,
        )
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
        let proxy_request = thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(2);
            let (mut stream, _) = loop {
                match https_proxy.accept() {
                    Ok(connection) => break connection,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && std::time::Instant::now() < deadline =>
                    {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("HTTPS proxy did not receive the request: {error}"),
                }
            };
            // Some Windows socket implementations inherit the listener's
            // nonblocking mode on accepted sockets. Restore blocking mode so
            // `read` waits for the CONNECT request instead of treating
            // WouldBlock as an empty request below.
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            loop {
                let bytes = stream.read(&mut buffer).unwrap_or(0);
                if bytes == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..bytes]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            String::from_utf8_lossy(&request).into_owned()
        });
        let result =
            tauri::async_runtime::block_on(async { https_client.get(remote).send().await });
        assert!(result.is_err(), "the test proxy does not complete TLS");
        let proxy_request = proxy_request.join().unwrap();
        assert!(
            proxy_request.starts_with("CONNECT example.com:443 HTTP/1.1"),
            "unexpected HTTPS proxy request: {proxy_request:?}"
        );
    }

    #[test]
    fn production_webview_csp_keeps_remote_network_outside_the_webview() {
        let config: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let csp = config["app"]["security"]["csp"].as_str().unwrap();
        let connect_src = csp
            .split(';')
            .map(str::trim)
            .find(|directive| directive.starts_with("connect-src "))
            .unwrap();
        assert_eq!(connect_src, "connect-src 'self' ipc: http://ipc.localhost");
    }

    fn one_shot_server(
        respond: impl FnOnce(&mut TcpStream) + Send + 'static,
    ) -> (String, thread::JoinHandle<String>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            loop {
                let bytes = stream.read(&mut buffer).unwrap_or(0);
                if bytes == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..bytes]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            sender
                .send(String::from_utf8_lossy(&request).into_owned())
                .unwrap();
            respond(&mut stream);
        });
        (
            format!("http://{address}"),
            thread::spawn(move || receiver.recv_timeout(Duration::from_secs(3)).unwrap()),
            server,
        )
    }
}
