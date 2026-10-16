import { beforeEach, describe, expect, it, vi } from 'vitest';
import { isTauri } from '@tauri-apps/api/core';
import { chooseSaveDirectory, defaults, getPreferences, parseSettings, updatePreferences } from '../../shared/api/settings';
import { useUiStore } from '../../app/ui-store';
import { useSettingsStore } from './store';
vi.mock('@tauri-apps/api/core', () => ({ isTauri: vi.fn(), invoke: vi.fn() }));
vi.mock('../../shared/api/settings', async (original) => ({ ...await original<typeof import('../../shared/api/settings')>(), getPreferences: vi.fn(), updatePreferences: vi.fn(), chooseSaveDirectory: vi.fn() }));
const initial = { preferences: defaults, registeredShortcut: 'Ctrl+Shift+2', shortcutError: null, registeredFullscreenShortcut: 'Ctrl+Shift+1', fullscreenShortcutError: null, storageError: null,directoryError:null };
beforeEach(() => { vi.resetAllMocks(); vi.mocked(isTauri).mockReturnValue(true); useUiStore.setState(useUiStore.getInitialState(), true); useSettingsStore.setState(useSettingsStore.getInitialState(), true); });
describe('persisted settings boundary', () => {
  it('validates system theme, annotation defaults, close behavior and directory warnings',()=>{expect(parseSettings({...initial,preferences:{...defaults,theme:'system',annotationColor:'#00aa88',annotationWidth:7,closeToTray:true},directoryError:'目录不可用'}).directoryError).toBe('目录不可用');for(const patch of [{annotationColor:'red'},{annotationWidth:0},{annotationWidth:33},{annotationWidth:1.5},{closeToTray:'yes'}])expect(()=>parseSettings({...initial,preferences:{...defaults,...patch}})).toThrow();});
  it('loads once and applies persisted appearance', async () => {
    const snapshot = { ...initial, preferences: { ...defaults, theme: 'light' as const, fontFamily: 'serif' as const, fontSize: 18 } };
    vi.mocked(getPreferences).mockResolvedValue(snapshot); await Promise.all([useSettingsStore.getState().initialize(), useSettingsStore.getState().initialize()]);
    expect(getPreferences).toHaveBeenCalledOnce(); expect(useUiStore.getState()).toMatchObject({ theme: 'light', fontFamily: 'serif', fontSize: 18 });
  });
  it('does not claim a failed save or conflict succeeded, and preserves previous appearance', async () => {
    vi.mocked(updatePreferences).mockRejectedValue({ code: 'SHORTCUT_CONFLICT', message: '已被占用' });
    await useSettingsStore.getState().change({ theme: 'light', captureShortcut: 'Ctrl+Shift+3' });
    expect(useUiStore.getState().theme).toBe('dark'); expect(useSettingsStore.getState()).toMatchObject({ phase: 'idle', error: '已被占用', snapshot: { registeredShortcut: null } });
  });
  it('folder cancellation preserves settings; a chosen folder applies the returned snapshot', async () => {
    vi.mocked(chooseSaveDirectory).mockResolvedValueOnce(null).mockResolvedValueOnce({ ...initial, preferences: { ...defaults, saveDirectory: 'C:/chosen' } });
    await useSettingsStore.getState().chooseDirectory(); expect(useSettingsStore.getState().snapshot.preferences.saveDirectory).toBeNull();
    await useSettingsStore.getState().chooseDirectory(); expect(useSettingsStore.getState().snapshot.preferences.saveDirectory).toBe('C:/chosen');
  });
  it('browser preview changes only session state and never calls native settings', async () => {
    vi.mocked(isTauri).mockReturnValue(false); await useSettingsStore.getState().initialize(); await useSettingsStore.getState().change({ theme: 'light' }); await useSettingsStore.getState().chooseDirectory();
    expect(useUiStore.getState().theme).toBe('light'); expect(getPreferences).not.toHaveBeenCalled(); expect(updatePreferences).not.toHaveBeenCalled(); expect(chooseSaveDirectory).not.toHaveBeenCalled();
  });
  it('rejects future versions, malformed fields and missing registration status', () => {
    expect(parseSettings(initial)).toEqual(initial);
    for (const bad of [null, { preferences: defaults }, { ...initial, preferences: { ...defaults, schemaVersion: 99 } }, { ...initial, preferences: { ...defaults, fontSize: 200 } }]) expect(() => parseSettings(bad)).toThrow();
  });
  it('saves full-screen shortcut independently; conflicts preserve both registered bindings',async()=>{
    useSettingsStore.setState({snapshot:initial});vi.mocked(updatePreferences).mockResolvedValueOnce({...initial,preferences:{...defaults,fullscreenShortcut:'Ctrl+Shift+4'},registeredFullscreenShortcut:'Ctrl+Shift+4'});
    await useSettingsStore.getState().change({fullscreenShortcut:'Ctrl+Shift+4'});expect(updatePreferences).toHaveBeenCalledWith({fullscreenShortcut:'Ctrl+Shift+4'});expect(useSettingsStore.getState().snapshot.registeredShortcut).toBe('Ctrl+Shift+2');
    const before=useSettingsStore.getState().snapshot;vi.mocked(updatePreferences).mockRejectedValue({code:'SHORTCUT_CONFLICT',message:'组合已占用'});await useSettingsStore.getState().change({fullscreenShortcut:'Ctrl+Shift+2'});expect(useSettingsStore.getState().snapshot).toBe(before);expect(useSettingsStore.getState().error).toBe('组合已占用');
    for(const bad of [{...initial,registeredFullscreenShortcut:undefined},{...initial,preferences:{...defaults,fullscreenShortcut:''}},{...initial,fullscreenShortcutError:undefined}])expect(()=>parseSettings(bad)).toThrow();
  });
});
