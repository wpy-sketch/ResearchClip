// Real Windows.Media.Ocr on generated, non-user images. No mocked recognition.
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {mkdir,writeFile} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import {chromium} from 'playwright-core';
const root=process.cwd(),id=process.argv[2]??String(Date.now());assert.match(id,/^[a-z0-9-]{1,64}$/);
const output=resolve(root,'artifacts',`ocr-engine-${id}`);await mkdir(output);
const probe=resolve(root,'src-tauri/target/debug/examples/ocr_probe.exe');
const report={mode:'actual Windows native OCR; generated text fixtures, no screen access or mocked engine',languages:execFileSync(probe,[],{encoding:'utf8',windowsHide:true}).trim(),cases:[],checks:[]};
const samples=[
  {id:'chinese',lines:['科研笔记与图片管理','文字识别结果需要人工校对。','','第二段研究结论保存在本地。'],expect:['科研笔记','图片管理','人工校对']},
  {id:'english',lines:['ResearchClip local research notes','Optical character recognition works offline.','','A second paragraph for science and knowledge.'],expect:['ResearchClip','recognition','offline']},
  {id:'mixed',lines:['ResearchClip 科研笔记 2026','中文 English 混合内容','结果保留本地 local only'],expect:['ResearchClip','科研笔记','English']},
  {id:'blank',lines:[],expect:[],blank:true},
  {id:'low-resolution',lines:['ResearchClip low resolution sample','Small text 12345'],size:10,width:280,height:70,expect:[]},
  {id:'symbols',lines:['Alpha 123: + - = % @ #','中文标点：，。！？（测试）','Greek symbols: α β ± ∑ √ ∞'],expect:['Alpha','123']},
];
let browser;
try{
  browser=await chromium.launch({channel:'msedge',headless:true});const page=await browser.newPage();
  for(const sample of samples){
    const bytes=await page.evaluate(async s=>{await document.fonts.ready;const canvas=document.createElement('canvas');canvas.width=s.width??1100;canvas.height=s.height??500;const c=canvas.getContext('2d');c.fillStyle='white';c.fillRect(0,0,canvas.width,canvas.height);c.fillStyle='black';const size=s.size??34;c.font=`${size}px "Microsoft YaHei", "Segoe UI", sans-serif`;s.lines.forEach((line,i)=>c.fillText(line,Math.max(4,size),size*(2+i*1.7)));return Array.from(new Uint8Array(await (await new Promise(r=>canvas.toBlob(r,'image/png'))).arrayBuffer()));},sample);
    const path=join(output,`${sample.id}.png`);await writeFile(path,Buffer.from(bytes));
    const t=Date.now(),actual=JSON.parse(execFileSync(probe,[path,'zh-Hans-CN'],{encoding:'utf8',windowsHide:true,timeout:60000}));
    const compact=actual.text.replace(/\s/g,'').toLowerCase();const matches=sample.expect.map(term=>({term,found:compact.includes(term.replace(/\s/g,'').toLowerCase())}));
    report.cases.push({id:sample.id,status:'PASS',milliseconds:Date.now()-t,...actual,matches,scope:sample.id==='low-resolution'?'bounded successful execution; no accuracy guarantee':sample.id==='symbols'?'ASCII anchor recognition; Unicode symbols evaluated below, not guaranteed':'required anchors'});
    assert.ok(matches.every(m=>m.found),`${sample.id}: expected anchors not recognized`);if(sample.blank)assert.equal(actual.text,'');
  }
  // Repeat actual calls with different images to detect stale results / apartment lifecycle errors.
  for(const id of ['chinese','english','mixed','blank','chinese']){
    const again=JSON.parse(execFileSync(probe,[join(output,`${id}.png`),'zh-Hans-CN'],{encoding:'utf8',windowsHide:true,timeout:60000}));
    assert.equal(again.text,report.cases.find(c=>c.id===id).text);
  }
  report.checks.push('six real image categories executed; anchor assertions and empty-image output verified','five consecutive image changes give identical corresponding results, including empty and Chinese restart');
  const symbols=report.cases.find(c=>c.id==='symbols').text;report.symbolAccuracy=Object.fromEntries(['α','β','±','∑','√','∞'].map(s=>[s,symbols.includes(s)]));
  report.result='PASS';
}catch(e){report.result='FAIL';report.failure=e.stack;process.exitCode=1;}
finally{if(browser)await browser.close();await writeFile(join(output,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));}
