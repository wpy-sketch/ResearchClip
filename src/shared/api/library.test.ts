import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createScreenshot, parseScreenshot, parseSummary, parseStatus, packScreenshot, saveNote, setScreenshotTags, screenshotImage, parseScreenshotPage, readScreenshotPage,parseKnowledgeCard,getKnowledgeCard,saveKnowledgeCard,openCardSource } from './library';
const native=vi.hoisted(()=>({invoke:vi.fn(),isTauri:vi.fn(()=>true)}));vi.mock('@tauri-apps/api/core',()=>native);
const id='a'.repeat(32),doc={schemaVersion:1 as const,image:{widthPx:32,heightPx:24},objects:[]};
const item={id,title:"O'Brien",originalImagePath:`originals/${id}.png`,thumbnailPath:`thumbnails/${id}.png`,widthPx:32,heightPx:24,createdAt:1,updatedAt:2,revision:1,isFavorite:false,annotationData:doc};
beforeEach(()=>{native.invoke.mockReset();native.isTauri.mockReturnValue(true);});
describe('native persisted library contract',()=>{
  it('loads and saves a card with literal Markdown, expected revisions and multiple tag names',async()=>{
    const card={screenshot:parseSummary({...item,imageAvailable:true,thumbnailAvailable:true}),note:null,tags:[],additionalNoteCount:0};
    native.invoke.mockResolvedValue(card);expect(await getKnowledgeCard(id)).toEqual(card);
    const draft={title:'A',content:'# 实验\n<script>alert(1)</script>',sourceUrl:'https://example.org',tagNames:['A','方法'],isFavorite:true};await saveKnowledgeCard(card,draft);
    expect(native.invoke).toHaveBeenLastCalledWith('save_knowledge_card',{input:{id,revision:1,title:'A',content:draft.content,sourceUrl:'https://example.org',tagNames:['A','方法'],isFavorite:true,noteId:null,noteRevision:null}});
    await openCardSource(id);expect(native.invoke).toHaveBeenLastCalledWith('open_card_source',{id});
  });
  it('rejects mismatched note ownership and duplicate card tags',()=>{
    const card={screenshot:{...item,imageAvailable:true,thumbnailAvailable:true},note:null,tags:[],additionalNoteCount:0};
    expect(()=>parseKnowledgeCard({...card,note:{id,screenshotId:'b'.repeat(32),content:'A',sourceUrl:null,createdAt:1,updatedAt:1,revision:1}})).toThrow('关联');
    expect(()=>parseKnowledgeCard({...card,tags:[{id,name:'A'},{id,name:'B'}]})).toThrow('关联');
  });
  it('keeps Chinese and wildcard search strings as typed arguments, validates tag filters before IPC',async()=>{
    native.invoke.mockResolvedValue({items:[],total:0,page:0,limit:24,sort:'createdDesc'});const filter={query:"科研%_\\ O'Brien",tagIds:[id],favorite:false};await readScreenshotPage(0,false,'createdDesc',filter);
    expect(native.invoke).toHaveBeenLastCalledWith('screenshot_page',{page:0,limit:24,favorites:false,sort:'createdDesc',filter});
    native.invoke.mockClear();await expect(readScreenshotPage(0,false,'createdDesc',{...filter,query:'x'.repeat(201)})).rejects.toThrow();await expect(readScreenshotPage(0,false,'createdDesc',{...filter,tagIds:['../a']})).rejects.toThrow();expect(native.invoke).not.toHaveBeenCalled();
  });
  it('reads a bounded page using typed sorting and validates empty/clamped responses',async()=>{
    const page={items:[{...item,imageAvailable:true,thumbnailAvailable:true}],total:25,page:1,limit:24,sort:'createdAsc'};
    native.invoke.mockResolvedValue(page);expect(await readScreenshotPage(3,true,'createdAsc')).toEqual({...page,items:page.items.map(parseSummary)});
    expect(native.invoke).toHaveBeenLastCalledWith('screenshot_page',{page:3,limit:24,favorites:true,sort:'createdAsc'});
    expect(parseScreenshotPage({items:[],total:0,page:0,limit:24,sort:'createdDesc'}).total).toBe(0);
  });
  it('rejects malformed/duplicate/oversized pages and mismatched sorting',async()=>{
    const summary={...item,imageAvailable:true,thumbnailAvailable:true};
    const page={items:[summary],total:1,page:0,limit:24,sort:'createdDesc'};
    for(const bad of [{...page,total:2},{...page,page:1},{...page,limit:50},{...page,total:25,items:Array(25).fill(summary)},{...page,total:2,items:[summary,summary]},{...page,sort:'id;DROP TABLE screenshots'}])expect(()=>parseScreenshotPage(bad)).toThrow();
    native.invoke.mockResolvedValue({...page,sort:'createdAsc'});await expect(readScreenshotPage()).rejects.toThrow('不一致');
  });
  it('rejects invalid page arguments before IPC',async()=>{
    await expect(readScreenshotPage(-1)).rejects.toThrow('参数');await expect(readScreenshotPage(1.5)).rejects.toThrow('参数');
    await expect(readScreenshotPage(1_000_001)).rejects.toThrow('参数');expect(native.invoke).not.toHaveBeenCalled();
  });
  it('uses bounded binary PNG upload, title and versioned annotation JSON; no arbitrary path',async()=>{
    const bytes=await packScreenshot(new Blob(['source PNG']),item.title,doc),size=new DataView(bytes).getUint32(0,true);expect(JSON.parse(new TextDecoder().decode(bytes.slice(4,4+size)))).toEqual({title:item.title,document:doc});expect(new TextDecoder().decode(bytes.slice(4+size))).toBe('source PNG');native.invoke.mockResolvedValue(item);expect(await createScreenshot(new Blob(['PNG']),item.title,doc)).toEqual(item);expect(native.invoke.mock.calls[0][1]).toBeInstanceOf(ArrayBuffer);
  });
  it('validates opaque IDs, managed refs, revisions, dimensions and future annotation versions',()=>{
    expect(parseScreenshot(item)).toEqual(item);for(const bad of [{...item,id:'../x'},{...item,originalImagePath:'C:/private.png'},{...item,revision:0},{...item,widthPx:33},{...item,annotationData:{...doc,schemaVersion:99}}])expect(()=>parseScreenshot(bad)).toThrow();
    expect(parseSummary({...item,imageAvailable:true,thumbnailAvailable:false}).imageAvailable).toBe(true);expect(()=>parseSummary(item)).toThrow();expect(parseStatus({schemaVersion:3,pendingWrites:1,pendingCleanup:0,warnings:['保留原图']})).toMatchObject({pendingWrites:1});expect(()=>parseStatus({schemaVersion:99})).toThrow();
  });
  it('propagates native errors and refuses browser-only persistence',async()=>{
    native.isTauri.mockReturnValue(false);await expect(createScreenshot(new Blob(['PNG']),'A',doc)).rejects.toThrow('桌面');expect(native.invoke).not.toHaveBeenCalled();native.isTauri.mockReturnValue(true);native.invoke.mockRejectedValue({code:'FILE_UNAVAILABLE',message:'图片保存失败'});await expect(createScreenshot(new Blob(['PNG']),'A',doc)).rejects.toMatchObject({code:'FILE_UNAVAILABLE'});
  });
  it('keeps note content and URL in typed arguments; validates returned note and tag transaction revision',async()=>{
    const input={screenshotId:id,content:"'); DROP TABLE screenshots; --",sourceUrl:'https://example.org'};native.invoke.mockResolvedValue({id,screenshotId:id,content:input.content,sourceUrl:input.sourceUrl,createdAt:1,updatedAt:1,revision:1});expect((await saveNote(input)).content).toBe(input.content);expect(native.invoke).toHaveBeenCalledWith('save_note',{input});native.invoke.mockResolvedValue({...item,revision:2});expect((await setScreenshotTags(id,1,['b'.repeat(32)])).revision).toBe(2);expect(native.invoke).toHaveBeenLastCalledWith('set_screenshot_tags',{id,revision:1,tagIds:['b'.repeat(32)]});
  });
  it('rejects malformed binary resource responses without returning fake images',async()=>{native.invoke.mockResolvedValue('C:/image.png');await expect(screenshotImage(id)).rejects.toThrow('响应');native.invoke.mockResolvedValue(new ArrayBuffer(4));await expect(screenshotImage(id)).rejects.toThrow();});
});
