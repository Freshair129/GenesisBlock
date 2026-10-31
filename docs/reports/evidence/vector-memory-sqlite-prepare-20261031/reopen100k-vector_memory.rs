//! Opt-in lifecycle memory mode for vbench-genesis. Run each case in a fresh process.
use genesis_block_native::{HybridSearchInput, NodeInput, OpenOptions, Storage};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn number(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .map(|v| v.parse().expect(key))
        .unwrap_or(default)
}

fn rss() -> u64 {
    let pid = sysinfo::get_current_pid().unwrap();
    let mut system = sysinfo::System::new();
    system.refresh_process(pid);
    system.process(pid).expect("current process").memory()
}

/// Sampled peak, not allocator accounting or the OS lifetime high-water mark.
fn measure<T>(f: impl FnOnce() -> T) -> (T, Value) {
    let before = rss();
    let stop = Arc::new(AtomicBool::new(false));
    let signal = stop.clone();
    let sampler = std::thread::spawn(move || {
        let limit = std::env::var("GB_MEMORY_RSS_LIMIT_BYTES")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(4u64 << 30);
        let mut peak = before;
        while !signal.load(Ordering::Relaxed) {
            let current = rss();
            if current > limit {
                eprintln!("memory audit resource limit: RSS {current} > {limit}; scratch artifacts retained");
                std::process::exit(70);
            }
            peak = peak.max(current);
            std::thread::sleep(Duration::from_millis(20));
        }
        peak.max(rss())
    });
    let start = Instant::now();
    let result = f();
    let seconds = start.elapsed().as_secs_f64();
    let after = rss();
    stop.store(true, Ordering::Relaxed);
    let peak = sampler.join().unwrap().max(after);
    (
        result,
        json!({"before_rss_bytes":before, "process_rss_bytes":after,
        "peak_rss_bytes":peak,"seconds":seconds}),
    )
}

fn row(reader: &mut impl Read, dim: usize) -> Vec<f32> {
    let mut bytes = vec![0; dim * 4];
    reader.read_exact(&mut bytes).unwrap();
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

fn write_row(writer: &mut impl Write, row: &[f32]) {
    for value in row {
        writer.write_all(&value.to_le_bytes()).unwrap();
    }
}

fn normalize(row: &mut [f32]) {
    let norm = row.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-9);
    for x in row {
        *x /= norm;
    }
}

/// Preparation is a separate invocation so corpus/truth allocations cannot inflate DB RSS.
fn prepare(path: &Path, n: usize, dim: usize, q: usize) {
    fs::create_dir_all(path).unwrap();
    assert!(
        !path.join("corpus.f32").exists(),
        "use a fresh preparation directory"
    );
    let mut rng = StdRng::seed_from_u64(42);
    let model = if let Ok(source) = std::env::var("GB_MEMORY_CORPUS") {
        assert_eq!(fs::metadata(&source).unwrap().len(), (n * dim * 4) as u64);
        // Dot-product truth below requires unit rows, just like the engine's
        // cosine space. Reject arbitrary external inputs rather than mis-score recall.
        let mut reader = BufReader::new(File::open(&source).unwrap());
        for id in 0..n {
            let values = row(&mut reader, dim);
            let norm2 = values.iter().map(|&v| f64::from(v).powi(2)).sum::<f64>();
            assert!(
                norm2.is_finite() && (norm2 - 1.0).abs() < 1e-4,
                "external corpus row {id} must be unit normalized for cosine ground truth"
            );
        }
        fs::copy(source, path.join("corpus.f32")).unwrap();
        "bge-m3-repository-fixture"
    } else {
        let mut out = BufWriter::new(File::create(path.join("corpus.f32")).unwrap());
        for _ in 0..n {
            let mut v: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
            normalize(&mut v);
            write_row(&mut out, &v);
        }
        out.flush().unwrap();
        "synthetic-unit-seed42"
    };
    let mut corpus = File::open(path.join("corpus.f32")).unwrap();
    let mut queries = Vec::new();
    for _ in 0..q {
        let id = rng.gen_range(0..n);
        corpus.seek(SeekFrom::Start((id * dim * 4) as u64)).unwrap();
        let mut v = row(&mut corpus, dim);
        for x in &mut v {
            *x += rng.gen_range(-0.02..0.02);
        }
        normalize(&mut v);
        queries.push(v);
    }
    let mut out = BufWriter::new(File::create(path.join("queries.f32")).unwrap());
    for v in &queries {
        write_row(&mut out, v);
    }
    out.flush().unwrap();
    let mut truth = Vec::new();
    // One O(N) score list in this preparation process only; corpus remains streamed.
    for query in &queries {
        let mut reader = BufReader::new(File::open(path.join("corpus.f32")).unwrap());
        let mut scores = Vec::with_capacity(n);
        for id in 0..n {
            let v = row(&mut reader, dim);
            let dot: f32 = v.iter().zip(query).map(|(a, b)| a * b).sum();
            scores.push((dot, id));
        }
        scores.sort_unstable_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        truth.push(scores.iter().take(100).map(|x| x.1).collect::<Vec<_>>());
    }
    fs::write(path.join("truth.json"), serde_json::to_vec(&truth).unwrap()).unwrap();
    fs::write(
        path.join("meta.json"),
        serde_json::to_vec_pretty(&json!({"n":n,
        "dim":dim,"q":q,"model":model,"metric":"cosine"}))
        .unwrap(),
    )
    .unwrap();
}

fn disk_bytes(path: &Path) -> u64 {
    fs::read_dir(path)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                disk_bytes(&e.path())
            } else {
                e.metadata().unwrap().len()
            }
        })
        .sum()
}

