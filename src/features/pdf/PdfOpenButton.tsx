import {useState} from 'react';
import {pdfDocuments,type PdfRecord} from './PdfDocumentService';
import {errorMessage} from '../../shared/api/errors';
import {workspaceManager} from '../workspace/manager';
export function PdfOpenButton(){const [recent,setRecent]=useState<PdfRecord[]>([]),[busy,setBusy]=useState(false);
 const open=async(id?:string)=>{setBusy(true);try{await(await import('./PdfViewerSession')).openPdf(id);}catch(e){workspaceManager.state.setState({error:errorMessage(e,'PDF打开失败。')});}finally{setBusy(false);}};
 return <><button className="secondary-button" disabled={busy} onClick={()=>void open()}>打开PDF</button><select aria-label="最近阅读PDF" value="" onFocus={()=>{void pdfDocuments.recent().then(setRecent).catch(e=>workspaceManager.state.setState({error:errorMessage(e,'最近PDF读取失败。')}));}} onChange={e=>{if(e.target.value)void open(e.target.value);}}><option value="">最近PDF</option>{recent.map(r=><option key={r.documentId} value={r.documentId}>{r.title}</option>)}</select></>;
}
