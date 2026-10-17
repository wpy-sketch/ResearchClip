import { invoke } from '@tauri-apps/api/core';
export interface Preferences { schemaVersion: 1; theme: 'dark' | 'light'|'system'; viewMode: 'grid' | 'list'; fontFamily: 'system' | 'sans' | 'serif'; fontSize: number; captureShortcut: string; fullscreenShortcut: string; saveDirectory: string | null; annotationColor:string;annotationWidth:number;closeToTray:boolean;markdownSplitPercent:number }
export interface SettingsSnapshot { preferences: Preferences; registeredShortcut: string | null; shortcutError: string | null; registeredFullscreenShortcut: string | null; fullscreenShortcutError: string | null; storageError: string | null;directoryError:string|null }
export type PreferencesPatch = Partial<Pick<Preferences, 'theme' | 'viewMode' | 'fontFamily' | 'fontSize' | 'captureShortcut' | 'fullscreenShortcut'|'annotationColor'|'annotationWidth'|'closeToTray'|'markdownSplitPercent'>>;
export const defaults: Preferences = { schemaVersion: 1, theme: 'dark', viewMode: 'grid', fontFamily: 'system', fontSize: 16, captureShortcut: 'Ctrl+Shift+2', fullscreenShortcut: 'Ctrl+Shift+1', saveDirectory: null,annotationColor:'#ef6262',annotationWidth:3,closeToTray:false,markdownSplitPercent:50 };
export function parseSettings(value: unknown): SettingsSnapshot {
  if (!value || typeof value !== 'object') throw new Error('设置响应无效。');
  const s = value as Record<string, unknown>; const p = s.preferences as Record<string, unknown> | undefined;
  const nullable = (v: unknown) => v === null || typeof v === 'string';
  if (!p || p.schemaVersion !== 1 || !['dark','light','system'].includes(String(p.theme)) || !['grid','list'].includes(String(p.viewMode)) || !['system','sans','serif'].includes(String(p.fontFamily))
    || typeof p.fontSize !== 'number' || !Number.isInteger(p.fontSize) || p.fontSize < 12 || p.fontSize > 20 || typeof p.captureShortcut !== 'string' || !p.captureShortcut || p.captureShortcut.length>128 || typeof p.fullscreenShortcut!=='string' || !p.fullscreenShortcut || p.fullscreenShortcut.length>128 || !nullable(p.saveDirectory)
    || typeof p.annotationColor!=='string'||!/^#[0-9a-f]{6}$/i.test(p.annotationColor)||typeof p.annotationWidth!=='number'||!Number.isInteger(p.annotationWidth)||p.annotationWidth<1||p.annotationWidth>32||typeof p.closeToTray!=='boolean'
    || !nullable(s.registeredShortcut) || !nullable(s.shortcutError) || !nullable(s.registeredFullscreenShortcut) || !nullable(s.fullscreenShortcutError) || !nullable(s.storageError)||!nullable(s.directoryError)) throw new Error('设置协议或版本不兼容，已有配置未修改。');
  if(p.markdownSplitPercent===undefined)p.markdownSplitPercent=50;
  if(typeof p.markdownSplitPercent!=='number'||!Number.isInteger(p.markdownSplitPercent)||p.markdownSplitPercent<20||p.markdownSplitPercent>80)throw new Error('分栏比例无效。');
  return value as SettingsSnapshot;
}
export async function getPreferences() { return parseSettings(await invoke<unknown>('get_preferences')); }
export async function updatePreferences(patch: PreferencesPatch) { return parseSettings(await invoke<unknown>('update_preferences', { patch })); }
export async function chooseSaveDirectory(): Promise<SettingsSnapshot | null> { const value = await invoke<unknown>('choose_save_directory'); return value === null ? null : parseSettings(value); }
