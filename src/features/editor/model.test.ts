import { describe,expect,it } from 'vitest';
import { createAnnotation,drawAnnotation,moveArrowEndpoint,resizeAnnotation,translateAnnotation,validateDocument } from './model';
import { screenToImage } from './geometry';
import type { Annotation } from './types';
const style={color:'#aa3311',strokeWidth:3,fontSize:24};
describe('serializable original-pixel annotation rules',()=>{
  it.each(['rectangle','ellipse'] as const)('reverse %s drawing normalizes original-pixel bounds after pan/zoom',(tool)=>{
    const view={x:-40,y:60,scale:2,mode:'manual' as const};const start=screenToImage({x:360,y:300},view),end=screenToImage({x:160,y:160},view);
    expect(drawAnnotation(createAnnotation(tool,start,style,'shape'),start,end)).toMatchObject({id:'shape',x:100,y:50,width:100,height:70});
  });
  it('move and resize preserve local points, styles, and IDs without changing source annotations',()=>{
    const a=drawAnnotation(createAnnotation('rectangle',{x:20,y:30},style,'rect'),{x:20,y:30},{x:100,y:90});
    const moved=translateAnnotation(a,{x:10,y:-5});const resized=resizeAnnotation(moved,30,25,2,1.5);
    expect(resized).toMatchObject({id:'rect',x:30,y:25,width:160,height:90,color:'#aa3311',strokeWidth:3}); expect(a).toMatchObject({x:20,y:30,width:80,height:60});
  });
  it('arrow endpoints remain absolute original-image positions when the origin moves',()=>{
    const a=drawAnnotation(createAnnotation('arrow',{x:50,y:60},style,'arrow'),{x:50,y:60},{x:150,y:100});
    const changed=moveArrowEndpoint(a,0,{x:20,y:30});expect(changed).toMatchObject({x:20,y:30,points:[0,0,130,70]});
    expect(moveArrowEndpoint(changed,1,{x:180,y:120})).toMatchObject({points:[0,0,160,90]});
  });
  it('free pen points remain vector data and resizing scales local coordinates',()=>{
    let a=createAnnotation('pen',{x:10,y:20},style,'pen');a=drawAnnotation(a,{x:10,y:20},{x:30,y:40});a=drawAnnotation(a,{x:10,y:20},{x:50,y:30});
    expect(resizeAnnotation(a,10,20,2,3)).toMatchObject({points:[0,0,0,0,40,60,80,30]});
  });
  it('editing imported arrow endpoints preserves the opposite endpoint after zoom and pan',()=>{
    const arrow:Annotation={id:'imported',type:'arrow',x:50,y:60,points:[10,-20,100,40],color:style.color,strokeWidth:3};
    const view={x:-40,y:70,scale:2.5,mode:'manual' as const};
    const point=screenToImage({x:410,y:370},view);
    const changed=moveArrowEndpoint(arrow,1,point);
    expect(changed).toMatchObject({x:50,y:60,points:[10,-20,130,60]});
    expect(moveArrowEndpoint(changed,0,{x:20,y:30})).toMatchObject({x:20,y:30,points:[0,0,160,90]});
    expect(arrow.points).toEqual([10,-20,100,40]);
  });
  it('serialization rejects duplicate IDs, non-finite geometry, empty text and future versions',()=>{
    const rectangle=drawAnnotation(createAnnotation('rectangle',{x:1,y:2},style,'same'),{x:1,y:2},{x:40,y:50});
    const doc={schemaVersion:1 as const,image:{widthPx:800,heightPx:600},objects:[rectangle]};validateDocument(JSON.parse(JSON.stringify(doc)));
    expect(()=>validateDocument({...doc,objects:[rectangle,rectangle]})).toThrow('ID');
    expect(()=>validateDocument({...doc,objects:[{...rectangle,x:NaN}]})).toThrow();
    expect(()=>validateDocument({...doc,objects:[createAnnotation('text',{x:10,y:10},style,'text')]})).toThrow();
    expect(()=>validateDocument({...doc,schemaVersion:2} as unknown as typeof doc)).toThrow('版本');
    expect(()=>validateDocument({...doc,objects:[{...rectangle,type:'unknown'} as unknown as Annotation]})).toThrow();
  });
});
