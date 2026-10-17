import {create} from 'zustand';
import {parsePngSize} from '../../shared/api/capture';
import {errorMessage} from '../../shared/api/errors';
import {ocrStatus,recognizeOcr,copyOcrText} from './api';
import {validateText,type OcrStatus,type OcrRegion} from './model';
interface State {
  source:{blob:Blob;url:string;widthPx:number;heightPx:number;name:string}|null;
  phase:'idle'|'preparing'|'recognizing'|'copying';status:OcrStatus|null;language:string;region:OcrRegion|null;
  text:string;recognizedText:string;error:string|null;notice:string|null;
  open:(image:()=>Promise<Blob>,name:string)=>Promise<void>;close:()=>void;
  setLanguage:(language:string)=>void;setRegion:(region:OcrRegion|null)=>void;edit:(text:string)=>void;
  run:()=>Promise<void>;copy:()=>Promise<void>;
}
export function createOcrStore(){return create<State>((set,get)=>({
  source:null,phase:'idle',status:null,language:'',region:null,text:'',recognizedText:'',error:null,notice:null,
  open:async(image,name)=>{
    if(get().phase!=='idle'||get().source)return;
    set({phase:'preparing',error:null,notice:null,status:null,text:'',recognizedText:'',region:null});
    let url:string|null=null;
    try{const blob=await image(),size=parsePngSize(await blob.arrayBuffer());url=URL.createObjectURL(blob);
      set({source:{blob,url,name,...size}});url=null;
      const status=await ocrStatus(),previous=get().language,language=status.languages.some(l=>l.tag===previous)?previous:status.languages.find(l=>l.tag.startsWith('zh'))?.tag??status.languages[0]?.tag??'';
      set({status,language,error:status.languages.length?null:'未发现 OCR 语言包。请在 Windows 设置 → 时间和语言 → 语言选项中安装文字识别组件，再关闭并重新打开此面板。'});
    }catch(error){if(url)URL.revokeObjectURL(url);set({error:errorMessage(error,'无法准备 OCR 图片，请重试。')});}
    finally{set({phase:'idle'});}
  },
  close:()=>{const s=get();if(s.phase!=='idle')return;if(s.source)URL.revokeObjectURL(s.source.url);set({source:null,status:null,region:null,text:'',recognizedText:'',error:null,notice:null});},
  setLanguage:language=>{if(get().phase==='idle'&&get().status?.languages.some(l=>l.tag===language))set({language});},
  setRegion:region=>{if(get().phase==='idle')set({region,notice:null});},
  edit:text=>{if(get().phase!=='idle')return;try{validateText(text);set({text,error:null,notice:null});}catch(error){set({error:errorMessage(error,'文字无效。')});}},
  run:async()=>{const s=get();if(s.phase!=='idle'||!s.source||!s.language||!s.status)return;set({phase:'recognizing',error:null,notice:null});
    try{const result=await recognizeOcr(s.source.blob,s.language,s.region);set({text:result.text,recognizedText:result.text,notice:result.text.trim()?`识别完成 · ${result.widthPx} × ${result.heightPx} 像素。请校对后复制或插入。`:'未识别到文字。可框选更清晰的文字区域或更换识别语言后重试。'});}
    catch(error){set({error:errorMessage(error,'文字识别失败，请重试。')});}finally{set({phase:'idle'});}
  },
  copy:async()=>{const s=get();if(s.phase!=='idle'||!s.text.trim())return;set({phase:'copying',error:null,notice:null});try{await copyOcrText(s.text);set({notice:'识别文字已复制，可粘贴到其他应用。'});}catch(error){set({error:errorMessage(error,'复制文字失败。')});}finally{set({phase:'idle'});}},
}));}
export const useOcrStore=createOcrStore();
