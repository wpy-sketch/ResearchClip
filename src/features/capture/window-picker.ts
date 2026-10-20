export interface PhysicalBounds { x:number; y:number; width:number; height:number }
export interface WindowPickerInfo { sessionId:string; desktop:PhysicalBounds; candidateCount:number; selected:null|{title:string;bounds:PhysicalBounds;dpi:number;blocked:boolean} }
export function pickerRect(bounds:PhysicalBounds,desktop:PhysicalBounds,width:number,height:number) {
  if (![bounds.x,bounds.y,bounds.width,bounds.height,desktop.x,desktop.y,desktop.width,desktop.height,width,height].every(Number.isFinite) || desktop.width<=0 || desktop.height<=0 || width<=0 || height<=0) throw new Error('选择层坐标无效。');
  // One transform for the complete virtual desktop. Never multiply by devicePixelRatio again.
  const sx=width/desktop.width,sy=height/desktop.height;
  return {left:(bounds.x-desktop.x)*sx,top:(bounds.y-desktop.y)*sy,width:bounds.width*sx,height:bounds.height*sy};
}
