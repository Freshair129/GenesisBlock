from pathlib import Path
import hashlib,json,datetime
r=Path(__file__).parent
p=r/'final-regression-projection_rebuild_tests.rs'; b=p.read_bytes(); old=b.replace(b'c"main".as_ptr()',b'b"main\\0".as_ptr().cast()')
assert hashlib.sha256(old).hexdigest()=='40c4a131c4137304764b38a61d5d4a1482893796fd5e032d8292653a966cd342'
(r/'prelint-tests-reconstructed.rs').write_bytes(old)
(r/'prelint-test-reconstruction.json').write_text(json.dumps({'note':'Lossless single-literal reversal from final E69 source; verified against pre-lint compile receipt. Reconstructed source, not an original snapshot.','sha256':hashlib.sha256(old).hexdigest()},indent=2)+'\n',newline='\n')
orig=r/'proposal-status.json'
if not (r/'proposal-status-at-design.json').exists():(r/'proposal-status-at-design.json').write_bytes(orig.read_bytes())
orig.write_text(json.dumps({'recorded_at':datetime.datetime.now().astimezone().isoformat(),'status':'applied_to_worktree; mechanism GREEN; final qualification resource-held','core_sha256':hashlib.sha256((r/'core-current.rs').read_bytes()).hexdigest(),'final_test_sha256':hashlib.sha256(b).hexdigest(),'historical_design_status':'proposal-status-at-design.json','qualification_passed':False},indent=2)+'\n',newline='\n')
p=r/'collect-evidence.py';s=p.read_text();s=s.replace("('entry-lib.rs','proposed-publication-lib.rs','unrun-projection_rebuild_tests.rs')","('proposed-publication-lib.rs','unrun-projection_rebuild_tests.rs')");p.write_text(s,newline='\n')
