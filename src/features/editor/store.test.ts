import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useEditorStore } from './store';
import { HISTORY_LIMIT } from './history';
import { resizeAnnotation } from './model';
const source = { id: 'one', name: '截图 1', widthPx: 800, heightPx: 600, blob: new Blob(['original']) };
beforeEach(() => { vi.restoreAllMocks(); useEditorStore.setState(useEditorStore.getInitialState(), true); vi.spyOn(URL, 'createObjectURL').mockImplementation(() => `blob:${crypto.randomUUID()}`); vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {}); });
describe('independent image editor sessions', () => {
 it('blocks disposal during overlapping file operations and releases each operation exactly once',()=>{const s=useEditorStore.getState();s.open(source);const finishA=s.beginIo(),finishB=s.beginIo();expect(s.close('one',true)).toBe(false);finishA();finishA();expect(useEditorStore.getState().pendingIo).toBe(1);expect(s.close('one',true)).toBe(false);finishB();expect(useEditorStore.getState().pendingIo).toBe(0);expect(s.close('one',true)).toBe(true);});
  it('refuses to close an unsaved tab and retains its editable data and image URL',()=>{
    const s=useEditorStore.getState();s.open(source);s.add({id:'unsaved',type:'rectangle',x:5,y:8,width:20,height:30,color:'#123456',strokeWidth:3});
    const doc=s.serialize('one');expect(s.close('one')).toBe(false);expect(s.serialize('one')).toEqual(doc);expect(URL.revokeObjectURL).not.toHaveBeenCalled();
  });
  it('allows saved tabs to close but requires explicit discard for an unfinished gesture',()=>{
    const s=useEditorStore.getState();s.open(source);s.setTool('rectangle');s.beginDraw({x:10,y:20});s.moveGesture({x:80,y:90});
    expect(s.close('one')).toBe(false);expect(useEditorStore.getState().gesture).not.toBeNull();
    expect(s.close('one',true)).toBe(true);expect(useEditorStore.getState().gesture).toBeNull();expect(useEditorStore.getState().sessions.one).toBeUndefined();expect(URL.revokeObjectURL).toHaveBeenCalledTimes(1);
    s.open(source);s.add({id:'saved',type:'rectangle',x:5,y:8,width:20,height:30,color:'#123456',strokeWidth:3});s.markSaved('one',s.serialize('one'));expect(s.close('one')).toBe(true);
  });
  it('applies persisted defaults only to future objects and new image sessions',()=>{const s=useEditorStore.getState();s.open(source);s.setTool('rectangle');s.beginDraw({x:10,y:10});s.moveGesture({x:40,y:40});s.finishGesture();const original=s.serialize('one');s.setDefaults({color:'#123456',strokeWidth:8});expect(s.serialize('one')).toEqual(original);expect(useEditorStore.getState().sessions.one.history.past).toHaveLength(1);s.open({...source,id:'two'});s.setTool('rectangle');s.beginDraw({x:10,y:10});s.moveGesture({x:40,y:40});s.finishGesture();expect(s.serialize('two').objects[0]).toMatchObject({color:'#123456',strokeWidth:8});expect(s.serialize('one')).toEqual(original);});
  it('tracks saved annotation snapshots while retaining newer changes and undo history',()=>{const s=useEditorStore.getState();s.open(source);const empty=s.serialize('one');s.setTool('rectangle');s.beginDraw({x:10,y:10});s.moveGesture({x:40,y:40});s.finishGesture();s.markSaved('one',empty);expect(JSON.stringify(s.serialize('one'))).not.toBe(useEditorStore.getState().sessions.one.savedDocument);const current=s.serialize('one');s.markSaved('one',current);expect(JSON.stringify(current)).toBe(useEditorStore.getState().sessions.one.savedDocument);expect(useEditorStore.getState().sessions.one.history.past).toHaveLength(1);s.undo();expect(JSON.stringify(s.serialize('one'))).not.toBe(useEditorStore.getState().sessions.one.savedDocument);});
  it('reloaded offset arrow handles commit once and undo/redo preserve both endpoints and layer order',()=>{
    const s=useEditorStore.getState();const arrow={id:'a',type:'arrow' as const,x:50,y:60,points:[10,-20,100,40],color:'#123456',strokeWidth:3};
    const text={id:'t',type:'text' as const,x:0,y:0,text:'科研图注',fontSize:24,color:'#ff0000',strokeWidth:2};
    const doc={schemaVersion:1,image:{widthPx:800,heightPx:600},objects:[arrow,text]};s.open(source,JSON.parse(JSON.stringify(doc)));s.setViewport({x:-40,y:70,scale:2.5,mode:'manual'});
    s.beginMove('a',{x:150,y:100},1);for(let i=1;i<=30;i++)s.moveGesture({x:150+i,y:100+i});s.finishGesture();
    expect(s.serialize('one').objects[0]).toMatchObject({points:[10,-20,130,70]});expect(s.serialize('one').objects[1]).toEqual(text);expect(useEditorStore.getState().sessions.one.history.past).toHaveLength(1);
    s.undo();expect(s.serialize('one')).toEqual(doc);s.redo();expect(s.serialize('one').objects[0]).toMatchObject({points:[10,-20,130,70]});
    s.beginMove('a',{x:180,y:130},1);s.moveGesture({x:9999999,y:130});s.finishGesture();expect(useEditorStore.getState().error).toContain('路径');expect(useEditorStore.getState().sessions.one.history.past).toHaveLength(1);expect(s.serialize('one').objects[0]).toMatchObject({points:[10,-20,130,70]});
  });
  it('records create, many move samples, resize, properties and deletion once each; undo/redo retain pixels and view',()=>{
    const s=useEditorStore.getState();s.open(source);s.setTool('rectangle');s.beginDraw({x:10,y:20});s.moveGesture({x:100,y:90});s.finishGesture();
    const initial=s.serialize('one').objects[0];s.setTool('select');s.setViewport({x:30,y:40,scale:2,mode:'manual'});s.beginMove(initial.id,{x:50,y:50});for(let i=1;i<=30;i++)s.moveGesture({x:50+i,y:50+i});s.finishGesture();
    expect(useEditorStore.getState().sessions.one.history.past).toHaveLength(2);s.update(resizeAnnotation(s.serialize('one').objects[0],40,50,2,3));s.setStyle({color:'#112233',strokeWidth:8});s.removeSelected();
    expect(useEditorStore.getState().sessions.one.history.past).toHaveLength(5);expect(s.serialize('one').objects).toEqual([]);
    s.undo();expect(s.serialize('one').objects[0]).toMatchObject({color:'#112233',width:180,height:210});s.undo();expect(s.serialize('one').objects[0].color).toBe(initial.color);s.undo();s.undo();expect(s.serialize('one').objects[0]).toEqual(initial);s.undo();expect(s.serialize('one').objects).toEqual([]);
    for(let i=0;i<5;i++)s.redo();expect(s.serialize('one').objects).toEqual([]);expect(useEditorStore.getState().sessions.one.viewport).toMatchObject({x:30,y:40,scale:2});expect(useEditorStore.getState().sessions.one.image.blob).toBe(source.blob);
  });
  it('new edit invalidates redo; cancelled/no-op gestures do not record; history is bounded and independent',()=>{
    const s=useEditorStore.getState();s.open(source);const a={id:'r',type:'rectangle' as const,x:0,y:0,width:20,height:30,color:'#123456',strokeWidth:3};s.add(a);s.update(a);expect(useEditorStore.getState().sessions.one.history.past).toHaveLength(1);
    s.setTool('select');s.beginMove('r',{x:0,y:0});s.moveGesture({x:20,y:30});s.undo();expect(s.serialize('one').objects).toEqual([]);s.add({...a,id:'new'});expect(useEditorStore.getState().sessions.one.history.future).toEqual([]);
    for(let i=1;i<150;i++)s.update({...a,id:'new',x:i});expect(useEditorStore.getState().sessions.one.history.past).toHaveLength(HISTORY_LIMIT);
    for(let i=0;i<HISTORY_LIMIT;i++)s.undo();expect(s.serialize('one').objects[0].x).toBe(49);for(let i=0;i<HISTORY_LIMIT;i++)s.redo();expect(s.serialize('one').objects[0].x).toBe(149);
    s.open({...source,id:'two'});s.undo();expect(s.serialize('two').objects).toEqual([]);s.activate('one');expect(s.serialize('one').objects[0].x).toBe(149);
  });
  it('loads editable objects in order without history; invalid version/dimensions preserve current state',()=>{
    const s=useEditorStore.getState();s.open(source);const a={id:'r',type:'rectangle' as const,x:10,y:20,width:20,height:30,color:'#123456',strokeWidth:3};s.add(a);const doc=s.serialize('one');s.open({...source,id:'loaded'},JSON.parse(JSON.stringify(doc)));expect(s.serialize('loaded')).toEqual(doc);expect(useEditorStore.getState().sessions.loaded.history.past).toEqual([]);s.select('r');s.update({...a,x:50});s.undo();expect(s.serialize('loaded')).toEqual(doc);
    s.open({...source,id:'bad'},{...doc,schemaVersion:99});expect(useEditorStore.getState().activeId).toBe('loaded');expect(useEditorStore.getState().error).toContain('版本');s.open({...source,id:'bad'}, {...doc,image:{widthPx:100,heightPx:100}});expect(useEditorStore.getState().activeId).toBe('loaded');expect(s.serialize('loaded')).toEqual(doc);
  });
  it('drawing commits once, zero area and Escape commit nothing, tool switch cancels draft',()=>{
    const s=useEditorStore.getState();s.open(source);s.setTool('rectangle');s.beginDraw({x:100,y:80});s.moveGesture({x:200,y:150});expect(s.serialize('one').objects).toHaveLength(0);s.finishGesture();expect(s.serialize('one').objects).toHaveLength(1);
    s.beginDraw({x:20,y:20});s.finishGesture();expect(s.serialize('one').objects).toHaveLength(1);
    s.beginDraw({x:20,y:20});s.moveGesture({x:100,y:100});s.setTool('ellipse');s.finishGesture();expect(s.serialize('one').objects).toHaveLength(1);
    s.beginDraw({x:20,y:20});s.moveGesture({x:100,y:100});s.cancelGesture();s.finishGesture();expect(s.serialize('one').objects).toHaveLength(1);
  });
  it('moving uses image deltas, cancel restores geometry, per-image objects stay independent',()=>{
    const s=useEditorStore.getState();s.open(source);s.setTool('rectangle');s.beginDraw({x:10,y:20});s.moveGesture({x:110,y:100});s.finishGesture();const a=s.serialize('one').objects[0];s.setTool('select');s.beginMove(a.id,{x:50,y:50});s.moveGesture({x:80,y:90});s.cancelGesture();expect(s.serialize('one').objects[0]).toEqual(a);
    s.beginMove(a.id,{x:50,y:50});s.moveGesture({x:80,y:90});s.finishGesture();expect(s.serialize('one').objects[0]).toMatchObject({x:40,y:60});
    s.open({...source,id:'two'});expect(s.serialize('two').objects).toEqual([]);s.removeSelected();s.activate('one');expect(s.serialize('one').objects).toHaveLength(1);s.removeSelected();expect(s.serialize('one').objects).toEqual([]);
  });
  it('style edits update only selected objects and never the source Blob or viewport',()=>{
    const s=useEditorStore.getState();s.open(source);s.setTool('arrow');s.beginDraw({x:20,y:20});s.moveGesture({x:120,y:120});s.finishGesture();s.setStyle({color:'#112233',strokeWidth:8});expect(s.serialize('one').objects[0]).toMatchObject({color:'#112233',strokeWidth:8});
    s.setViewport({x:30,y:-10,scale:2,mode:'manual'});s.zoom(4,{x:100,y:100});expect(s.serialize('one').objects[0]).toMatchObject({x:20,y:20,points:[0,0,100,100]});expect(useEditorStore.getState().sessions.one.image.blob).toBe(source.blob);
  });
  it('keeps independent viewport and documents, and reuses an existing image session', () => {
    const s = useEditorStore.getState(); s.open(source); s.fit(848,648); s.setViewport({ x:20,y:30,scale:2,mode:'manual' }); s.open({ ...source,id:'two' });
    expect(useEditorStore.getState().sessions.two.viewport.scale).toBe(1); s.activate('one'); expect(useEditorStore.getState().sessions.one.viewport.scale).toBe(2);
    s.open(source); expect(Object.keys(useEditorStore.getState().sessions)).toHaveLength(2); expect(URL.createObjectURL).toHaveBeenCalledTimes(2);
  });
  it('serializes only pixel metadata and annotation data; close releases only that image URL', () => {
    const s = useEditorStore.getState(); s.open(source); const url = useEditorStore.getState().sessions.one.image.url;
    const json = JSON.stringify(s.serialize('one')); expect(json).not.toContain('blob:'); expect(json).not.toContain('viewport'); expect(json).not.toContain('selected');
    s.close('one'); expect(URL.revokeObjectURL).toHaveBeenCalledWith(url); expect(useEditorStore.getState().activeId).toBeNull();
  });
  it('invalid dimensions and image load errors are reported without modifying another image', () => {
    const s=useEditorStore.getState();s.open(source);s.open({...source,id:'bad',widthPx:0}); expect(useEditorStore.getState().error).toContain('尺寸');expect(Object.keys(useEditorStore.getState().sessions)).toEqual(['one']);
    s.imageFailed('one','读取失败'); expect(useEditorStore.getState().sessions.one.loadError).toBe('读取失败');
  });
});

describe('docked image resize isolation',()=>{it('fits an inactive image without switching focus or modifying either annotation history',()=>{const s=useEditorStore.getState();s.open(source);s.open({...source,id:'two'});s.activate('one');const before=useEditorStore.getState().sessions.one;s.fitSession('two',320,200);const next=useEditorStore.getState();expect(next.activeId).toBe('one');expect(next.sessions.one).toBe(before);expect(next.sessions.two.viewport.scale).toBeLessThan(1);expect(next.sessions.two.history).toEqual({past:[],future:[]});s.fitSession('two',NaN,0);expect(useEditorStore.getState().sessions.two).toBe(next.sessions.two);});});
