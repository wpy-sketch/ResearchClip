import {mkdir,cp,copyFile,readdir,readFile,writeFile} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {join,resolve,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
const source=dirname(fileURLToPath(import.meta.url)),root=dirname(source),output=resolve(process.argv[2]??'artifacts/extension-v1.0.2');
if(!output.startsWith(resolve(root,'artifacts')+'/')&&!output.startsWith(resolve(root,'artifacts')+'\\'))throw Error('Package output must be a new workspace artifact directory');
await mkdir(output);await cp(join(source,'dist'),join(output,'dist'),{recursive:true,errorOnExist:true,force:false});await mkdir(join(output,'bin'));await copyFile(join(source,'target/release/researchclip-native-host.exe'),join(output,'bin/researchclip-native-host.exe'));await mkdir(join(output,'protocol'));await copyFile(join(source,'protocol/extension-id.txt'),join(output,'protocol/extension-id.txt'));
for(const name of ['PROTOCOL.md','register-host.ps1'])await copyFile(join(source,name),join(output,name));
await writeFile(join(output,'README.md'),(await readFile(join(source,'README.md'),'utf8')).replace('(../docs/qa/RELEASE-1.0.2.md)','(https://github.com/wpy-sketch/ResearchClip/blob/v1.0.2/docs/qa/RELEASE-1.0.2.md)'));
const licenses=join(output,'licenses');await mkdir(licenses);const runtime=['@mozilla/readability','dompurify','turndown','@mixmark-io/domino','vite'];const notices=[];
for(const name of runtime){const folder=join(source,'node_modules',name),p=JSON.parse(await readFile(join(folder,'package.json'),'utf8'));notices.push(`${name} ${p.version} — ${p.license}\n`);for(const f of await readdir(folder))if(/^(license|notice)/i.test(f))await copyFile(join(folder,f),join(licenses,`${name.replaceAll('/','-')}-${f}`));}
const metadata=JSON.parse(execFileSync(process.execPath,[join(root,'scripts/with-rust.mjs'),'cargo','metadata','--manifest-path',join(source,'Cargo.toml'),'--locked','--format-version','1'],{cwd:root,encoding:'utf8',windowsHide:true,maxBuffer:8*1024*1024}));
for(const p of metadata.packages.filter(p=>p.source)){notices.push(`${p.name} ${p.version} — ${p.license??'see bundled license'} — ${p.repository??''}\n`);const folder=dirname(p.manifest_path);for(const f of await readdir(folder))if(/^(license|notice|copying)([.-]|$)/i.test(f)){try{await copyFile(join(folder,f),join(licenses,`${p.name}-${p.version}-${f}`));}catch(e){if(e.code!=='EISDIR')throw e;}}}
await writeFile(join(output,'THIRD-PARTY-NOTICES.txt'),notices.join(''));console.log(JSON.stringify({result:'PASS',output,licenseEntries:notices.length}));
