from pathlib import Path
r=Path(__file__).parent
old=Path(r'C:\Users\suanr\genesis-vector-memory-followup-20261031\final-runtime.py').read_text()
old=old.replace("r=pathlib.Path(r'C:\\Users\\suanr\\genesis-vector-memory-followup-20261031')", "r=pathlib.Path(r'C:\\Users\\suanr\\genesis-sqlite-prepare-20261031')")
old=old.replace("work=pathlib.Path(r'D:\\GenesisBlock_Dev\\GenesisBlock')", "work=pathlib.Path(r'C:\\Users\\suanr\\genesis-vector-memory-20261031\\verify-src')")
old=old.replace("# The interrupted concurrency target is rerun below with the heavy bulk case excluded.","# Same whole-executable acceptance protocol: no filters or skipped targets.")
old=old.replace("while not (r/'final-core-build.status').exists() or not (r/'six2-runtime.status').exists():time.sleep(1)\nassert (r/'final-core-build.status').read_text().strip()=='0'", "while not (r/'all-core-build/status.json').exists():time.sleep(1)\nbuild_status=json.loads((r/'all-core-build/status.json').read_text(encoding='utf-8-sig'))\nassert build_status['exit']==0 and not build_status['reason'],build_status")
old=old.replace("r/'final-core-build.jsonl'", "r/'all-core-build/stdout.log'")
old=old.replace("('src/lib.rs','src/vector_memory_tests.rs','Cargo.toml','Cargo.lock')", "('src/lib.rs','src/vector_memory_tests.rs','src/projection_rebuild_tests.rs','Cargo.toml','Cargo.lock')")
old=old.replace(" is_concurrency=False\n if exe.name in started and not is_concurrency: continue\n", "")
old=old.replace(" if is_concurrency: args+=['--skip','bulk_add_edges_1000']\n", "")
assert '--skip' not in old
(r/'run-full-core.py').write_text(old,newline='\n')
