import {beforeEach,expect,it,vi} from 'vitest';
import {createAnnotation,drawAnnotation,resizeAnnotation,translateAnnotation,effectBounds,validateDocument} from './model';
import {screenToImage} from './geometry';
import {parseDocument} from './serialization';
import {useEditorStore} from './store';
const style={color:'#123456',strokeWidth:3,fontSize:24};
beforeEach(()=>{useEditorStore.setState(useEditorStore.getInitialState(),true);vi.spyOn(URL,'createObjectURL').mockReturnValue('blob:advanced');vi.spyOn(URL,'revokeObjectURL').mockImplementation(()=>{});});
it.each(['mosaic','blur'] as const)('%s keeps original pixels through zoom, reverse draw, move, resize and JSON reload',tool=>{
 for(const scale of [0.25,0.5,1,2,4]){
  const view={x:-30,y:50,scale,mode:'manual' as const},start=screenToImage({x:-30+160*scale,y:50+120*scale},view),end=screenToImage({x:-30+20*scale,y:50+30*scale},view);
  const a=drawAnnotation(createAnnotation(tool,start,style,'effect'),start,end),b=resizeAnnotation(translateAnnotation(a,{x:10,y:5}),30,35,0.5,2);
  expect(b).toMatchObject({type:tool,x:30,y:35,width:70,height:180,strength:tool==='mosaic'?12:8});
  expect(parseDocument(JSON.parse(JSON.stringify({schemaVersion:1,image:{widthPx:800,heightPx:600},objects:[b]}))).objects[0]).toEqual(b);
 }
});
it('clips negative/outside/fractional effect regions and rejects strengths, numbers and excessive pixel budgets',()=>{
 expect(effectBounds({x:-2.5,y:3.2,width:10,height:12.1},{widthPx:8,heightPx:10})).toEqual({x:0,y:3,width:8,height:7});
 expect(effectBounds({x:20,y:20,width:5,height:5},{widthPx:8,heightPx:10})).toBeNull();
 const base={id:'a',x:0,y:0,color:'#ffffff',strokeWidth:2},doc={schemaVersion:1 as const,image:{widthPx:4096,heightPx:4096},objects:[{...base,type:'mosaic' as const,width:4096,height:4096,strength:12}]};
 validateDocument(doc);expect(()=>parseDocument({...doc,objects:[doc.objects[0],{...doc.objects[0],id:'b'}]})).toThrow('滤镜');
 for(const strength of [0,1,65,NaN,2.5])expect(()=>parseDocument({...doc,objects:[{...doc.objects[0],strength}]})).toThrow();
 for(const value of [0,-1,1.5,10000])expect(()=>parseDocument({...doc,objects:[{...base,type:'number',value,size:36}]})).toThrow();
});
it('records one continuous effect drag, property edits and resize; undo redo and save/reopen retain all objects',()=>{
 const s=useEditorStore.getState();s.open({id:'image',name:'test',widthPx:800,heightPx:600,blob:new Blob(['controlled'])});
 s.setTool('mosaic');s.beginDraw({x:20,y:30});for(let i=1;i<=30;i++)s.moveGesture({x:20+i*3,y:30+i*2});s.finishGesture();
 expect(useEditorStore.getState().sessions.image.history.past).toHaveLength(1);
 const id=s.serialize('image').objects[0].id;s.select(id);s.setStyle({mosaicSize:24});let a=s.serialize('image').objects[0];expect(a).toMatchObject({strength:24});
 s.beginMove(id,{x:20,y:30});s.moveGesture({x:40,y:60});s.finishGesture();a=s.serialize('image').objects[0];s.update(resizeAnnotation(a,a.x,a.y,2,0.5));
 const saved=s.serialize('image');s.undo();s.redo();expect(s.serialize('image')).toEqual(saved);s.markSaved('image',saved);expect(s.close('image')).toBe(true);s.open({id:'image',name:'restored',widthPx:800,heightPx:600,blob:new Blob(['same'])},JSON.parse(JSON.stringify(saved)));expect(s.serialize('image')).toEqual(saved);
 s.setTool('blur');s.beginDraw({x:10,y:10});s.moveGesture({x:60,y:60});s.cancelGesture();expect(s.serialize('image')).toEqual(saved);
});
it('numbers increment independently, deletion never renumbers others and number/size/color are undoable',()=>{
 const s=useEditorStore.getState();s.open({id:'image',name:'test',widthPx:800,heightPx:600,blob:new Blob(['controlled'])});s.setTool('number');
 for(const x of [50,150,250])s.beginDraw({x,y:100});let numbers=s.serialize('image').objects;expect(numbers.map(a=>a.type==='number'?a.value:null)).toEqual([1,2,3]);expect(numbers[0]).toMatchObject({x:32,y:82,size:36});
 s.select(numbers[1].id);s.removeSelected();expect(s.serialize('image').objects.map(a=>a.type==='number'?a.value:null)).toEqual([1,3]);s.beginDraw({x:350,y:100});numbers=s.serialize('image').objects;expect(numbers.at(-1)).toMatchObject({value:4});
 const last=numbers.at(-1)!;if(last.type!=='number')throw Error('number');s.select(last.id);s.setStyle({numberSize:60,color:'#abcdef'});s.update({...s.serialize('image').objects.at(-1)!,type:'number',size:60,value:42});
 expect(s.serialize('image').objects.at(-1)).toMatchObject({value:42,size:60,color:'#abcdef'});s.undo();expect(s.serialize('image').objects.at(-1)).toMatchObject({value:4});s.redo();expect(s.serialize('image').objects.at(-1)).toMatchObject({value:42});
});
