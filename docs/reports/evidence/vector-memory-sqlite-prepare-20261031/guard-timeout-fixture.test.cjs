const {test}=require('node:test');
const {spawn}=require('node:child_process');
const fs=require('node:fs');
test('intentional timeout with owned child',async()=>{
 const child=spawn(process.execPath,['-e','setInterval(()=>{},1000)'],{stdio:'ignore',windowsHide:true});
 fs.writeFileSync(__dirname+'/guard-fixture-child.json',JSON.stringify({root_worker_pid:process.pid,child_pid:child.pid}));
 setInterval(()=>{},1000);
 await new Promise(()=>{});
});
