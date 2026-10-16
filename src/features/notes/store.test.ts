import {beforeEach,afterEach,describe,it,expect,vi} from 'vitest';
import {createCardStore,CARD_SAVE_DELAY,isDirty,MAX_CARD_SESSIONS} from './store';
import type {KnowledgeCard,CardDraft} from '../../shared/api/library';
const api=vi.hoisted(()=>({getKnowledgeCard:vi.fn(),saveKnowledgeCard:vi.fn()}));vi.mock('../../shared/api/library',()=>api);
const fixture=(id='a'.repeat(32)):KnowledgeCard=>({screenshot:{id,title:'细胞图像',originalImagePath:`originals/${id}.png`,thumbnailPath:`thumbnails/${id}.png`,widthPx:32,heightPx:24,createdAt:1,updatedAt:2,revision:1,isFavorite:false,imageAvailable:true,thumbnailAvailable:true},note:null,tags:[],additionalNoteCount:0});
const saved=(card:KnowledgeCard,draft:CardDraft):KnowledgeCard=>({...card,screenshot:{...card.screenshot,title:draft.title,isFavorite:draft.isFavorite,revision:card.screenshot.revision+1,updatedAt:3},note:{id:'f'.repeat(32),screenshotId:card.screenshot.id,content:draft.content,sourceUrl:draft.sourceUrl||null,createdAt:1,updatedAt:3,revision:(card.note?.revision??0)+1},tags:draft.tagNames.map((name,i)=>({id:(i+1).toString(16).padStart(32,'0'),name}))});
let controller:ReturnType<typeof createCardStore>;
beforeEach(()=>{vi.useFakeTimers();vi.clearAllMocks();api.getKnowledgeCard.mockImplementation(async(id:string)=>fixture(id));api.saveKnowledgeCard.mockImplementation(async(card:KnowledgeCard,draft:CardDraft)=>saved(card,draft));controller=createCardStore();});
afterEach(()=>{controller.dispose();vi.useRealTimers();});
describe('per-card debounced autosave and recoverable drafts',()=>{
  it('refuses deletion while drafts or in-flight loads exist, preserving failed notes',async()=>{
    const {store}=controller,id='a'.repeat(32);await store.getState().open(id);store.getState().edit(id,{content:'不可丢失的笔记'});api.saveKnowledgeCard.mockRejectedValueOnce(new Error('写入失败'));await store.getState().flush(id);
    expect(store.getState().beginDelete(id)).toBe(false);expect(store.getState().sessions[id].draft?.content).toBe('不可丢失的笔记');expect(store.getState().deletingIds).toEqual([]);
    const other='b'.repeat(32);let resolve:(card:KnowledgeCard)=>void=()=>{};api.getKnowledgeCard.mockImplementationOnce(()=>new Promise(r=>{resolve=r;}));const load=store.getState().open(other);expect(store.getState().beginDelete(other)).toBe(false);resolve(fixture(other));await load;
  });
  it('locks only the deleting card, removes its clean cache on success and restores it on failure',async()=>{
    const {store}=controller,id='a'.repeat(32),other='b'.repeat(32);await store.getState().open(id);await store.getState().open(other);
    expect(store.getState().beginDelete(id)).toBe(true);expect(store.getState().beginDelete(id)).toBe(false);store.getState().edit(id,{content:'删除途中不写入'});expect(store.getState().sessions[id].draft?.content).toBe('');await store.getState().open(id);expect(store.getState().activeId).toBe(other);
    store.getState().edit(other,{content:'其他卡片继续编辑'});store.getState().finishDelete(id,false);expect(store.getState().sessions[id]).toBeDefined();expect(store.getState().deletingIds).toEqual([]);
    expect(store.getState().beginDelete(id)).toBe(true);store.getState().finishDelete(id,true);expect(store.getState().sessions[id]).toBeUndefined();expect(store.getState().sessions[other].draft?.content).toBe('其他卡片继续编辑');expect(store.getState().deletingIds).toEqual([]);expect(await store.getState().flushAll()).toBe(true);
  });
  it('debounces text and stores Markdown literally in one atomic card write',async()=>{
    const {store}=controller,id='a'.repeat(32);await store.getState().open(id);store.getState().edit(id,{content:'# 研究'});await vi.advanceTimersByTimeAsync(400);store.getState().edit(id,{content:'# 研究\n<script>alert(1)</script>',tagNames:['细胞','方法'],sourceUrl:'https://example.org'});await vi.advanceTimersByTimeAsync(CARD_SAVE_DELAY-1);expect(api.saveKnowledgeCard).not.toHaveBeenCalled();await vi.advanceTimersByTimeAsync(1);expect(api.saveKnowledgeCard).toHaveBeenCalledTimes(1);expect(store.getState().sessions[id].draft?.content).toContain('<script>');expect(isDirty(store.getState().sessions[id])).toBe(false);
  });
  it('flushes a previous card on switch and preserves independent drafts and note IDs',async()=>{
    const {store}=controller,a='a'.repeat(32),b='b'.repeat(32);await store.getState().open(a);store.getState().edit(a,{content:'A未保存'});await store.getState().open(b);store.getState().edit(b,{content:'B科研'});await store.getState().flushAll();expect(store.getState().sessions[a].draft?.content).toBe('A未保存');expect(store.getState().sessions[b].draft?.content).toBe('B科研');expect(store.getState().sessions[a].card?.note?.screenshotId).toBe(a);expect(store.getState().sessions[b].card?.note?.screenshotId).toBe(b);
  });
  it('serializes a late save and drains newer edits without overwriting them',async()=>{
    const {store}=controller,id='a'.repeat(32);await store.getState().open(id);store.getState().edit(id,{content:'旧请求'});
    let resolve:(v:KnowledgeCard)=>void=()=>{};api.saveKnowledgeCard.mockImplementationOnce(()=>new Promise(r=>{resolve=r;}));
    const old=store.getState().sessions[id],pending=store.getState().flush(id);store.getState().edit(id,{content:'最新文本',tagNames:['新标签']});
    const repeated=store.getState().flush(id);expect(api.saveKnowledgeCard).toHaveBeenCalledTimes(1);resolve(saved(old.card!,old.draft!));await pending;await repeated;
    expect(api.saveKnowledgeCard).toHaveBeenCalledTimes(2);expect(api.saveKnowledgeCard.mock.calls[1][0].note?.id).toBe('f'.repeat(32));expect(store.getState().sessions[id].draft?.content).toBe('最新文本');expect(store.getState().sessions[id].savedVersion).toBe(2);expect(isDirty(store.getState().sessions[id])).toBe(false);
  });
  it('retains failed and conflicting drafts across card switches and supports retry',async()=>{
    const {store}=controller,id='a'.repeat(32);await store.getState().open(id);store.getState().edit(id,{content:'不可丢失'});api.saveKnowledgeCard.mockRejectedValue({code:'REVISION_CONFLICT',message:'资料冲突'});expect(await store.getState().flush(id)).toBe(false);await store.getState().open('b'.repeat(32));await store.getState().open(id);expect(store.getState().sessions[id].draft?.content).toBe('不可丢失');expect(isDirty(store.getState().sessions[id])).toBe(true);expect(store.getState().sessions[id].error).toContain('冲突');api.saveKnowledgeCard.mockImplementation(async(c:KnowledgeCard,d:CardDraft)=>saved(c,d));expect(await store.getState().flush(id)).toBe(true);
  });
  it('late load response belongs to its own card, not the currently active one',async()=>{
    const {store}=controller,a='a'.repeat(32),b='b'.repeat(32);let resolve:(v:KnowledgeCard)=>void=()=>{};api.getKnowledgeCard.mockImplementationOnce(()=>new Promise(r=>{resolve=r;}));const old=store.getState().open(a);await store.getState().open(b);resolve(fixture(a));await old;expect(store.getState().activeId).toBe(b);expect(store.getState().sessions[b].card?.screenshot.id).toBe(b);
  });
  it('reload failure preserves dirty content and successful explicit reload restores persisted data',async()=>{
    const {store}=controller,id='a'.repeat(32);await store.getState().open(id);store.getState().edit(id,{content:'保留'});api.getKnowledgeCard.mockRejectedValueOnce({message:'读取失败'});await store.getState().reload(id);expect(store.getState().sessions[id].draft?.content).toBe('保留');expect(isDirty(store.getState().sessions[id])).toBe(true);api.getKnowledgeCard.mockResolvedValue(fixture(id));await store.getState().reload(id);expect(store.getState().sessions[id].draft?.content).toBe('');expect(isDirty(store.getState().sessions[id])).toBe(false);
  });
  it('never evicts dirty sessions when the cache is full',async()=>{
    const {store}=controller;api.saveKnowledgeCard.mockRejectedValue({message:'保存失败'});
    for(let n=1;n<=MAX_CARD_SESSIONS;n++){const id=n.toString(16).padStart(32,'0');await store.getState().open(id);store.getState().edit(id,{content:`草稿 ${n}`});}
    await store.getState().open('e'.repeat(32));expect(Object.keys(store.getState().sessions)).toHaveLength(20);expect(store.getState().sessions['e'.repeat(32)]).toBeUndefined();expect(store.getState().error).toContain('20');expect(store.getState().sessions['1'.padStart(32,'0')].draft?.content).toBe('草稿 1');
  });
  it('clean cached cards reload notes and tags rather than replacing persisted data with an old snapshot',async()=>{
    const {store}=controller,id='a'.repeat(32);await store.getState().open(id);api.getKnowledgeCard.mockResolvedValue(saved(fixture(id),{title:'新标题',content:'重新加载',sourceUrl:'',isFavorite:true,tagNames:['标签']}));await store.getState().open(id);expect(store.getState().sessions[id].draft).toMatchObject({content:'重新加载',tagNames:['标签'],isFavorite:true});
  });
});
