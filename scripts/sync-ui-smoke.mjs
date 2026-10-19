import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdir,writeFile,access,copyFile,constants} from 'node:fs/promises';
import {join} from 'node:path';
import {setTimeout as delay} from 'node:timers/promises';
import {chromium} from 'playwright-core';
const root=process.cwd(),id=process.argv[2]??Date.now().toString();assert.match(id,/^[a-z0-9-]{1,60}$/);
const out=join(root,'artifacts',`sync-ui-${id}`);await mkdir(out);const report={mode:'actual headless Edge rendering of browser preview; no native IPC/credentials/server claims',checks:[],errors:[]};let server,browser,page;
async function until(fn){for(let i=0;i<120;i++){if(await fn())return;await delay(100);}throw Error('startup timeout');}
try{
 server=spawn(process.execPath,['node_modules/vite/bin/vite.js','--port','1431','--strictPort'],{windowsHide:true,stdio:'pipe'});await until(()=>fetch('http://127.0.0.1:1431').then(r=>r.ok).catch(()=>false));
 browser=await chromium.launch({channel:'msedge',headless:true});page=await browser.newPage({viewport:{width:1440,height:1050}});page.on('pageerror',e=>report.errors.push(e.message));await page.goto('http://127.0.0.1:1431');await page.getByRole('button',{name:'设置',exact:true}).click();
 await page.getByRole('navigation',{name:'设置分类'}).getByRole('button',{name:'跨设备同步',exact:false}).click();assert.equal(await page.evaluate(()=>document.activeElement?.id),'webdav-title');report.checks.push('settings task navigation scrolls/focuses actual section');
 const section=page.locator('.sync-settings');assert.ok(await section.getByText('浏览器仅预览布局', {exact:false}).isVisible());assert.ok(await page.getByRole('textbox',{name:'WebDAV服务器',exact:true}).isDisabled());assert.ok(await page.getByRole('button',{name:'手动双向同步 / 重试',exact:true}).isDisabled());report.checks.push('browser preview cannot submit credentials or start synchronization');
 for(const theme of ['dark','light']){
  if(await page.evaluate(()=>document.documentElement.dataset.theme)!==theme)await page.getByRole('button',{name:theme==='light'?'切换为浅色主题':'切换为深色主题',exact:true}).click();
  await section.scrollIntoViewIfNeeded();await section.screenshot({path:join(out,`${theme}-settings.png`)});
  for(const width of [1440,1180,900,760,600,480]){await page.setViewportSize({width,height:1000});await delay(100);assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));const box=await section.boundingBox();assert.ok(box.width>0&&box.x>=0&&box.x+box.width<=width+1);}
  report.checks.push(`${theme}: WebDAV form, actions and task navigation contained at 6 widths 480–1440`);await page.setViewportSize({width:1440,height:1050});
 }
 // Add the genuine preview screenshot to the guide once; never replace an existing user file.
 const figure=join(root,'docs/images/v300-sync-settings.png');try{await access(figure);if(process.argv.includes('--update-guide'))await copyFile(join(out,'light-settings.png'),figure);}catch{await copyFile(join(out,'light-settings.png'),figure,constants.COPYFILE_EXCL);}
 await page.getByRole('button',{name:'新建空白 Markdown',exact:true}).click();const editor=page.getByRole('textbox',{name:'Markdown 源文',exact:true});await editor.fill('# 同步前保留草稿\n\n$x^2$');await page.getByRole('button',{name:'截图素材',exact:true}).click();await page.getByRole('button',{name:'设置',exact:true}).click();assert.equal(await editor.inputValue(),'# 同步前保留草稿\n\n$x^2$');report.checks.push('left navigation/settings remount leaves independent Markdown draft untouched');
 assert.deepEqual(report.errors,[]);report.result='PASS';
}catch(e){report.result='FAIL';report.failure=e.stack;process.exitCode=1;}finally{if(browser)await browser.close();if(server)server.kill();await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));}
