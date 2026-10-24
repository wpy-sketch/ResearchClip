import {useEffect,type RefObject} from 'react';
import {scrollRatio} from './projection';
export function useScrollSync(source:RefObject<HTMLElement|null>,preview:RefObject<HTMLElement|null>,enabled:boolean){
 useEffect(()=>{const left=source.current,right=preview.current;if(!enabled||!left||!right)return;const programmed=new WeakMap<HTMLElement,number>();let last=left,frame=0;
  const drive=(from:HTMLElement,to:HTMLElement)=>{const target=programmed.get(from);if(target!==undefined&&Math.abs(from.scrollTop-target)<=1){programmed.delete(from);return;}programmed.delete(from);last=from;const top=scrollRatio(from.scrollTop,from.scrollHeight,from.clientHeight)*Math.max(0,to.scrollHeight-to.clientHeight);if(Math.abs(to.scrollTop-top)>1){programmed.set(to,top);to.scrollTop=top;}};
  const a=()=>drive(left,right),b=()=>drive(right,left),measure=()=>{cancelAnimationFrame(frame);frame=requestAnimationFrame(()=>{programmed.delete(last);drive(last,last===left?right:left);});};
  left.addEventListener('scroll',a,{passive:true});right.addEventListener('scroll',b,{passive:true});right.addEventListener('load',measure,true);const observer=new ResizeObserver(measure);observer.observe(left);observer.observe(right);if(right.firstElementChild)observer.observe(right.firstElementChild);measure();
  return()=>{cancelAnimationFrame(frame);observer.disconnect();left.removeEventListener('scroll',a);right.removeEventListener('scroll',b);right.removeEventListener('load',measure,true);};
 });
}