fn search(
    storage: &Storage,
    queries: &[Vec<f32>],
    truth: &[Vec<usize>],
    k: usize,
    ef: u32,
) -> Value {
    let mut latency = Vec::new();
    let mut result_counts = Vec::new();
    let mut hits = 0;
    for (query, expected) in queries.iter().zip(truth) {
        let input = HybridSearchInput {
            query_vector: query.iter().map(|&x| x as f64).collect(),
            k: k as u32,
            alpha: Some(0.0),
            lang: None,
            as_of: None,
            collection: Some("bench".into()),
            ef_search: Some(ef),
            oversample: None,
        };
        let start = Instant::now();
        let result = match storage.hybrid_search(input) {
            Ok(result) => result,
            Err(error) => {
                // Preserve the measured lifecycle phases and the actual query
                // contract failure; never widen the engine budget for a benchmark.
                return json!({"k":k,"error":error.to_string(),
                    "completed_queries":latency.len(),"requested_queries":queries.len(),
                    "result_count_min":result_counts.iter().copied().min(),
                    "result_count_max":result_counts.iter().copied().max(),
                    "recall":null,"p50_us":null,"p95_us":null,"p99_us":null});
            }
        };
        latency.push(start.elapsed().as_secs_f64() * 1e6);
        result_counts.push(result.len());
        let got: std::collections::HashSet<usize> =
            result.iter().map(|x| x.node.id.parse().unwrap()).collect();
        hits += expected
            .iter()
            .take(k)
            .filter(|id| got.contains(id))
            .count();
    }
    latency.sort_by(f64::total_cmp);
    let percentile = |p: f64| latency[((latency.len() - 1) as f64 * p).round() as usize];
    json!({"k":k,"recall":hits as f64/(queries.len()*k) as f64,
        "result_count_min":result_counts.iter().copied().min(),"result_count_max":result_counts.iter().copied().max(),
        "p50_us":percentile(0.5),"p95_us":percentile(0.95),"p99_us":percentile(0.99)})
}

#[cfg(windows)]
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct WinFileTime {
    low: u32,
    high: u32,
}

#[cfg(windows)]
impl WinFileTime {
    fn as_u64(self) -> u64 {
        ((self.high as u64) << 32) | self.low as u64
    }
}

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct WinProcessMemoryCounters {
    cb: u32,
    page_fault_count: u32,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentProcess() -> *mut std::ffi::c_void;
    fn GetSystemTimes(
        idle_time: *mut WinFileTime,
        kernel_time: *mut WinFileTime,
        user_time: *mut WinFileTime,
    ) -> i32;
    fn GetProcessTimes(
        process: *mut std::ffi::c_void,
        creation_time: *mut WinFileTime,
        exit_time: *mut WinFileTime,
        kernel_time: *mut WinFileTime,
        user_time: *mut WinFileTime,
    ) -> i32;
}

#[cfg(windows)]
#[link(name = "Psapi")]
extern "system" {
    fn GetProcessMemoryInfo(
        process: *mut std::ffi::c_void,
        counters: *mut WinProcessMemoryCounters,
        size: u32,
    ) -> i32;
}

#[cfg(windows)]
fn query_resource_sample() -> Value {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    let mut idle = WinFileTime::default();
    let mut kernel = WinFileTime::default();
    let mut user = WinFileTime::default();
    let mut process_kernel = WinFileTime::default();
    let mut process_user = WinFileTime::default();
    let mut creation = WinFileTime::default();
    let mut exit = WinFileTime::default();
    let process = unsafe { GetCurrentProcess() };
    let system_times_ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } != 0;
    let process_times_ok = unsafe {
        GetProcessTimes(
            process,
            &mut creation,
            &mut exit,
            &mut process_kernel,
            &mut process_user,
        )
    } != 0;
    let mut counters = WinProcessMemoryCounters {
        cb: std::mem::size_of::<WinProcessMemoryCounters>() as u32,
        ..WinProcessMemoryCounters::default()
    };
    let process_memory_ok = unsafe {
        GetProcessMemoryInfo(process, &mut counters, counters.cb)
    } != 0;
    let timestamp_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    json!({
        "timestamp_unix_ms":timestamp_unix_ms,
        "total_memory_bytes":system.total_memory(),
        "available_memory_bytes":system.available_memory(),
        "process_working_set_bytes":process_memory_ok.then_some(counters.working_set_size),
        "process_page_fault_count":process_memory_ok.then_some(counters.page_fault_count),
        "process_user_time_100ns":process_times_ok.then_some(process_user.as_u64()),
        "process_kernel_time_100ns":process_times_ok.then_some(process_kernel.as_u64()),
        "host_idle_time_100ns":system_times_ok.then_some(idle.as_u64()),
        "host_kernel_time_100ns":system_times_ok.then_some(kernel.as_u64()),
        "host_user_time_100ns":system_times_ok.then_some(user.as_u64()),
    })
}

#[cfg(not(windows))]
fn query_resource_sample() -> Value {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    json!({
        "timestamp_unix_ms":SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis(),
        "total_memory_bytes":system.total_memory(),
        "available_memory_bytes":system.available_memory(),
        "process_working_set_bytes":rss(),
        "process_page_fault_count":Value::Null,
        "host_cpu_times":Value::Null,
    })
}

fn host_cpu_busy_percent(start: &Value, end: &Value) -> Option<f64> {
    let start_kernel = start.get("host_kernel_time_100ns")?.as_u64()?;
    let start_user = start.get("host_user_time_100ns")?.as_u64()?;
    let start_idle = start.get("host_idle_time_100ns")?.as_u64()?;
    let end_kernel = end.get("host_kernel_time_100ns")?.as_u64()?;
    let end_user = end.get("host_user_time_100ns")?.as_u64()?;
    let end_idle = end.get("host_idle_time_100ns")?.as_u64()?;
    let total_delta = (end_kernel + end_user).checked_sub(start_kernel + start_user)?;
    let idle_delta = end_idle.checked_sub(start_idle)?;
    if total_delta == 0 {
        return None;
    }
    Some(
        (total_delta.saturating_sub(idle_delta) as f64 / total_delta as f64) * 100.0,
    )
}

