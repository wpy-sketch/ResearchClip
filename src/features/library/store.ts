import { create } from 'zustand';
import { isTauri } from '@tauri-apps/api/core';
import { libraryStatus, readScreenshotPage } from '../../shared/api/library';
import type { LibrarySort, LibraryStatus, ScreenshotSummary, LibraryFilter } from '../../shared/api/library';
import { errorMessage } from '../../shared/api/errors';

interface LibraryState {
  items: ScreenshotSummary[];
  status: LibraryStatus | null;
  error: string | null;
  loading: boolean;
  favorites: boolean;
  sort: LibrarySort;
  page: number;
  total: number;
  query:string;
  tagIds:string[];
  favorite:boolean|null;
  generation: number;
  refresh: (favorites?: boolean, options?: {page?:number; sort?:LibrarySort}&Partial<LibraryFilter>) => Promise<void>;
  retryRecovery: () => Promise<void>;
}

export const useLibraryStore = create<LibraryState>((set, get) => ({
  items: [], status: null, error: null, loading: false, favorites: false,
  sort: 'createdDesc', page: 0, total: 0, generation: 0,query:'',tagIds:[],favorite:null,
  refresh: async (favorites = get().favorites, options = {}) => {
    if (!isTauri()) return;
    const current = get(), sort = options.sort ?? current.sort;
    const filter:LibraryFilter={query:(options.query??current.query).trim(),tagIds:options.tagIds??current.tagIds,favorite:options.favorite===undefined?current.favorite:options.favorite};
    const changed = favorites !== current.favorites || sort !== current.sort||filter.query!==current.query||filter.favorite!==current.favorite||JSON.stringify(filter.tagIds)!==JSON.stringify(current.tagIds);
    const page = options.page ?? (changed ? 0 : current.page);
    const generation = current.generation + 1;
    set({generation, loading: true, error: null, favorites, sort,...filter,
      ...(changed ? {items: [], page: 0, total: 0} : {})});
    try {
      const [result, status] = await Promise.all([readScreenshotPage(page, favorites, sort,filter), libraryStatus()]);
      if (get().generation === generation) {
        set({items: result.items, page: result.page, total: result.total, status, loading: false});
      }
    } catch (error) {
      if (get().generation === generation) {
        set({loading: false, error: errorMessage(error, '无法读取本地素材库，已有资料未重置。')});
      }
    }
  },
  retryRecovery: async () => {
    try {const status = await libraryStatus(true); set({status}); await get().refresh();}
    catch (error) {set({error: errorMessage(error, '恢复未完成，请检查工作区文件和权限。')});}
  },
}));
