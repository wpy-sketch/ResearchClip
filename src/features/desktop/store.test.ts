import {useBatchStore} from '../projects/batch';
import {beforeEach,describe,it,expect,vi} from 'vitest';
import {useDesktopStore} from './store';
const mock=vi.hoisted(()=>({getDesktopStatus:vi.fn(),desktopAction:vi.fn(),completeDesktopExit:vi.fn(),flushAll:vi.fn(),editor:{gesture:null as unknown,sessions:{} as Record<string,{document:unknown;savedDocument:string}>},importing:{busy:false},exporting:{busy:false},settings:{phase:'idle'},documentDirty:false,document:{phase:'idle',save:vi.fn()}}));
vi.mock('../../shared/api/desktop',()=>mock);
vi.mock('../notes/store',()=>({useCardStore:{getState:()=>({flushAll:mock.flushAll})}}));
vi.mock('../editor/store',()=>({useEditorStore:{getState:()=>mock.editor}}));
vi.mock('../import/store',()=>({useImportStore:{getState:()=>mock.importing}}));
vi.mock('../export/store',()=>({useMarkdownExportStore:{getState:()=>mock.exporting}}));
vi.mock('../document/store',()=>({allDocumentStores:()=>[{getState:()=>mock.document}],useDocumentStore:{getState:()=>mock.document},documentDirty:()=>mock.documentDirty}));
vi.mock('../workspace/manager',()=>({workspaceManager:{focusDocument:vi.fn()}}));
vi.mock('../settings/store',()=>({useSettingsStore:{getState:()=>mock.settings}}));
beforeEach(()=>{useBatchStore.setState({busy:false});vi.clearAllMocks();mock.flushAll.mockResolvedValue(true);mock.completeDesktopExit.mockResolvedValue(undefined);mock.editor={gesture:null,sessions:{}};mock.importing.busy=false;mock.exporting.busy=false;mock.settings.phase='idle';mock.documentDirty=false;mock.document.phase='idle';mock.document.save.mockResolvedValue(false);useDesktopStore.setState(useDesktopStore.getInitialState(),true);});
describe('graceful explicit exit',()=>{
  it('refuses exit while clipboard/file imports are pending before the native permit exists',async()=>{mock.importing.busy=true;await useDesktopStore.getState().requestQuit();expect(mock.completeDesktopExit).toHaveBeenCalledWith(false);expect(mock.flushAll).not.toHaveBeenCalled();expect(useDesktopStore.getState().error).toContain('导入图片');});
  it('waits for real note saving before completing exit, ignoring duplicate requests',async()=>{let finish:(v:boolean)=>void=()=>{};mock.flushAll.mockImplementation(()=>new Promise(resolve=>{finish=resolve;}));const pending=useDesktopStore.getState().requestQuit();await useDesktopStore.getState().requestQuit();expect(mock.flushAll).toHaveBeenCalledOnce();expect(mock.completeDesktopExit).not.toHaveBeenCalled();finish(true);await pending;expect(mock.completeDesktopExit).toHaveBeenCalledExactlyOnceWith(true);});
  it('refuses exit after save failure without changing editor or note drafts',async()=>{mock.flushAll.mockResolvedValue(false);await useDesktopStore.getState().requestQuit();expect(mock.completeDesktopExit).toHaveBeenCalledExactlyOnceWith(false);expect(useDesktopStore.getState().error).toContain('草稿已保留');expect(useDesktopStore.getState().phase).toBe('idle');});
  it('requires confirmation for changed annotations; cancellation retains them',async()=>{mock.editor.sessions.a={document:{objects:[1]},savedDocument:'{"objects":[]}'};await useDesktopStore.getState().requestQuit();expect(useDesktopStore.getState().phase).toBe('confirm');expect(mock.completeDesktopExit).not.toHaveBeenCalled();await useDesktopStore.getState().finishQuit(false);expect(mock.editor.sessions.a.document).toEqual({objects:[1]});expect(mock.completeDesktopExit).toHaveBeenCalledWith(false);});
  it('saved annotations need no discard confirmation and active drawings do',async()=>{mock.editor.sessions.a={document:{objects:[1]},savedDocument:'{"objects":[1]}'};await useDesktopStore.getState().requestQuit();expect(mock.completeDesktopExit).toHaveBeenCalledWith(true);useDesktopStore.setState({phase:'idle'});mock.editor.gesture={draft:1};await useDesktopStore.getState().requestQuit();expect(useDesktopStore.getState().phase).toBe('confirm');});
  it('does not quit while export or preference save is pending',async()=>{mock.exporting.busy=true;await useDesktopStore.getState().requestQuit();expect(mock.flushAll).not.toHaveBeenCalled();expect(mock.completeDesktopExit).toHaveBeenCalledWith(false);expect(useDesktopStore.getState().error).toContain('正在导出');});
  it('shows cleanup failures and leaves exit retryable',async()=>{mock.completeDesktopExit.mockRejectedValueOnce({message:'快捷键注销失败'});await useDesktopStore.getState().requestQuit();expect(useDesktopStore.getState().error).toBe('快捷键注销失败');expect(useDesktopStore.getState().phase).toBe('idle');await useDesktopStore.getState().requestQuit();expect(mock.completeDesktopExit).toHaveBeenCalledTimes(2);});
});

describe('Markdown exit protection',()=>{
 it('offers three choices and cancelling retains the document',async()=>{mock.documentDirty=true;await useDesktopStore.getState().requestQuit();expect(useDesktopStore.getState().phase).toBe('document');expect(mock.completeDesktopExit).not.toHaveBeenCalled();await useDesktopStore.getState().documentQuit('cancel');expect(mock.completeDesktopExit).toHaveBeenCalledWith(false);});
 it('failed or cancelled document saving blocks exit; discard is explicit',async()=>{mock.documentDirty=true;await useDesktopStore.getState().requestQuit();await useDesktopStore.getState().documentQuit('save');expect(mock.completeDesktopExit).not.toHaveBeenCalled();expect(useDesktopStore.getState().phase).toBe('document');await useDesktopStore.getState().documentQuit('discard');expect(mock.completeDesktopExit).toHaveBeenCalledWith(true);});
 it('busy document operations cancel exit rather than losing in-flight writes',async()=>{mock.document.phase='saving';await useDesktopStore.getState().requestQuit();expect(mock.completeDesktopExit).toHaveBeenCalledWith(false);});
});


describe('batch transaction exit protection',()=>{it('keeps the app open while a batch transaction is running',async()=>{useBatchStore.setState({busy:true});await useDesktopStore.getState().requestQuit();expect(mock.completeDesktopExit).toHaveBeenCalledWith(false);expect(useDesktopStore.getState().error).toContain('批量处理');});});
