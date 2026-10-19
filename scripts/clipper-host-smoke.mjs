import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {join,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
const root=dirname(dirname(fileURLToPath(import.meta.url))),id=process.argv[2]??Date.now().toString();assert.match(id,/^[a-z0-9-]{1,64}$/);const out=join(root,'artifacts',`clipper-host-${id}`);await mkdir(out);const eid=(await readFile(join(root,'browser-extension/protocol/extension-id.txt'),'utf8')).trim();const origin=`chrome-extension://${eid}/`;
const report={mode:'actual standalone Windows Host process with hostile/partial stdio frames, no UI or real user configuration',checks:[],errors:[]};
async function run(bytes,source=origin,leaveOpen=false){const p=spawn(join(root,process.argv.includes('--release')?'browser-extension/target/release/researchclip-native-host.exe':'browser-extension/target/debug/researchclip-native-host.exe'),[source],{windowsHide:true,stdio:['pipe','pipe','pipe'],env:{...process.env,RESEARCHCLIP_TEST_SETTINGS_DIR:out,APPDATA:out}});let data=Buffer.alloc(0),stderr='';p.stdout.on('data',d=>data=Buffer.concat([data,d]));p.stderr.on('data',d=>stderr+=d);if(bytes)p.stdin.write(bytes);if(!leaveOpen)p.stdin.end();const code=await new Promise((resolve,reject)=>{const timer=setTimeout(()=>{p.kill();reject(Error('Owned host exceeded deadline'));},15000);p.on('exit',c=>{clearTimeout(timer);resolve(c);});p.on('error',reject);});assert.equal(code,0);assert.equal(stderr,'');assert.equal(data.readUInt32LE(0),data.length-4);return JSON.parse(data.subarray(4).toString('utf8'));}
const packet=s=>{const b=Buffer.from(s),h=Buffer.alloc(4);h.writeUInt32LE(b.length);return Buffer.concat([h,b]);};
try{
 assert.equal((await run(null,'chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/')).code,'ORIGIN_DENIED');report.checks.push('different extension origin rejected before reading stdin');
 const large=Buffer.alloc(4);large.writeUInt32LE(1024*1024);assert.equal((await run(large)).code,'INVALID_MESSAGE');report.checks.push('oversize declared frame rejected without allocating it');
 for(const payload of [Buffer.from([128,0,0,0,123,125]),packet('{broken'),packet(JSON.stringify({version:1,requestId:'a'.repeat(32),operation:'ping',clip:null,path:'C:/private'}))])assert.equal((await run(payload)).code,'INVALID_MESSAGE');report.checks.push('truncated frame, malformed JSON and unknown local path field rejected');
 const start=Date.now();assert.equal((await run(null,origin,true)).code,'INVALID_MESSAGE');assert.ok(Date.now()-start>=9000);report.checks.push('incomplete native caller times out after actual ten-second read deadline');
 assert.equal((await run(packet(JSON.stringify({version:1,requestId:'b'.repeat(32),operation:'ping',clip:null})))).code,'DESKTOP_UNAVAILABLE');report.checks.push('absent desktop endpoint produces a user-facing recoverable error');report.result='PASS';
}catch(e){report.result='FAIL';report.failure=e.stack;process.exitCode=1;}finally{await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));}
