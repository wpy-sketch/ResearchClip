/** WebView2 may change devicePixelRatio without a resolution media-query event. */
export function observePdfDensity(changed:()=>void,read:()=>number=()=>window.devicePixelRatio||1){let density=read();const timer=setInterval(()=>{const next=read();if(next!==density){density=next;changed();}},250);return()=>clearInterval(timer);}
