import {ScanText} from 'lucide-react';
import {useOcrStore} from './store';
export function OcrButton({image,name,disabled=false}:{image:()=>Promise<Blob>;name:string;disabled?:boolean}){
  const phase=useOcrStore(s=>s.phase),open=useOcrStore(s=>Boolean(s.source));
  return <button className="secondary-button" disabled={disabled||phase!=='idle'||open} onClick={()=>void useOcrStore.getState().open(image,name)}><ScanText size={15}/>识别文字</button>;
}
