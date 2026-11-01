from pathlib import Path
import hashlib, json
r=Path(__file__).parent
out={}
for mode in ('before','after'):
 d=r/mode
 p=d/'src/lib.rs'
 code=p.read_text(encoding='utf-8')
 start=code.index('        if let (Ok(dir), VecIndex::F32(h)) = (std::env::var("GB_MEMORY_TOPOLOGY_DIR"), &index) {')
 end=code.index('        *self.hnsw.write() = Some(index);',start)
 block=code[start:end]
 code=code[:start]+code[end:]
 block=block.replace('if let (Ok(dir), VecIndex::F32(h)) = (std::env::var("GB_MEMORY_TOPOLOGY_DIR"), &index) {','if let (Ok(dir), Some(VecIndex::F32(h))) = (std::env::var("GB_MEMORY_TOPOLOGY_DIR"), guard.as_ref()) {')
 block=block.replace('            static SEQUENCE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);\n            let sequence = SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);\n','')
 block=block.replace('format!("topology-{}-{}.txt", std::process::id(), sequence)', 'format!("topology-{}-{}.txt", std::process::id(), label)')
 method='''    /// Diagnostic-only graph capture; invoked only after the measured queries.
    pub fn diagnostic_capture_topology(&self, label: &str) {
        let Some(collection) = self.collections.get("bench") else { return };
        let guard = collection.hnsw.read();
'''+block+'''    }

'''
 marker='    pub fn validate_index_coverage(&self) -> Result<Vec<IndexCoverageReport>> {'
 assert code.count(marker)==1
 code=code.replace(marker,method+marker)
 p.write_text(code,encoding='utf-8',newline='\n')
 p=d/'benches/vector_memory.rs'; bench=p.read_text(encoding='utf-8')
 needle='    let search100 = search(&storage, &queries, &truth, 100, ef);'
 assert bench.count(needle)==1
 bench=bench.replace(needle,needle+'\n    storage.diagnostic_capture_topology("initial");')
 old='''        Ok(reopened) => (
            Some(reopened.validate_index_coverage().unwrap()),
            Some(search(&reopened, &queries, &truth, 10, ef)),
            None,
        ),'''
 new='''        Ok(reopened) => {
            let coverage = Some(reopened.validate_index_coverage().unwrap());
            let results = Some(search(&reopened, &queries, &truth, 10, ef));
            reopened.diagnostic_capture_topology("reopen");
            (coverage, results, None)
        },'''
 assert bench.count(old)==1
 bench=bench.replace(old,new)
 p.write_text(bench,encoding='utf-8',newline='\n')
 out[mode]={'lib_sha256':hashlib.sha256((d/'src/lib.rs').read_bytes()).hexdigest(), 'bench_sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
(r/'postquery-manifest.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps(out,indent=2))