fn run_open_repeat_pass(
    path: &str,
    input_path: &str,
    pass: usize,
    repeat: usize,
    sample_count: usize,
) -> Value {
    let stage = |name: &str, extra: Value| {
        eprintln!(
            "GB_MEMORY_OPEN_REPEAT {}",
            json!({"pass":pass,"repeat":repeat,"stage":name,"extra":extra})
        );
    };
    stage("open_start", json!({"path":path}));
    let (open_result, open_memory) = measure(|| {
        Storage::open(OpenOptions {
            path: path.to_string(),
            page_cache_mb: Some(64),
            read_only: Some(false),
            vector_dim: Some(1024),
            retention: Some("full".into()),
        })
    });
    let storage = match open_result {
        Ok(storage) => storage,
        Err(error) => {
            stage("open_error", json!({"error":error.to_string()}));
            return json!({
                "pass":pass,
                "state":"open_error",
                "open_memory":open_memory,
                "error":error.to_string(),
            });
        }
    };
    stage("open_complete", json!({"open_memory":open_memory}));
    // The original same-process reopen path validates coverage immediately
    // before its reopened search. Keep the first pass query-before-drop, then
    // run coverage only on the final reopen so it does not add work between
    // that first query and the measured Drop/reopen transition.
    let (coverage_ok, coverage, coverage_memory) = if pass == repeat {
        let (coverage_result, memory) = measure(|| storage.validate_index_coverage());
        let ok = coverage_result.is_ok();
        let value = match coverage_result {
            Ok(report) => json!({"state":"ok","collections":report}),
            Err(error) => json!({"state":"error","message":error.to_string()}),
        };
        (ok, value, memory)
    } else {
        (
            true,
            json!({"state":"skipped","reason":"reserved for final reopen to match original query path"}),
            Value::Null,
        )
    };
    stage("coverage_complete", json!({"coverage":coverage,"coverage_memory":coverage_memory}));

    let input_meta: Value = serde_json::from_slice(
        &fs::read(Path::new(input_path).join("meta.json")).unwrap(),
    )
    .unwrap();
    let input_n = input_meta["n"].as_u64().unwrap_or(0);
    let dim = input_meta["dim"].as_u64().unwrap_or(0) as usize;
    let query_count = input_meta["q"].as_u64().unwrap_or(0) as usize;
    let mut query = json!({"state":"skipped","reason":"index coverage failed"});
    let mut query_memory = Value::Null;
    let mut query_resources = Value::Null;
    let mut query_samples = json!({"state":"skipped","reason":"first query did not complete"});
    if coverage_ok {
        let query_file = Path::new(input_path).join("queries.f32");
        let mut reader = BufReader::new(File::open(&query_file).unwrap());
        let query_vector = row(&mut reader, dim)
            .into_iter()
            .map(f64::from)
            .collect();
        let resources_start = query_resource_sample();
        stage(
            "query_start",
            json!({"query_index":0,"k":10,"ef_search":200,"alpha":0.0,"resources":resources_start}),
        );
        let (query_result, measured_query) = measure(|| {
            storage.hybrid_search(HybridSearchInput {
                query_vector,
                k: 10,
                alpha: Some(0.0),
                lang: None,
                as_of: None,
                collection: Some("bench".into()),
                ef_search: Some(200),
                oversample: None,
            })
        });
        let resources_end = query_resource_sample();
        query_resources = json!({
            "at_query_start":resources_start,
            "after_query":resources_end,
            "host_cpu_busy_percent_during_query":host_cpu_busy_percent(&resources_start,&resources_end),
            "process_page_fault_delta":resources_start["process_page_fault_count"].as_u64()
                .zip(resources_end["process_page_fault_count"].as_u64())
                .map(|(before,after)|after.saturating_sub(before)),
        });
        query_memory = measured_query;
        query = match query_result {
            Ok(rows) => json!({
                "state":"ok",
                "input":input_path,
                "input_n":input_n,
                "input_dim":dim,
                "query_index":0,
                "requested_queries":1,
                "completed_queries":1,
                "result_count":rows.len(),
                "first_hits":rows.iter().take(10).map(|row| json!({
                    "id":row.node.id,
                    "score":row.score,
                })).collect::<Vec<_>>(),
            }),
            Err(error) => json!({
                "state":"error",
                "input":input_path,
                "input_n":input_n,
                "input_dim":dim,
                "query_index":0,
                "requested_queries":1,
                "completed_queries":0,
                "error":error.to_string(),
            }),
        };
        stage(
            "query_complete",
            json!({"query":query,"query_memory":query_memory,"query_resources":query_resources}),
        );
        let pass_sample_count = if pass == repeat { sample_count } else { 0 };
        if query["state"].as_str() == Some("ok") && pass_sample_count > 0 {
            assert!(query_count > 0, "input metadata must contain queries");
            let mut query_reader = BufReader::new(
                File::open(Path::new(input_path).join("queries.f32")).unwrap(),
            );
            let all_queries: Vec<Vec<f32>> =
                (0..query_count).map(|_| row(&mut query_reader, dim)).collect();
            let all_truth: Vec<Vec<usize>> = serde_json::from_slice(
                &fs::read(Path::new(input_path).join("truth.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(all_truth.len(), query_count);
            let sample_queries: Vec<Vec<f32>> = (0..pass_sample_count)
                .map(|index| all_queries[index % query_count].clone())
                .collect();
            let sample_truth: Vec<Vec<usize>> = (0..pass_sample_count)
                .map(|index| all_truth[index % query_count].clone())
                .collect();
            let query_indices: Vec<usize> =
                (0..pass_sample_count).map(|index| index % query_count).collect();
            let saved_diagnostics = std::env::var_os("GB_VECTOR_QUERY_DIAGNOSTICS");
            if saved_diagnostics.is_some() {
                std::env::remove_var("GB_VECTOR_QUERY_DIAGNOSTICS");
            }
            stage(
                "query_sample_sweep_start",
                json!({
                    "sample_count":pass_sample_count,
                    "query_indices":query_indices,
                    "diagnostic_trace_disabled":saved_diagnostics.is_some(),
                }),
            );
            let (sample_result, sample_memory) = measure(|| {
                search(&storage, &sample_queries, &sample_truth, 10, 200)
            });
            if let Some(value) = saved_diagnostics {
                std::env::set_var("GB_VECTOR_QUERY_DIAGNOSTICS", value);
            }
            query_samples = json!({
                "state":if sample_result.get("error").is_some() {"error"} else {"ok"},
                "sample_kind":"additional warm cyclic query sweep after query0",
                "sample_count":pass_sample_count,
                "query_indices":query_indices,
                "diagnostic_trace_disabled":true,
                "metrics":sample_result,
                "memory":sample_memory,
            });
            stage("query_sample_sweep_complete", query_samples.clone());
        }
        if pass != repeat && query["state"].as_str() == Some("ok") && sample_count > 0 {
            query_samples = json!({
                "state":"skipped",
                "reason":"deferred until the final reopen so pass1 query remains adjacent to Drop",
            });
        }
    }

    let (metadata_summary, metadata_summary_memory) = if pass == repeat && coverage_ok {
        let (result, memory) = measure(|| storage.diagnostic_vector_metadata_summary("bench", 8));
        let value = match result {
            Ok(summary) => summary,
            Err(error) => json!({"state":"error","message":error.to_string()}),
        };
        (value, memory)
    } else {
        (
            json!({"state":"skipped","reason":"deferred until final repeat pass"}),
            Value::Null,
        )
    };
    stage("drop_start", json!({"pass":pass}));
    let ((), storage_drop_memory) = measure(|| drop(storage));
    stage("drop_complete", json!({"storage_drop_memory":storage_drop_memory}));
    json!({
        "pass":pass,
        "state":"ok",
        "open_memory":open_memory,
        "coverage":coverage,
        "coverage_memory":coverage_memory,
        "query":query,
        "query_memory":query_memory,
        "query_resources":query_resources,
        "query_samples":query_samples,
        "metadata_summary":metadata_summary,
        "metadata_summary_memory":metadata_summary_memory,
        "storage_drop_memory":storage_drop_memory,
    })
}

pub fn run() {
    // A new process separates reopen residency from retained ingest allocator pages.
    if let Ok(path) = std::env::var("GB_MEMORY_OPEN_ONLY") {
        let open_path = path.clone();
        let diagnostics = std::env::var_os("GB_VECTOR_QUERY_DIAGNOSTICS").is_some();
        let read_write = std::env::var_os("GB_MEMORY_OPEN_READ_WRITE").is_some();
        let input = std::env::var("GB_MEMORY_INPUT").ok();
        let repeat = number("GB_MEMORY_OPEN_REPEAT", 1);
        assert!((1..=2).contains(&repeat), "GB_MEMORY_OPEN_REPEAT must be 1 or 2");
        if std::env::var("GB_MEMORY_OPEN_MUTATE_BEFORE_REOPEN").ok().as_deref() == Some("1") {
            assert_eq!(repeat, 2, "GB_MEMORY_OPEN_MUTATE_BEFORE_REOPEN=1 requires exactly two opens");
            assert!(read_write, "GB_MEMORY_OPEN_MUTATE_BEFORE_REOPEN=1 requires GB_MEMORY_OPEN_READ_WRITE=1");
            assert!(diagnostics, "GB_MEMORY_OPEN_MUTATE_BEFORE_REOPEN=1 requires GB_VECTOR_QUERY_DIAGNOSTICS=1");
            assert_eq!(
                std::env::var("RAYON_NUM_THREADS").ok().as_deref(),
                Some("4"),
                "same-process reopen requires the original matrix setting RAYON_NUM_THREADS=4"
            );
            let input_path = input.as_deref().expect("mutation/reopen mode requires GB_MEMORY_INPUT");
            let sample_count = number("GB_MEMORY_OPEN_QUERY_SAMPLES", 24);
            assert_eq!(sample_count, 24, "mutation/reopen mode keeps the existing 24-query sample");
            assert_eq!(number("GB_EF", 200), 200, "mutation/reopen mode keeps GB_EF=200");
            let value = run_mutate_before_reopen(&path, input_path, sample_count);
            fs::write(
                std::env::var("GB_MEMORY_OPEN_OUT").expect("GB_MEMORY_OPEN_OUT is required"),
                serde_json::to_vec_pretty(&value).unwrap(),
            )
            .unwrap();
            return;
        }
        if repeat == 2 {
            assert!(read_write, "GB_MEMORY_OPEN_REPEAT=2 requires GB_MEMORY_OPEN_READ_WRITE=1");
            assert!(diagnostics, "GB_MEMORY_OPEN_REPEAT=2 requires GB_VECTOR_QUERY_DIAGNOSTICS=1");
            assert_eq!(
                std::env::var("RAYON_NUM_THREADS").ok().as_deref(),
                Some("4"),
                "GB_MEMORY_OPEN_REPEAT=2 requires the original matrix setting RAYON_NUM_THREADS=4"
            );
            let input_path = input
                .as_deref()
                .expect("GB_MEMORY_OPEN_REPEAT=2 requires GB_MEMORY_INPUT");
            let sample_count = number("GB_MEMORY_OPEN_QUERY_SAMPLES", 24);
            assert!(
                [0, 24, 120].contains(&sample_count),
                "GB_MEMORY_OPEN_QUERY_SAMPLES must be 0, 24, or 120"
            );
            let mut passes = Vec::with_capacity(repeat);
            for pass in 1..=repeat {
                let result =
                    run_open_repeat_pass(&path, input_path, pass, repeat, sample_count);
                let pass_failed = result["state"].as_str() != Some("ok");
                passes.push(result);
                if pass_failed {
                    break;
                }
            }
            let value = json!({
                "open_path":path,
                "open_mode":"read_write",
                "read_only":false,
                "vector_dim":1024,
                "repeat_count":repeat,
                "query_sample_count":sample_count,
                "same_process":true,
                "passes":passes,
                "rayon_num_threads":std::env::var("RAYON_NUM_THREADS").ok(),
                "query_budget_default_elapsed_ms":5000,
                "rss_limit_bytes":std::env::var("GB_MEMORY_RSS_LIMIT_BYTES").ok(),
            });
            fs::write(
                std::env::var("GB_MEMORY_OPEN_OUT").unwrap(),
                serde_json::to_vec_pretty(&value).unwrap(),
            )
            .unwrap();
            return;
        }
        let (result, memory) = measure(|| {
            Storage::open(OpenOptions {
                path,
                page_cache_mb: Some(64),
                read_only: Some(!read_write),
                vector_dim: if read_write { Some(1024) } else { None },
                retention: Some("full".into()),
            })
        });
        let value = match result {
            Ok(storage) => {
                let (coverage_result, coverage_memory) =
                    measure(|| storage.validate_index_coverage());
                let coverage_ok = coverage_result.is_ok();
                let coverage = match coverage_result {
                    Ok(report) => json!({"state":"ok","collections":report}),
                    Err(error) => json!({"state":"error","message":error.to_string()}),
                };
                let mut query = json!({"state":"skipped","reason":"diagnostics disabled"});
                let mut query_memory = Value::Null;
                let mut metadata_summary = Value::Null;
                let mut metadata_summary_memory = Value::Null;
                if diagnostics {
                    if !coverage_ok {
                        query = json!({"state":"skipped","reason":"index coverage failed"});
                    } else if let Some(input_path) = input {
                        let meta: Value = serde_json::from_slice(
                            &fs::read(Path::new(&input_path).join("meta.json")).unwrap(),
                        )
                        .unwrap();
                        let n = meta["n"].as_u64().unwrap_or(0);
                        let dim = meta["dim"].as_u64().unwrap_or(0) as usize;
                        let query_file = Path::new(&input_path).join("queries.f32");
                        let mut reader = BufReader::new(File::open(&query_file).unwrap());
                        let query_vector = row(&mut reader, dim)
                            .into_iter()
                            .map(f64::from)
                            .collect();
                        let (query_result, measured_query) = measure(|| {
                            storage.hybrid_search(HybridSearchInput {
                                query_vector,
                                k: 10,
                                alpha: Some(0.0),
                                lang: None,
                                as_of: None,
                                collection: Some("bench".into()),
                                ef_search: Some(200),
                                oversample: None,
                            })
                        });
                        query_memory = measured_query;
                        query = match query_result {
                            Ok(rows) => json!({
                                "state":"ok",
                                "input":input_path,
                                "input_n":n,
                                "input_dim":dim,
                                "query_index":0,
                                "requested_queries":1,
                                "completed_queries":1,
                                "result_count":rows.len(),
                                "first_hits":rows.iter().take(8).map(|row| json!({
                                    "id":row.node.id,
                                    "score":row.score,
                                })).collect::<Vec<_>>(),
                            }),
                            Err(error) => json!({
                                "state":"error",
                                "input":input_path,
                                "input_n":n,
                                "input_dim":dim,
                                "query_index":0,
                                "requested_queries":1,
                                "completed_queries":0,
                                "error":error.to_string(),
                            }),
                        };
                        let (summary_result, measured_summary) = measure(|| {
                            storage.diagnostic_vector_metadata_summary("bench", 8)
                        });
                        metadata_summary_memory = measured_summary;
                        metadata_summary = match summary_result {
                            Ok(summary) => summary,
                            Err(error) => json!({"state":"error","message":error.to_string()}),
                        };
                    } else {
                        query = json!({"state":"skipped","reason":"GB_MEMORY_INPUT not set"});
                    }
                }
                let listed_collections = storage.list_collections();
                // Measure the API's normal close/checkpoint path separately from
                // open and the first query. This diagnostic runs on a disposable
                // full copy when read_write is enabled.
                let ((), storage_drop_memory) = measure(|| drop(storage));
                let storage_drop_seconds = storage_drop_memory["seconds"].as_f64();
                json!({
                    "memory":memory,
                    "open_path":open_path,
                    "open_mode":if read_write {"read_write"} else {"read_only"},
                    "read_only":!read_write,
                    "vector_dim":if read_write {json!(1024)} else {Value::Null},
                    "rss_limit_bytes":std::env::var("GB_MEMORY_RSS_LIMIT_BYTES").ok(),
                    "collections":listed_collections,
                    "coverage":coverage,
                    "coverage_memory":coverage_memory,
                    "query":query,
                    "query_memory":query_memory,
                    "metadata_summary":metadata_summary,
                    "metadata_summary_memory":metadata_summary_memory,
                    "storage_drop_seconds":storage_drop_seconds,
                    "storage_drop_memory":storage_drop_memory,
                    "error":null,
                })
            }
            Err(error) => json!({
                "memory":memory,
                "open_path":open_path,
                "open_mode":if read_write {"read_write"} else {"read_only"},
                "read_only":!read_write,
                "vector_dim":if read_write {json!(1024)} else {Value::Null},
                "rss_limit_bytes":std::env::var("GB_MEMORY_RSS_LIMIT_BYTES").ok(),
                "error":error.to_string(),
            }),
        };
        fs::write(
            std::env::var("GB_MEMORY_OPEN_OUT").unwrap(),
            serde_json::to_vec_pretty(&value).unwrap(),
        )
        .unwrap();
        return;
    }
    let input = std::env::var("GB_VBENCH").expect("GB_VBENCH input directory");
    let input = Path::new(&input);
    if std::env::var_os("GB_MEMORY_PREPARE").is_some() {
        prepare(
            input,
            number("GB_VEC_N", 10_000),
            number("GB_VEC_DIM", 768),
            number("GB_VEC_Q", 24),
        );
        return;
    }
    let meta: Value = serde_json::from_slice(&fs::read(input.join("meta.json")).unwrap()).unwrap();
    let n = meta["n"].as_u64().unwrap() as usize;
    let dim = meta["dim"].as_u64().unwrap() as usize;
    let q = meta["q"].as_u64().unwrap() as usize;
    assert!(n >= 100 && q > 0);
    let quant = std::env::var("GB_QUANT").unwrap_or_else(|_| "none".into());
    let rerank = std::env::var("GB_RERANK").unwrap_or_default() == "1";
    let output = std::env::var("GB_MEMORY_OUT").expect("fresh GB_MEMORY_OUT directory");
    let output = Path::new(&output);
    fs::create_dir_all(output).unwrap();
    let db = output.join("gdb");
    assert!(!db.exists(), "refuse to overwrite a database");
    assert_eq!(
        fs::metadata(input.join("corpus.f32")).unwrap().len(),
        (n * dim * 4) as u64
    );
    let mut reader = BufReader::new(File::open(input.join("queries.f32")).unwrap());
    let queries: Vec<_> = (0..q).map(|_| row(&mut reader, dim)).collect();
    let truth: Vec<Vec<usize>> =
        serde_json::from_slice(&fs::read(input.join("truth.json")).unwrap()).unwrap();
    assert_eq!(truth.len(), q);
    assert!(truth
        .iter()
        .all(|v| v.len() >= 100 && v.iter().all(|&id| id < n)));
    let options = OpenOptions {
        path: db.to_string_lossy().into(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(dim as u32),
        retention: Some(std::env::var("GB_MEMORY_RETENTION").unwrap_or_else(|_| "full".into())),
    };
    let storage = Storage::open(options.clone()).unwrap();
    let ef = number("GB_EF", 200) as u32;
    storage.set_index_params(ef, ef);
    storage
        .create_collection(
            "bench".into(),
            meta["model"].as_str().unwrap().into(),
            dim as u32,
            Some("cosine".into()),
            Some(quant.clone()),
            None,
            Some(rerank),
        )
        .unwrap();
    let idle = rss();
    let (_, ingest) = measure(|| {
        let mut reader = BufReader::new(File::open(input.join("corpus.f32")).unwrap());
        for first in (0..n).step_by(256) {
            let nodes: Vec<_> = (first..(first + 256).min(n))
                .map(|id| NodeInput {
                    id: Some(id.to_string()),
                    labels: vec!["doc".into()],
                    props: None,
                    embedding: Some(
                        row(&mut reader, dim)
                            .into_iter()
                            .map(|x| x as f64)
                            .collect(),
                    ),
                    lang: None,
                    valid_from: None,
                    caused_by: None,
                    ttl: None,
                    collection: Some("bench".into()),
                })
                .collect();
            storage.bulk_add_nodes(nodes).unwrap();
            storage.flush_index(); // bound pending input vectors; includes indexing time
        }
    });
    let collections_after_ingest = storage.list_collections();
    let (_, rebuild) = measure(|| storage.rebuild_index_parallel().unwrap());
    let search10 = search(&storage, &queries, &truth, 10, ef);
    let search100 = search(&storage, &queries, &truth, 100, ef);
    let collections_after_search = storage.list_collections();
    fs::write(output.join("before_snapshot.json"),serde_json::to_vec_pretty(&json!({
        "n":n,"dim":dim,"quant":quant,"rerank":rerank,"idle_rss_bytes":idle,
        "ingest":ingest,"rebuild":rebuild,"search10":search10,"search100":search100,
        "collections_after_ingest":collections_after_ingest,"collections_after_search":collections_after_search
    })).unwrap()).unwrap();
    let (snapshot_result, snapshot) = measure(|| storage.save_state());
    let snapshot_error = snapshot_result.err().map(|e| e.to_string());
    let compaction = if std::env::var_os("GB_MEMORY_COMPACT").is_some() {
        Some(measure(|| storage.perform_index_compaction().unwrap()).1)
    } else {
        None
    };
    let disk = disk_bytes(&db);
    // Drop also checkpoints; report reopen in the same process (allocator may retain pages).
    drop(storage);
    let retention = options.retention.clone();
    let (reopened, reopen) = measure(|| Storage::open(options));
    let (coverage, reopen_search10, reopen_error) = match reopened {
        Ok(reopened) => (
            Some(reopened.validate_index_coverage().unwrap()),
            Some(search(&reopened, &queries, &truth, 10, ef)),
            None,
        ),
        Err(error) => (None, None, Some(error.to_string())),
    };
    let cold_out = output.join("fresh_reopen.json");
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .env("GB_MEMORY_OPEN_ONLY", &db)
        .env("GB_MEMORY_OPEN_OUT", &cold_out)
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "fresh reopen subprocess failed");
    let fresh_reopen: Value = serde_json::from_slice(&fs::read(cold_out).unwrap()).unwrap();
    let result = json!({"method":"20ms sampled working set; same-process and fresh-process reopen",
        "n":n,"dim":dim,"q":q,"quant":quant,"rerank":rerank,"model":meta["model"],
        "batch_rows":256,"ef_construction":ef,"ef_search":ef,
        "idle_rss_bytes":idle,"ingest":ingest,"rebuild":rebuild,
        "index_rebuild_peak_rss_bytes":rebuild["peak_rss_bytes"],"snapshot":snapshot,"reopen":reopen,
        "disk_bytes":disk,"retention":retention,"snapshot_error":snapshot_error,"reopen_error":reopen_error,
        "compaction":compaction,"fresh_reopen":fresh_reopen,"collections_after_ingest":collections_after_ingest,
        "collections_after_search":collections_after_search,"search10":search10,
        "search100":search100,"reopen_search10":reopen_search10,"coverage":coverage});
    fs::write(
        output.join("memory.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    println!(
        "memory audit: {n}x{dim} {quant} rerank={rerank}: {}",
        output.display()
    );
}

/// Opt-in same-process regression workload: mutate only through Storage, Drop to
/// write its ordinary valid snapshot, then reopen and issue q0 before any probes.
fn run_mutate_before_reopen(path: &str, input_path: &str, sample_count: usize) -> Value {
    let input_root = Path::new(input_path);
    let input_meta: Value = serde_json::from_slice(&fs::read(input_root.join("meta.json")).unwrap()).unwrap();
    let input_n = input_meta["n"].as_u64().unwrap_or(0);
    let dim = input_meta["dim"].as_u64().unwrap_or(0) as usize;
    let query_count = input_meta["q"].as_u64().unwrap_or(0) as usize;
    assert_eq!(input_n, 100_000, "this reproducer requires the unchanged 100k vector fixture");
    assert_eq!(dim, 1024, "this reproducer requires the unchanged 1024-dimensional fixture");
    assert_eq!(sample_count, 24, "this reproducer uses the existing 24-query sample");
    assert!(query_count >= sample_count, "fixture must contain at least 24 queries");

    // Read the small query fixture before opening Storage. The first operation on
    // the reopened database remains query 0, with no coverage/metadata preflight.
    let query_reader_file = File::open(input_root.join("queries.f32")).unwrap();
    let mut query_reader = BufReader::new(query_reader_file);
    let all_queries: Vec<Vec<f32>> = (0..query_count).map(|_| row(&mut query_reader, dim)).collect();
    let all_truth: Vec<Vec<usize>> = serde_json::from_slice(&fs::read(input_root.join("truth.json")).unwrap()).unwrap();
    assert_eq!(all_truth.len(), query_count, "fixture truth rows must match query count");

    let stage = |name: &str, extra: Value| {
        eprintln!("GB_MEMORY_MUTATE_REOPEN {}", json!({"stage":name,"extra":extra}));
    };
    let options = || OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(1024),
        retention: Some("full".into()),
    };

    stage("first_open_start", json!({"path":path,"input_n":input_n,"dim":dim}));
    let (first_open_result, first_open_memory) = measure(|| Storage::open(options()));
    let first_storage = first_open_result.expect("first writable Storage open failed");
    stage("first_open_complete", json!({"memory":first_open_memory}));
    let marker_id = format!(
        "sqlite-publication-probe-{}",
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    );
    let (marker_result, marker_memory) = measure(|| first_storage.add_node(NodeInput {
        id: Some(marker_id),
        labels: vec!["doc".into()],
        props: Some(json!({"marker":"sqlite-publication-probe"})),
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: Some("bench".into()),
    }));
    let marker = marker_result.expect("Storage.add_node publication marker failed");
    stage("marker_added", json!({"id":marker.id,"memory":marker_memory}));
    let ((), first_drop_memory) = measure(|| drop(first_storage));
    stage("first_drop_complete", json!({"memory":first_drop_memory,"snapshot":"normal Storage::drop"}));

    stage("second_open_start", json!({"path":path}));
    let (second_open_result, second_open_memory) = measure(|| Storage::open(options()));
    let storage = second_open_result.expect("second writable Storage open failed");
    stage("second_open_complete", json!({"memory":second_open_memory,"next":"query0"}));

    let q0_query = std::slice::from_ref(&all_queries[0]);
    let q0_truth = std::slice::from_ref(&all_truth[0]);
    stage("query0_start", json!({"query_index":0,"k":10,"ef_search":200,"alpha":0.0}));
    let (query0, query0_memory) = measure(|| search(&storage, q0_query, q0_truth, 10, 200));
    stage("query0_complete", json!({"query":query0,"memory":query0_memory}));

    // Match the existing aggregate search contract for both k values, with
    // diagnostic tracing disabled across both sweeps and restored afterward.
    let saved_diagnostics = std::env::var_os("GB_VECTOR_QUERY_DIAGNOSTICS");
    if saved_diagnostics.is_some() {
        std::env::remove_var("GB_VECTOR_QUERY_DIAGNOSTICS");
    }
    let query_indices: Vec<usize> = (0..sample_count).collect();
    stage("query_sample_k10_start", json!({"sample_count":sample_count,"query_indices":query_indices}));
    let (sample_k10, sample_k10_memory) = measure(|| {
        search(&storage, &all_queries[..sample_count], &all_truth[..sample_count], 10, 200)
    });
    stage("query_sample_k100_start", json!({"sample_count":sample_count,"query_indices":query_indices}));
    let (sample_k100, sample_k100_memory) = measure(|| {
        search(&storage, &all_queries[..sample_count], &all_truth[..sample_count], 100, 200)
    });
    if let Some(value) = saved_diagnostics {
        std::env::set_var("GB_VECTOR_QUERY_DIAGNOSTICS", value);
    }
    stage("query_sample_sweeps_complete", json!({
        "k10":{"metrics":sample_k10,"memory":sample_k10_memory},
        "k100":{"metrics":sample_k100,"memory":sample_k100_memory},
    }));

    let (coverage_result, coverage_memory) = measure(|| storage.validate_index_coverage());
    let coverage_ok = coverage_result.is_ok();
    let coverage = match coverage_result {
        Ok(report) => json!({"state":"ok","collections":report}),
        Err(error) => json!({"state":"error","message":error.to_string()}),
    };
    stage("coverage_complete", json!({"coverage":coverage,"memory":coverage_memory}));
    let (metadata_summary, metadata_summary_memory) = if coverage_ok {
        let (result, memory) = measure(|| storage.diagnostic_vector_metadata_summary("bench", 8));
        let summary = match result {
            Ok(summary) => summary,
            Err(error) => json!({"state":"error","message":error.to_string()}),
        };
        (summary, memory)
    } else {
        (json!({"state":"skipped","reason":"index coverage failed"}), Value::Null)
    };
    stage("metadata_summary_complete", json!({"summary":metadata_summary,"memory":metadata_summary_memory}));
    let metadata_node_count = metadata_summary.get("node_count").and_then(Value::as_u64);
    let metadata_count = metadata_summary.get("metadata_count").and_then(Value::as_u64);
    let metadata_counts_ok = metadata_node_count == Some(100_001) && metadata_count == Some(100_000);
    let coverage_bench = coverage.get("collections").and_then(Value::as_array).and_then(|reports| {
        reports.iter().find(|report| report["collection"].as_str() == Some("bench"))
    }).cloned().unwrap_or(Value::Null);
    let coverage_bench_ok = coverage_bench["state"].as_str() == Some("READY")
        && coverage_bench["source_count"].as_u64() == Some(100_000)
        && coverage_bench["indexed_count"].as_u64() == Some(100_000);
    // Validate the legitimate non-vector write only after all queries and
    // diagnostics, keeping query 0 first on the reopened Storage.
    let marker_node = storage.node_view(&marker.id);
    let marker_recovered = marker_node.as_ref()
        .map(|node| node.props["marker"].as_str() == Some("sqlite-publication-probe"))
        .unwrap_or(false);
    stage("mutation_recovery_check", json!({
        "marker_recovered":marker_recovered,
        "metadata_node_count":metadata_node_count,
        "metadata_count":metadata_count,
        "coverage_bench":coverage_bench,
    }));
    let ((), second_drop_memory) = measure(|| drop(storage));

    let query0_ok = query0.get("error").is_none();
    let sample_k10_ok = sample_k10.get("error").is_none();
    let sample_k100_ok = sample_k100.get("error").is_none();
    let query0_result_count_ok = query0["result_count_min"].as_u64() == Some(10)
        && query0["result_count_max"].as_u64() == Some(10);
    let sample_k10_result_count_ok = sample_k10["result_count_min"].as_u64() == Some(10);
    let sample_k100_result_count_ok = sample_k100["result_count_min"].as_u64() == Some(100);
    let result_count_contract_ok = query0_result_count_ok
        && sample_k10_result_count_ok && sample_k100_result_count_ok;
    json!({
        "state":if query0_ok && sample_k10_ok && sample_k100_ok && result_count_contract_ok
            && coverage_ok && metadata_counts_ok && coverage_bench_ok && marker_recovered {"ok"} else {"error"},
        "open_path":path,
        "open_mode":"read_write",
        "read_only":false,
        "vector_dim":1024,
        "repeat_count":2,
        "same_process":true,
        "mutation_mode":"GB_MEMORY_OPEN_MUTATE_BEFORE_REOPEN=1",
        "mutation":"Storage.add_node(non-vector marker), then normal Storage drop",
        "marker_id":marker.id,
        "input_n":input_n,
        "input_dim":dim,
        "expected_nodes_after_marker":100_001,
        "expected_vector_nodes":100_000,
        "first_open_memory":first_open_memory,
        "marker_memory":marker_memory,
        "first_drop_memory":first_drop_memory,
        "second_open_memory":second_open_memory,
        "query0":query0,
        "query0_memory":query0_memory,
        "result_count_contract":{"q0_exact_10":query0_result_count_ok,"sample_k10_min_10":sample_k10_result_count_ok,"sample_k100_min_100":sample_k100_result_count_ok,"passed":result_count_contract_ok},
        "retrieval_quality_gate":"none; recall is reported for comparison against existing contracts and the real-embedding pair",
        "query_sample_count":sample_count,
        "query_samples_k10":sample_k10,
        "query_samples_k10_memory":sample_k10_memory,
        "query_samples_k100":sample_k100,
        "query_samples_k100_memory":sample_k100_memory,
        "coverage":coverage,
        "coverage_bench":coverage_bench,
        "coverage_bench_ok":coverage_bench_ok,
        "coverage_memory":coverage_memory,
        "metadata_node_count":metadata_node_count,
        "metadata_count":metadata_count,
        "metadata_counts_ok":metadata_counts_ok,
        "marker_recovered":marker_recovered,
        "metadata_summary":metadata_summary,
        "metadata_summary_memory":metadata_summary_memory,
        "second_drop_memory":second_drop_memory,
        "rayon_num_threads":std::env::var("RAYON_NUM_THREADS").ok(),
        "query_budget_default_elapsed_ms":5000,
        "rss_limit_bytes":std::env::var("GB_MEMORY_RSS_LIMIT_BYTES").ok(),
    })
}