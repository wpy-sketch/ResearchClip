import { describe, expect, it } from 'vitest';
import { packImage, parseDocument, parseProject, unpackProject } from './serialization';
import { shapeAttributes } from './export';
import { boundHistory, HISTORY_BYTES } from './history';
const doc={schemaVersion:1 as const,image:{widthPx:100,heightPx:80},objects:[{id:'a',type:'arrow' as const,x:10,y:20,points:[0,0,40,20],color:'#123456',strokeWidth:4},{id:'t',type:'text' as const,x:5,y:6,text:'研究\nAlpha',fontSize:24,color:'#ff0000',strokeWidth:3}]};
describe('versioned project format and export shapes',()=>{
  it('roundtrips object order, text and arrow properties; serializes no view/selection/history',async()=>{
    const clone=parseDocument(JSON.parse(JSON.stringify({...doc,viewport:{scale:3},history:{past:[]}})));expect(clone).toEqual(doc);clone.objects[0].x=90;expect(doc.objects[0].x).toBe(10);
    const packet=await packImage(new Blob(['png']),doc);const n=new DataView(packet).getUint32(0,true);expect(JSON.parse(new TextDecoder().decode(new Uint8Array(packet,4,n)))).toEqual({document:doc});expect(new TextDecoder().decode(packet.slice(4+n))).toBe('png');
    const project={format:'ResearchClip',schemaVersion:1,originalImage:'original-123.png',document:doc};const json=new TextEncoder().encode(JSON.stringify(project)),buf=new ArrayBuffer(4+json.length+3);new DataView(buf).setUint32(0,json.length,true);new Uint8Array(buf,4,json.length).set(json);new Uint8Array(buf,4+json.length).set([1,2,3]);const result=unpackProject(buf)!;expect(result.project.document).toEqual(doc);expect(result.blob.size).toBe(3);expect(unpackProject(new ArrayBuffer(0))).toBeNull();
  });
  it('rejects future versions, malformed values, duplicate IDs, escapes and oversized packets',()=>{
    for(const value of [null,[],{}, {...doc,schemaVersion:2},{...doc,objects:[{type:'text'}]},{...doc,objects:[doc.objects[0],doc.objects[0]]},{...doc,objects:[{...doc.objects[0],points:[0,0,Infinity,5]}]}])expect(()=>parseDocument(value)).toThrow();
    for(const name of ['../x.png','C:\\x.png','original-../a.png'])expect(()=>parseProject({format:'ResearchClip',schemaVersion:1,originalImage:name,document:doc})).toThrow();
    expect(()=>unpackProject(new Uint8Array([255,255,255,255]).buffer)).toThrow();expect(()=>parseDocument({...doc,objects:[{...doc.objects[1],text:32}]})).toThrow();
  });
  it('export attributes retain raw pixels, no handles, exact arrow head and multiline font',()=>{
    expect(shapeAttributes(doc.objects[0])).toMatchObject({points:[0,0,40,20],strokeWidth:4,pointerLength:12,pointerWidth:12});expect(shapeAttributes(doc.objects[1])).toMatchObject({text:'研究\nAlpha',fontSize:24,fontFamily:'Arial, Microsoft YaHei'});expect(JSON.stringify(doc)).not.toMatch(/viewport|Transformer|selected/);
  });
  it('bounds history by bytes as well as operation count',()=>{
    const large=Array.from({length:200},(_,i)=>({...doc.objects[1],id:`t${i}`,text:'A'.repeat(2000)}));const history=boundHistory({past:Array(40).fill(large),future:[]});const bytes=history.past.reduce((n,p)=>n+new TextEncoder().encode(JSON.stringify(p)).length,0);expect(bytes).toBeLessThanOrEqual(HISTORY_BYTES);expect(history.past.length).toBeLessThan(40);
  });
});
