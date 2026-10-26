// Fictional protocol fixtures only. Never connected to a real Zotero library.
export function annotationFixture(library,start,mode){
 const item=(key,type,pageIndex,rects,color='#ffd400',text='专用练习批注')=>({key,version:1,library,data:{key,itemType:'annotation',parentItem:'ATTACH01',annotationType:type,annotationColor:color,annotationText:text,annotationComment:'只读练习；未核实文献',annotationPageLabel:String(pageIndex+1),annotationPosition:JSON.stringify({pageIndex,rects})}});
 const list=[
  item('HL000001','highlight',0,[[100,550,300,650]],'#ffd400','矩阵高亮（虚构）'),
  item('UN000001','underline',0,[[100,500,300,525]],'#ff6666','旋转下划线'),
  item('NT000001','note',0,[[320,600,350,630]],'#5fb236','文字便签'),
  item('TX000001','text',1,[[100,600,250,640]],'#2ea8e5','文本批注 <script>绝不执行</script>'),
  item('RG000001','image',1,[[100,550,300,650]],'#a28ae5','区域批注'),
  item('CR000001','highlight',0,[[30,10,200,30]],'#ffd400','跨页延续'),
  item('IK000001','ink',0,[], '#ff6666','Zotero ink，不是私有手写'),
  item('BD000001','highlight',0,[], '#ffd400','畸形位置，明确保留诊断'),
  item('CB000001','image',2,[[90,725,190,775]],'#a28ae5','裁切与物理单位校准'),
  ...Array.from({length:11},(_,n)=>item('A'+String(n).padStart(7,'0'),'highlight',0,[[30+n*3,700,32+n*3,710]],'#ffd400','分页练习 '+n))
 ];
 const cross=JSON.parse(list[5].data.annotationPosition);cross.nextPageRects=[[30,770,200,790]];list[5].data.annotationPosition=JSON.stringify(cross);
 list[6].data.annotationPosition=JSON.stringify({pageIndex:0,paths:[[10,20,30,40]],width:1});
 list[7].data.annotationPosition='{broken';
 if(start===20){const newer=structuredClone(list[0]);newer.version=2;newer.data.annotationText+=' · v2';const batch=[newer,item('EX000001','note',1,[[40,50,70,80]],'#5fb236','第二页响应')];if(library.type==='group')for(const a of batch)a.data.annotationComment='群组库7独立批注';return batch;}
 if(mode==='annotationWrongLibrary')list[0].library={type:library.type,id:99};
 if(library.type==='group')for(const a of list)a.data.annotationComment='群组库7独立批注';
 return list;
}
