import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdir,writeFile} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import {setTimeout as delay} from 'node:timers/promises';
import {chromium} from 'playwright-core';
const root=process.cwd(),id=process.argv[2]??String(Date.now());assert.match(id,/^[a-z0-9-]{1,64}$/);const out=resolve(root,'artifacts',`advanced-renderer-${id}`);await mkdir(out);
let server,browser,page;const report={mode:'real Edge canvas pixel/render tests using explicit fixtures; not native screen capture',checks:[],errors:[]};
try{
 server=spawn(process.execPath,['node_modules/vite/bin/vite.js','--host','127.0.0.1','--port','1427','--strictPort'],{cwd:root,windowsHide:true,stdio:'pipe'});for(let i=0;i<100;i++){if(await fetch('http://127.0.0.1:1427').then(r=>r.ok).catch(()=>false))break;await delay(100);}
 browser=await chromium.launch({channel:'msedge',headless:true});page=await browser.newPage({viewport:{width:1280,height:900}});page.on('pageerror',e=>report.errors.push(e.message));await page.goto('http://127.0.0.1:1427');
 const data=await page.evaluate(async()=>{
  const {renderComposite}=await import('/src/features/editor/export.ts');const {useEditorStore}=await import('/src/features/editor/store.ts');const {useUiStore}=await import('/src/app/ui-store.ts');
  const c=document.createElement('canvas');c.width=320;c.height=200;const ctx=c.getContext('2d'),pixels=ctx.createImageData(320,200);for(let y=0;y<200;y++)for(let x=0;x<320;x++){const i=(y*320+x)*4,v=(Math.floor(x/3)+Math.floor(y/3))%2?255:0;pixels.data.set([v,255-v,100,y>=160?128:255],i);}ctx.putImageData(pixels,0,0);const blob=await new Promise(r=>c.toBlob(r));
  const base={color:'#cc3311',strokeWidth:3};const mosaic={...base,id:'mosaic',type:'mosaic',x:20,y:20,width:100,height:100,strength:12},blur={...base,id:'blur',type:'blur',x:180,y:20,width:100,height:100,strength:8},alpha={...mosaic,id:'alpha',x:20,y:160,width:100,height:40};
  const doc={schemaVersion:1,image:{widthPx:320,heightPx:200},objects:[mosaic,blur,alpha,{...blur,id:'alpha-blur',y:160,height:40}]};
  const decode=async blob=>{const b=await createImageBitmap(blob),p=new OffscreenCanvas(b.width,b.height),ct=p.getContext('2d');ct.drawImage(b,0,0);b.close();return ct.getImageData(0,0,p.width,p.height).data;};
  const original=await decode(blob),png=await renderComposite(blob,doc),output=await decode(png),pixel=(a,x,y)=>Array.from(a.slice((y*320+x)*4,(y*320+x)*4+4));
  const variance=(a,left)=>{let sum=0,sq=0,n=0;for(let y=30;y<110;y++)for(let x=left+10;x<left+90;x++){const v=a[(y*320+x)*4];sum+=v;sq+=v*v;n++;}return sq/n-(sum/n)**2;};
  const metrics={outsideBefore:pixel(original,5,5),outsideAfter:pixel(output,5,5),mosaic1:pixel(output,24,24),mosaic2:pixel(output,25,24),beforeVariance:variance(original,180),blurVariance:variance(output,180),alpha:pixel(output,25,175)[3],originalAlpha:pixel(original,25,175)[3],blurAlpha:pixel(output,195,175)[3]};
  const stronger=await decode(await renderComposite(blob,{...doc,objects:[{...blur,strength:24}]}));metrics.strongerVariance=variance(stronger,180);
  const withNumber=await decode(await renderComposite(blob,{...doc,objects:[{...base,id:'n',type:'number',x:35,y:35,value:42,size:36},mosaic]}));metrics.numberPixel= pixel(withNumber,39,53);metrics.effectPixel=pixel(output,39,53);
  useEditorStore.getState().open({id:'advanced',name:'pixel fixture',widthPx:320,heightPx:200,blob},doc);useUiStore.getState().navigate('editor');window.advanced={store:useEditorStore,blob,doc,output:Array.from(output)};return metrics;
 });
 assert.deepEqual(data.outsideAfter,data.outsideBefore);assert.deepEqual(data.mosaic1,data.mosaic2);assert.ok(data.blurVariance<data.beforeVariance*0.1);assert.ok(data.strongerVariance<=data.blurVariance+2);assert.equal(data.alpha,data.originalAlpha);assert.equal(data.blurAlpha,data.originalAlpha);assert.notDeepEqual(data.numberPixel,data.effectPixel);report.metrics=data;report.checks.push('real PNG pixels show constant mosaic cells and reduced blur variance; stronger blur verified','outside unchanged, transparent alpha preserved, numbered vector remains above later image effects');
 const canvas=page.getByRole('application',{name:'图片编辑画布'});await canvas.waitFor();await delay(250);for(const scale of [0.5,1,2]){
  await page.evaluate(scale=>{const s=window.advanced.store.getState();s.setViewport({x:30,y:30,scale,mode:'manual'});s.select('mosaic');},scale);await delay(100);
  const equal=await page.evaluate(async()=>{const {renderComposite}=await import('/src/features/editor/export.ts');const b=await createImageBitmap(await renderComposite(window.advanced.blob,window.advanced.store.getState().serialize('advanced'))),c=new OffscreenCanvas(b.width,b.height),ctx=c.getContext('2d');ctx.drawImage(b,0,0);b.close();return Array.from(ctx.getImageData(0,0,c.width,c.height).data).every((v,i)=>v===window.advanced.output[i]);});assert.equal(equal,true);
 }
 report.checks.push('0.5x/1x/2x viewport and selected Transformer do not change full-resolution exported RGBA pixels');
 await page.evaluate(()=>{const s=window.advanced.store.getState();s.setViewport({x:30,y:30,scale:1,mode:'manual'});});await delay(100);
 const preview=await canvas.evaluate(host=>{const c=host.querySelector('canvas'),ctx=c.getContext('2d'),d=c.width/c.clientWidth;return Array.from(ctx.getImageData(Math.round(55*d),Math.round(55*d),1,1).data);});assert.deepEqual(preview,data.mosaic1);report.checks.push('actual live base-layer preview contains the same mosaic pixels as PNG export');
 await page.screenshot({path:join(out,'advanced.png')});assert.equal(report.errors.length,0);report.result='PASS';
}catch(e){report.result='FAIL';report.failure=e.stack;process.exitCode=1;if(page)await page.screenshot({path:join(out,'failure.png')}).catch(()=>{});}finally{if(browser)await browser.close();if(server)server.kill();await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));}
