import { useState } from 'react';
import { Pin } from 'lucide-react';
import { recoverPins } from './api';
import { errorMessage } from '../../shared/api/errors';

export function RecoverPinsButton(){
  const [message,setMessage]=useState<string|null>(null),[busy,setBusy]=useState(false);
  return <span className="relative"><button className="icon-button" title="将全部贴图恢复到主显示器可见位置" aria-label="恢复贴图位置" disabled={busy} onClick={()=>{setBusy(true);void recoverPins().then(()=>setMessage(null)).catch(e=>setMessage(errorMessage(e,'恢复贴图失败。'))).finally(()=>setBusy(false));}}><Pin size={16}/></button>{message&&<button className="pin-recover-error" role="alert" onClick={()=>setMessage(null)}>{message}</button>}</span>;
}
