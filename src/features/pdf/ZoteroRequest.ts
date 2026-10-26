/** Per-panel ownership, not a resource ID or a second task manager. */
export class ZoteroRequest {
 private pending:{id:string;cancelled:boolean}|null=null;
 begin(id:string){this.pending={id,cancelled:false};}
 get id(){return this.pending?.id??null;}
 owns(id:string){return this.pending?.id===id;}
 accepts(id:string){return this.owns(id)&&!this.pending?.cancelled;}
 cancel(){if(this.pending)this.pending.cancelled=true;return this.id;}
 finish(id:string){if(!this.owns(id))return false;this.pending=null;return true;}
 clear(){const id=this.id;this.pending=null;return id;}
}
