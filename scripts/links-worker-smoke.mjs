import assert from 'node:assert/strict';
import {readdir} from 'node:fs/promises';
import {Worker} from 'node:worker_threads';
import {once} from 'node:events';
import {pathToFileURL} from 'node:url';
import {resolve} from 'node:path';
const files=(await readdir('dist/assets')).filter(n=>/^index-worker-.*\.js$/.test(n));assert.equal(files.length,1);
const worker=new Worker(`const {parentPort,workerData}=require('node:worker_threads');globalThis.postMessage=v=>parentPort.postMessage(v);parentPort.on('message',data=>globalThis.onmessage({data}));import(workerData.url).then(()=>parentPort.postMessage({ready:true}));`,{eval:true,workerData:{url:pathToFileURL(resolve('dist/assets',files[0])).href}});
try{
 assert.deepEqual((await once(worker,'message'))[0],{ready:true});
 const target='a'.repeat(32),raw='[A &amp; B](researchclip://document/'+target+')',text='中文😀\r\n'+raw+'\n\n`'+raw+'`';
 worker.postMessage(text);const [{links,error}]=await once(worker,'message');assert.equal(error,undefined);assert.equal(links.length,1);assert.equal(links[0].label,'A & B');assert.equal(links[0].start,'中文😀\r\n'.length);
 worker.postMessage((raw+'\n\n').repeat(5001));const [oversize]=await once(worker,'message');assert.match(oversize.error,/5000/);
 console.log(JSON.stringify({result:'PASS',checks:2,mode:'actual production Worker bundle loaded in DOM-free worker_threads; parser regression, not Windows UI validation'}));
}finally{await worker.terminate();}
