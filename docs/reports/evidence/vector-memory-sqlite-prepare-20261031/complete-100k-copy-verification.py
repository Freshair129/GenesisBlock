from pathlib import Path
import runpy,shutil,json
r=Path(__file__).parent
api=runpy.run_path(str(r/'prepare-100k-cases.py'))
source=api['SOURCE_DB'];cases=r/'100k-cases'
manifest=cases/'copy-manifest.json'
assert not manifest.exists(),'Refusing existing copy manifest'
assert shutil.disk_usage(r).free>=api['MIN_FREE_BYTES']
wal_before=api['source_wal_state']()
trees=[source,cases/'before/gdb',cases/'after/gdb',source]
for p in trees[1:3]:api['require_under'](p,r)
values=[api['tree_manifest'](p) for p in trees]
assert all(v==values[0] for v in values),'Interrupted copy is incomplete or source changed; no DB may be opened'
wal_after=api['source_wal_state']()
out={'schema_version':1,'operation':'Complete full byte-hash verification after interrupted copy preparation',
     'prior_guard':'100k-copy-preparation-final/status.json','prior_disposition':'resource_competition, owned Python stopped; no DB opened',
     'sqlite_opened':False,'benchmark_launched':False,'source_db':str(source),'rnew_root':str(r),
     'wal_before':wal_before,'wal_after':wal_after,'source':values[0],'source_after':values[3],
     'before':values[1],'after':values[2],'copies_equal':True,'source_matches_both_copies':True,
     'equal_file_count':True,'equal_byte_count':True,'database_files_written_by_this_verifier':False}
api['write_json'](manifest,out)
print(json.dumps({'copies_equal':True,'source_matches_both_copies':True,'file_count_each':values[0]['file_count'],
 'bytes_each':values[0]['byte_count'],'tree_sha256':values[0]['tree_sha256'],'sqlite_opened':False}))
