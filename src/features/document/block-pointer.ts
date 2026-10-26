/** Insertion target for local pointer dragging; no resource or storage state. */
export function pointerBlockTarget(rows:{index:number;top:number;bottom:number}[],y:number,total:number){
 if(!Number.isFinite(y)||!Number.isInteger(total)||total<0||rows.some(r=>!Number.isInteger(r.index)||r.index<0||r.index>=total||!Number.isFinite(r.top)||!Number.isFinite(r.bottom)||r.bottom<r.top))throw Error('块拖动位置无效。');
 return rows.find(row=>y<(row.top+row.bottom)/2)?.index??total;
}
