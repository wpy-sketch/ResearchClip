import matlab from 'highlight.js/lib/languages/matlab';
import latex from 'highlight.js/lib/languages/latex';
import {common,createLowlight} from 'lowlight';
const highlighter=createLowlight({...common,matlab,latex});
export const codeLanguages=['','plaintext','python','javascript','typescript','json','r','rust','cpp','c','csharp','java','bash','sql','matlab','latex','yaml','xml','css','markdown'] as const;
export function highlightCode(code:string,language:string){if(!language||language==='plaintext')return {nodes:null,notice:null};if(code.length>32768)return {nodes:null,notice:'代码超过32 Ki字符，高亮回退纯文本，原文保留。'};if(!highlighter.registered(language))return {nodes:null,notice:`暂不支持${language}高亮，保留纯文本。`};try{return {nodes:highlighter.highlight(language,code).children,notice:null};}catch{return {nodes:null,notice:'高亮失败，保留纯文本。'};}}
