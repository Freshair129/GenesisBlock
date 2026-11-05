import pathlib,json
r=pathlib.Path(__file__).parent
rows=[]
for s in (r/'all-core-build/stdout.log').read_text(errors='replace').splitlines():
 try:d=json.loads(s)
 except:continue
 if d.get('reason')=='compiler-artifact' and d.get('profile',{}).get('test') and d.get('executable'): rows.append(d['target']['name'])
print(json.dumps({'compiled_test_targets':len(rows),'recent':rows[-5:]}))
