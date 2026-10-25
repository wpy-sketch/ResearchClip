// Exact installed PDF.js resources, with their original licenses. No scripting runtime.
import {mkdirSync,readdirSync,copyFileSync,readFileSync} from 'node:fs';
import {join} from 'node:path';
const source='node_modules/pdfjs-dist',target='public/pdfjs';
if(JSON.parse(readFileSync(join(source,'package.json'))).version!=='6.4.299')throw Error('Unexpected PDF.js version');
mkdirSync(target,{recursive:true});
// Flat official asset directories. Avoid Node 22's recursive cp Windows fast-fail.
for(const folder of ['cmaps','iccs']){
 mkdirSync(join(target,folder),{recursive:true});
 for(const file of readdirSync(join(source,folder)))copyFileSync(join(source,folder,file),join(target,folder,file));
}
for(const folder of ['wasm','standard_fonts']){
 mkdirSync(join(target,folder),{recursive:true});
 for(const file of readdirSync(join(source,folder))){
  if(folder==='wasm'&&(/quickjs/i.test(file)||file.endsWith('.js')))continue;
  if(folder==='standard_fonts'&&!(/\.pfb$/.test(file)||file==='LICENSE_FOXIT'))continue;
  copyFileSync(join(source,folder,file),join(target,folder,file));
 }
}
copyFileSync(join(source,'LICENSE'),join(target,'LICENSE'));
