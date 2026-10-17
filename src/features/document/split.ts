// Preference survives narrow windows; only the displayed width is clamped.
export function splitWidth(width:number,percent:number){
  const available=Math.max(0,width-8),minimum=Math.min(280,available*.35);
  return Math.min(available-minimum,Math.max(minimum,available*Math.min(80,Math.max(20,percent))/100));
}
export function splitPercent(width:number,x:number){
  const available=Math.max(1,width-8),left=splitWidth(width,x/available*100);
  return Math.round(Math.min(80,Math.max(20,left/available*100)));
}
