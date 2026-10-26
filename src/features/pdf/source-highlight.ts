/** One timer per PDF session; stale timers cannot clear a newer navigation. */
export class SourceHighlight {
 private timer:ReturnType<typeof setTimeout>|null=null;
 show(clear:()=>void){this.dispose();this.timer=setTimeout(()=>{this.timer=null;clear();},8000);}
 dispose(){if(this.timer!==null){clearTimeout(this.timer);this.timer=null;}}
}
