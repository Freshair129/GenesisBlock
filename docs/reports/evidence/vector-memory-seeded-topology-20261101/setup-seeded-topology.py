from pathlib import Path
import hashlib, json, shutil

r = Path(__file__).parent
source = r / 'release-src/query-evidence-20261101'
crate_src = Path.home() / '.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hnsw_rs-0.3.4'
out = r / 'seeded-topology-20261101'
assert not out.exists()
out.mkdir()
crate = out / 'hnsw_rs'
shutil.copytree(crate_src, crate)
path = crate / 'src/hnsw.rs'
s = path.read_text(encoding='utf-8')
needle = 'StdRng::from_os_rng()'
assert s.count(needle) == 2
s = s.replace(needle, 'match std::env::var("GB_MEMORY_HNSW_SEED") { Ok(seed) => StdRng::seed_from_u64(seed.parse::<u64>().expect("invalid diagnostic HNSW seed")), Err(_) => StdRng::from_os_rng() }')
path.write_text(s, encoding='utf-8', newline='\n')
hook = '''
        if let (Ok(dir), VecIndex::F32(h)) = (std::env::var("GB_MEMORY_TOPOLOGY_DIR"), &index) {
            use std::io::Write as _;
            static SEQUENCE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let sequence = SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = std::path::Path::new(&dir).join(format!("topology-{}-{}.txt", std::process::id(), sequence));
            let mut out = std::io::BufWriter::new(std::fs::File::create(path).expect("topology create"));
            let mut points: Vec<_> = h.get_point_indexation().into_iter().collect();
            points.sort_unstable_by_key(|p| p.get_origin_id());
            for point in points {
                let id = point.get_point_id();
                write!(out, "{} {} {}", point.get_origin_id(), id.0, id.1).expect("topology write");
                for (layer, neighbors) in point.get_neighborhood_id().iter().enumerate() {
                    write!(out, " |{}", layer).expect("topology write");
                    for n in neighbors {
                        write!(out, " {}:{}:{}:{:08x}", n.d_id, n.p_id.0, n.p_id.1, n.distance.to_bits()).expect("topology write");
                    }
                }
                writeln!(out).expect("topology write");
            }
            out.flush().expect("topology flush");
        }
'''
manifest = {'seed_patch': hashlib.sha256(path.read_bytes()).hexdigest(), 'crate_original': str(crate_src), 'cases': {}}
for mode, lib in [('before', r/'entry-lib.rs'), ('after', r/'proposed-publication-lib.rs')]:
    dst = out / mode
    shutil.copytree(source, dst)
    code = lib.read_text(encoding='utf-8')
    target = '        *self.hnsw.write() = Some(index);\n        *self.coverage.write() = None;\n    }\n\n    fn validate_coverage'
    assert code.count(target) == 1
    code = code.replace(target, hook + target)
    (dst/'src/lib.rs').write_text(code, encoding='utf-8', newline='\n')
    manifest['cases'][mode] = {'original_lib_sha256': hashlib.sha256(lib.read_bytes()).hexdigest(), 'diagnostic_lib_sha256': hashlib.sha256((dst/'src/lib.rs').read_bytes()).hexdigest()}
(out/'manifest.json').write_text(json.dumps(manifest, indent=2)+'\n')
print(json.dumps(manifest, indent=2))
