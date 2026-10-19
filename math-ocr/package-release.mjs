import {mkdir,copyFile,readFile,writeFile} from 'node:fs/promises';
import {constants} from 'node:fs';
import {resolve,join} from 'node:path';
const output=resolve(process.argv[2]??'artifacts/math-ocr-setup-v1.0.4');
await mkdir(output,{recursive:false});
const files=['README.md','worker.py','models.json','requirements-lock.txt','install-runtime.ps1','download-models.py','LICENSE-RapidLaTeXOCR','LICENSE-LaTeX-OCR'];
for(const name of files)await copyFile(join('math-ocr',name),join(output,name),constants.COPYFILE_EXCL);
let readme=await readFile(join(output,'README.md'),'utf8');readme=readme.replace('仓库`docs/MATH-OCR.md`与`docs/qa/RELEASE-1.0.4.md`','[模型比较](https://github.com/wpy-sketch/ResearchClip/blob/v1.0.4/docs/MATH-OCR.md)与[实际验收](https://github.com/wpy-sketch/ResearchClip/blob/v1.0.4/docs/qa/RELEASE-1.0.4.md)');await writeFile(join(output,'README.md'),readme);
console.log(JSON.stringify({result:'PASS',output,files:files.length,containsRuntime:false,containsModels:false}));
