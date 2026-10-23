import {beforeEach, describe, expect, it, vi} from 'vitest';
import {useLibraryStore} from './store';
import type {LibrarySort} from '../../shared/api/library';
const api = vi.hoisted(() => ({readScreenshotPage:vi.fn(), libraryStatus:vi.fn()}));
vi.mock('../../shared/api/library', () => api);
vi.mock('@tauri-apps/api/core', () => ({isTauri:() => true}));
const status = {schemaVersion:12, pendingWrites:0, pendingCleanup:0, warnings:[]};
const item = (n:number, isFavorite=false) => ({id:n.toString(16).padStart(32,'0'), title:`素材 ${n}`,
  originalImagePath:`originals/${n.toString(16).padStart(32,'0')}.png`, thumbnailPath:`thumbnails/${n.toString(16).padStart(32,'0')}.png`,
  widthPx:32, heightPx:24, createdAt:n, updatedAt:n, revision:1, isFavorite, imageAvailable:true, thumbnailAvailable:true});
const result = (page=0, total=50, sort:LibrarySort='createdDesc', favorites=false) => ({
  page, total, limit:24, sort, items:Array.from({length:Math.min(24,total-page*24)}, (_,n) => item(page*24+n+1,favorites)),
});
beforeEach(() => {vi.clearAllMocks(); useLibraryStore.setState(useLibraryStore.getInitialState(),true); api.libraryStatus.mockResolvedValue(status);});
describe('bounded native library pages', () => {
  it('source changes clear old results and send the same source on subsequent pages',async()=>{api.readScreenshotPage.mockResolvedValue(result(0,1));await useLibraryStore.getState().refresh(false,{query:'科研',source:'ocr'});expect(api.readScreenshotPage).toHaveBeenLastCalledWith(0,false,'createdDesc',expect.objectContaining({source:'ocr',query:'科研'}));await useLibraryStore.getState().refresh(false,{source:'all',query:''});expect(useLibraryStore.getState().source).toBe('all');});
  it('combines query, tags, favorites and sort while resetting the page',async()=>{
    api.readScreenshotPage.mockResolvedValueOnce(result(2)).mockResolvedValueOnce(result(0,1,'createdAsc'));
    await useLibraryStore.getState().refresh(false,{page:2});const tagIds=['a'.repeat(32),'b'.repeat(32)];
    await useLibraryStore.getState().refresh(false,{query:'  科研 Alpha%_  ',tagIds,favorite:true,sort:'createdAsc'});
    expect(api.readScreenshotPage).toHaveBeenLastCalledWith(0,false,'createdAsc',{query:'科研 Alpha%_',tagIds,favorite:true});
    expect(useLibraryStore.getState()).toMatchObject({query:'科研 Alpha%_',tagIds,favorite:true,page:0});
  });
  it('a stale query result cannot replace a newer Chinese query response',async()=>{
    let resolve:(v:unknown)=>void=()=>{};api.readScreenshotPage.mockImplementationOnce(()=>new Promise(r=>{resolve=r;})).mockResolvedValueOnce(result(0,1));
    const old=useLibraryStore.getState().refresh(false,{query:'Alpha'});await useLibraryStore.getState().refresh(false,{query:'中文'});resolve(result(0,24));await old;
    expect(useLibraryStore.getState()).toMatchObject({query:'中文',total:1,items:[item(1)]});
  });
  it('clears all filter values including false favorites and handles zero matches',async()=>{
    api.readScreenshotPage.mockResolvedValue(result(0,0));await useLibraryStore.getState().refresh(true,{query:'none',tagIds:['a'.repeat(32)],favorite:false});
    await useLibraryStore.getState().refresh(false,{query:'',tagIds:[],favorite:null,sort:'createdDesc',page:0});
    expect(useLibraryStore.getState()).toMatchObject({favorites:false,query:'',tagIds:[],favorite:null,total:0,page:0});
  });
  it('ignores late previous-view responses and clears records from a different filter', async () => {
    let oldResolve:(v:unknown)=>void = () => {}, nextResolve:(v:unknown)=>void = () => {};
    api.readScreenshotPage.mockImplementationOnce(() => new Promise(resolve => {oldResolve=resolve;}))
      .mockImplementationOnce(() => new Promise(resolve => {nextResolve=resolve;}));
    const old=useLibraryStore.getState().refresh(false), next=useLibraryStore.getState().refresh(true);
    nextResolve(result(0,1,'createdDesc',true)); await next; oldResolve(result()); await old;
    expect(useLibraryStore.getState()).toMatchObject({items:[item(1,true)], favorites:true, total:1, loading:false});
  });
  it('replaces each page instead of accumulating original images or records', async () => {
    api.readScreenshotPage.mockResolvedValueOnce(result()).mockResolvedValueOnce(result(1));
    await useLibraryStore.getState().refresh(); await useLibraryStore.getState().refresh(false,{page:1});
    expect(useLibraryStore.getState()).toMatchObject({items:result(1).items,page:1,total:50});
    expect(useLibraryStore.getState().items).toHaveLength(24);
    expect(api.readScreenshotPage).toHaveBeenLastCalledWith(1,false,'createdDesc',{query:'',tagIds:[],favorite:null});
  });
  it('keeps the visible page and error after a failed next-page read', async () => {
    api.readScreenshotPage.mockResolvedValueOnce(result()).mockRejectedValueOnce({message:'数据库忙'});
    await useLibraryStore.getState().refresh(); await useLibraryStore.getState().refresh(false,{page:1});
    expect(useLibraryStore.getState()).toMatchObject({items:result().items,page:0,total:50,loading:false});
    expect(useLibraryStore.getState().error).toContain('忙');
  });
  it('resets the page on sort/filter changes and rejects stale sort responses', async () => {
    api.readScreenshotPage.mockResolvedValueOnce(result(2)); await useLibraryStore.getState().refresh(false,{page:2});
    let resolve:(v:unknown)=>void = () => {};
    api.readScreenshotPage.mockImplementationOnce(() => new Promise(r => {resolve=r;}));
    const sorting=useLibraryStore.getState().refresh(false,{sort:'createdAsc'});
    expect(useLibraryStore.getState()).toMatchObject({items:[],page:0,total:0,loading:true});
    api.readScreenshotPage.mockResolvedValueOnce(result(0,1,'createdAsc',true)); await useLibraryStore.getState().refresh(true);
    resolve(result(0,50,'createdAsc')); await sorting;
    expect(useLibraryStore.getState()).toMatchObject({page:0,total:1,sort:'createdAsc',favorites:true,items:[item(1,true)]});
  });
  it('accepts the clamped page after deleting the last item or unfavoriting it', async () => {
    api.readScreenshotPage.mockResolvedValueOnce(result(1,25)).mockResolvedValueOnce(result(0,24));
    await useLibraryStore.getState().refresh(false,{page:1}); await useLibraryStore.getState().refresh();
    expect(api.readScreenshotPage).toHaveBeenLastCalledWith(1,false,'createdDesc',{query:'',tagIds:[],favorite:null});
    expect(useLibraryStore.getState()).toMatchObject({page:0,total:24,items:result(0,24).items});
  });
  it('preserves the page during mutation refresh and supports a truly empty library', async () => {
    api.readScreenshotPage.mockResolvedValueOnce(result(1,25)).mockResolvedValueOnce({...result(1,25),items:[{...item(25),title:'重命名',revision:2,isFavorite:true}]}).mockResolvedValueOnce(result(0,0));
    await useLibraryStore.getState().refresh(false,{page:1}); await useLibraryStore.getState().refresh();
    expect(useLibraryStore.getState().items[0]).toMatchObject({title:'重命名',revision:2,isFavorite:true});
    await useLibraryStore.getState().refresh(true); expect(useLibraryStore.getState()).toMatchObject({items:[],total:0,page:0});
  });
  it('reports recovery failure without clearing saved records', async () => {
    api.readScreenshotPage.mockResolvedValue(result(0,1)); await useLibraryStore.getState().refresh();
    api.libraryStatus.mockRejectedValue({message:'文件清理失败'}); await useLibraryStore.getState().retryRecovery();
    expect(useLibraryStore.getState().items).toHaveLength(1); expect(useLibraryStore.getState().error).toContain('清理失败');
  });
});


describe('project filters reach the actual page query',()=>{it('forwards project and unclassified scopes, then removes both for all materials',async()=>{api.readScreenshotPage.mockResolvedValue(result(0,0));const s=useLibraryStore.getState();await s.refresh(false,{projectId:'a'.repeat(32),unclassified:false});expect(api.readScreenshotPage).toHaveBeenLastCalledWith(0,false,'createdDesc',{query:'',tagIds:[],favorite:null,projectId:'a'.repeat(32)});await s.refresh(false,{projectId:null,unclassified:true});expect(api.readScreenshotPage).toHaveBeenLastCalledWith(0,false,'createdDesc',{query:'',tagIds:[],favorite:null,unclassified:true});await s.refresh(false,{projectId:null,unclassified:false});expect(api.readScreenshotPage).toHaveBeenLastCalledWith(0,false,'createdDesc',{query:'',tagIds:[],favorite:null});});});
