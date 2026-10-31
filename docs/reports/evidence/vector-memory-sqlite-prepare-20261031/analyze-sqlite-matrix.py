from pathlib import Path
import json,re,sys
r=Path(__file__).parent
folder=sys.argv[1] if len(sys.argv)>1 else 'matrix-real'
x=json.loads((r/folder/'results.json').read_text())
rows=[]
for case in x['cases']:
 mode=case['mode'];s=(r/folder/f'{mode}.stdout.log').read_text()
 assert 'sqlite_version="3.46.0"' in s,'unexpected SQLite runtime version'
 def state(label):
  m=re.search(r'wal_index stage='+re.escape(label)+r' mx_frame=(\d+) n_backfill=(\d+)',s)
  return {'mx_frame':int(m[1]),'n_backfill':int(m[2])} if m else None
 q={}
 for m in re.finditer(r'query=(\w+) prepare_us=(\d+) row_us=(\d+) finalize_us=(\d+) payload_len_bytes=(\d+) payload=(.*)',s):
  q[m[1]]={'prepare_us':int(m[2]),'row_us':int(m[3]),'finalize_us':int(m[4]),'payload':m[6]}
 maintenance=re.search(r'publication_maintenance_us=(\d+)',s)
 assert case['exit']==0 and set(q)=={'first','second'} and all(v['payload']=='{}' for v in q.values())
 rows.append({'mode':mode,'seconds_including_backup_and_connection_lifecycle':case['seconds'],'first':q['first'],'second':q['second'],
              'after_backup':state('after_source_drop_before_select'),'before_first_prepare':state('first_before_prepare'),
              'after_first_prepare':state('first_after_prepare'),'publication_maintenance_us':int(maintenance[1]) if maintenance else None,
              'after_publication_maintenance':state('after_publication_maintenance')})
assert len(rows)==4 and x['source_unchanged']
by={i['mode']:i for i in rows}
normal=by['backup'];disabled=by['backup-noauto'];fixed=by['backup-checkpoint']
mechanism=(normal['after_backup']['mx_frame']>=1000 and normal['after_backup']['n_backfill']<normal['after_backup']['mx_frame']
 and normal['after_first_prepare']['n_backfill']==normal['after_backup']['mx_frame']
 and disabled['after_first_prepare']['n_backfill']==disabled['after_backup']['n_backfill']
 and fixed['after_publication_maintenance']['n_backfill']==fixed['after_publication_maintenance']['mx_frame']
 and fixed['after_first_prepare']==fixed['before_first_prepare'])
out={'passed_contents_and_source_integrity':True,'mechanism_observed':mechanism,'source_sha256':x['source_sha256_after'],
 'binary_sha256':x['binary_sha256'],'cases':rows,'method':'Pinned SQLite only, no Genesis projection replay; fixed order, no OS cache reset, no SQL warmup. Second query is reported separately and never substitutes for the first.'}
(r/('matrix-analysis.json' if folder=='matrix-real' else folder+'-analysis.json')).write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps(out,indent=2))
assert mechanism
