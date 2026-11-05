const fs=require('node:fs');const path=require('node:path');const crypto=require('node:crypto');
const [mode,addon,dbpath,ntext,output]=process.argv.slice(2);const n=Number(ntext);const {GenesisDatabase}=require(addon);
(async()=>{
 const result={mode,n,dim:8,retention:'default',addon};
 if(mode==='write'&&fs.existsSync(dbpath))throw new Error('refuse to overwrite a diagnostic DB');
 const db=GenesisDatabase.open({path:dbpath,pageCacheMb:16,readOnly:mode==='open',vectorDim:8});
 if(mode==='write'){
  await db.bulkAddNodes(Array.from({length:n},(_,id)=>({id:`proof-${id}`,labels:['Test'],props:{ord:id},embedding:[1,(id%7)*0.125,0,0,0,0,0,0]})));
  await db.flushIndex();result.ingest_complete=true;result.collection=db.listCollections().find(x=>x.name==='default');
  fs.writeFileSync(output,JSON.stringify({...result,checkpoint:'pending'},null,2));
  const t=performance.now();try{await db.saveState();result.checkpoint_success=true;result.checkpoint_error=null}catch(e){result.checkpoint_success=false;result.checkpoint_error=String(e.message||e)}
  result.checkpoint_seconds=(performance.now()-t)/1000;
  const file=path.join(dbpath,'vec_default.bin');if(fs.existsSync(file)){const bytes=fs.readFileSync(file);result.vector_file_bytes=bytes.length;result.vector_file_sha256=crypto.createHash('sha256').update(bytes).digest('hex')}
 }else{
  const c=db.listCollections().find(x=>x.name==='default');result.collection=c;result.count_and_indexed_match=c.count===n&&c.indexed===n;
  const hits=await db.hybridSearch({queryVector:[1,0,0,0,0,0,0,0],k:10,alpha:0});result.search_count=hits.length;result.search_ids=hits.map(x=>x.node.id);
  result.search_sanity=hits.length===Math.min(n,10)&&hits.every(x=>/^proof-\d+$/.test(x.node.id)&&Number(x.node.id.slice(6))<n);
 }
 fs.writeFileSync(output,JSON.stringify(result,null,2));process.exit(0);
})().catch(e=>{fs.writeFileSync(output,JSON.stringify({mode,n,error:String(e.stack||e)},null,2));process.exit(1)});
