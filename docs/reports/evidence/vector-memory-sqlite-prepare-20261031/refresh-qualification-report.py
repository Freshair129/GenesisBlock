from pathlib import Path
r=Path(__file__).parent
p=Path(r'D:\GenesisBlock_Dev\GenesisBlock\docs\reports\GENESISDB-VECTOR-MEMORY-AUDIT.md')
s=p.read_text(encoding='utf-8')
s=s.replace('## SQLite prepare follow-up: confirmed deferral, final gates on resource hold','## SQLite prepare follow-up: confirmed deferral, final correctness gates passed')
s=s.replace('The source-bound qualification receipt deliberately remains `passed:false`.','The source-bound correctness receipt is `passed:true` for the final immutable source; it does not qualify the pending timed workloads.')
s=s.replace('The source-bound qualification receipt deliberately\nremains `passed:false`.','The source-bound correctness receipt is `passed:true` for the final immutable source;\nit does not qualify the pending timed workloads.')
s=s.replace('| Final whole-suite attempt | 85 targets completed, zero failures; `hql_cypher_tests` interrupted by resource hold at 216.8 s, before its 300 s cap; not a complete suite or a timeout result |','| Earlier final-source attempt (preserved) | 85 targets completed, zero failures; `hql_cypher_tests` interrupted by resource hold at 216.8 s, before its 300 s cap; not a complete suite or a timeout result |\n| Fresh complete final-source suite | 159 targets; 1,328 passed / 0 failed / 7 ignored; original 300 s / 4 GiB per executable; frozen executable hashes and 287-path source manifest bound to receipt |')
s=s.replace('| Final strict Clippy, Node/NAPI and host-mobile checks | Pending after the test-only literal fix; earlier source-stage results are not relabeled as final |','| Final strict Clippy, Node/NAPI and host-mobile checks | All 8 gates passed: strict Clippy core/default, host-mobile compile, doctests, isolated NAPI build, DTS freshness, native smoke, Node 88 passed / 0 failed / 0 skipped |')
s=s.replace('hashed; the interrupted set requires a fresh complete run after resource release.','hashed; a fresh complete final-source run supersedes the interrupted attempt.\nRaw fresh logs are under `final-runtime-fresh-20261031T213000/`;\n`qualification-receipt.json` binds the full source manifest and four final gate files.\nThe receipt covers correctness only, not the pending timing/100k claims.')
p.write_text(s,encoding='utf-8',newline='\n')
print('Updated final correctness status; pending benchmark claims retained.')
