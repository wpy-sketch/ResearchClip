import {linkCall} from '../links/api';
import {validRegion,type AnnotationAnchor,type AnchorInput,type AnchorPage} from './model';
const id=(v:unknown):v is string=>typeof v==='string'&&/^[a-f0-9]{32}$/.test(v);
export function parseAnchor(value:unknown):AnnotationAnchor{
 if(!value||typeof value!=='object')throw Error('锚点响应无效。');const a=value as AnnotationAnchor;
 if(!id(a.anchorId)||!id(a.screenshotId)||!/^[a-f0-9]{64}$/.test(a.imageRevision)||!Number.isSafeInteger(a.revision)||a.revision<1||typeof a.title!=='string'||!a.title.trim()||Array.from(a.title).length>200||typeof a.description!=='string'||Array.from(a.description).length>4000||!validRegion(a,16777217,16777217))throw Error('锚点响应无效。');return a;
}
export async function anchorPage(id:string){const p=await linkCall<AnchorPage>('annotation_anchors',{id});if(!p||!/^[a-f0-9]{64}$/.test(p.imageRevision)||!Number.isInteger(p.widthPx)||!Number.isInteger(p.heightPx)||p.widthPx<=0||p.heightPx<=0||p.widthPx*p.heightPx>16777216||!Array.isArray(p.anchors)||p.anchors.length>200)throw Error('锚点列表无效。');p.anchors=p.anchors.map(parseAnchor);if(new Set(p.anchors.map(a=>a.anchorId)).size!==p.anchors.length||p.anchors.some(a=>a.screenshotId!==id||!validRegion(a,p.widthPx,p.heightPx)))throw Error('锚点列表位置无效。');return p;}
export const resolveAnchor=async(screenshotId:string,anchorId:string)=>{const a=parseAnchor(await linkCall('resolve_annotation_anchor',{screenshotId,anchorId}));if(a.screenshotId!==screenshotId||a.anchorId!==anchorId)throw Error('锚点响应目标不匹配。');return a;};
export const saveAnchor=async(input:AnchorInput)=>{const a=parseAnchor(await linkCall('save_annotation_anchor',{input}));if(a.screenshotId!==input.screenshotId||(input.anchorId&&a.anchorId!==input.anchorId)||a.imageRevision!==input.imageRevision)throw Error('保存响应与当前锚点不一致，请重新加载核对。');return a;};
export const deleteAnchor=(a:AnnotationAnchor)=>linkCall<void>('delete_annotation_anchor',{screenshotId:a.screenshotId,anchorId:a.anchorId,revision:a.revision});
