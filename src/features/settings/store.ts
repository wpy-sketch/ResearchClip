import { create } from 'zustand';
import { isTauri } from '@tauri-apps/api/core';
import { useUiStore } from '../../app/ui-store';
import { chooseSaveDirectory, defaults, getPreferences, updatePreferences, type PreferencesPatch, type SettingsSnapshot } from '../../shared/api/settings';
import { errorMessage } from '../../shared/api/errors';
import {resolveTheme} from './theme';
import {useEditorStore} from '../editor/store';

function apply(snapshot: SettingsSnapshot) {
  const { theme, viewMode, fontFamily, fontSize } = snapshot.preferences;
  useUiStore.setState({ theme:resolveTheme(theme,typeof matchMedia==='function'?matchMedia('(prefers-color-scheme: dark)').matches:true), viewMode, fontFamily, fontSize });
  const editor=useEditorStore.getState(),preferences=snapshot.preferences;
  if(editor.defaultStyle.color!==preferences.annotationColor||editor.defaultStyle.strokeWidth!==preferences.annotationWidth)editor.setDefaults({color:preferences.annotationColor,strokeWidth:preferences.annotationWidth});
}
interface SettingsState {
  snapshot: SettingsSnapshot; phase: 'idle' | 'loading' | 'saving'; initialized: boolean; error: string | null;
  initialize: () => Promise<void>; refresh:()=>Promise<void>;change: (patch: PreferencesPatch) => Promise<void>; chooseDirectory: () => Promise<void>;
}
export const useSettingsStore = create<SettingsState>((set, get) => ({
  snapshot: { preferences: defaults, registeredShortcut: null, shortcutError: null, registeredFullscreenShortcut: null, fullscreenShortcutError: null, storageError: null,directoryError:null }, phase: 'idle', initialized: false, error: null,
  refresh:async()=>{if(!isTauri()||get().phase!=='idle')return;set({phase:'loading'});try{const snapshot=await getPreferences();apply(snapshot);set({snapshot,phase:'idle'});}catch(error){set({phase:'idle',error:errorMessage(error,'设置状态读取失败。')});}},
  initialize: async () => {
    if (get().initialized || get().phase !== 'idle') return;
    if (!isTauri()) { set({ initialized: true }); return; }
    set({ phase: 'loading', error: null });
    try { const snapshot = await getPreferences(); apply(snapshot); set({ snapshot, phase: 'idle', initialized: true }); }
    catch (error) { set({ phase: 'idle', initialized: true, error: errorMessage(error, '无法加载本地设置。') }); }
  },
  change: async (patch) => {
    if (get().phase !== 'idle') return;
    set({ phase: 'saving', error: null });
    try {
      const snapshot = isTauri() ? await updatePreferences(patch) : { ...get().snapshot, preferences: { ...get().snapshot.preferences, ...patch } };
      apply(snapshot); set({ snapshot, phase: 'idle' });
    } catch (error) { set({ phase: 'idle', error: errorMessage(error, '设置未保存，请重试。') }); }
  },
  chooseDirectory: async () => {
    if (!isTauri() || get().phase !== 'idle') return;
    set({ phase: 'saving', error: null });
    try { const snapshot = await chooseSaveDirectory(); if (snapshot) { apply(snapshot); set({ snapshot }); } set({ phase: 'idle' }); }
    catch (error) { set({ phase: 'idle', error: errorMessage(error, '无法选择保存目录。') }); }
  },
}));
