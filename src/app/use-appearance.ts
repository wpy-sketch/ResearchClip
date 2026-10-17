import { useEffect } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useUiStore } from './ui-store';
import {useSettingsStore} from '../features/settings/store';
import {observeTheme} from '../features/settings/theme';

export function useAppearance() {
  const preference = useSettingsStore(state=>state.snapshot.preferences.theme);
  const fontFamily = useUiStore((state) => state.fontFamily);
  const fontSize = useUiStore((state) => state.fontSize);
  const setError = useUiStore((state) => state.setWindowThemeError);
  useEffect(() => {
    const cleanup=observeTheme(preference,window.matchMedia('(prefers-color-scheme: dark)'),theme=>{
      document.documentElement.dataset.theme=theme;useUiStore.setState({theme});
      document.querySelector('meta[name="theme-color"]')?.setAttribute('content', theme === 'dark' ? '#131314' : '#f0f4f9');
    });
    let active = true;
    if (isTauri()) {
      void getCurrentWindow().setTheme(preference==='system'?null:preference).then(() => {
        if (active) setError(null);
      }).catch(() => {
        if (active) setError('系统标题栏主题同步失败，应用内主题仍可使用。');
      });
    }
    return () => { active = false;cleanup(); };
  }, [preference, setError]);
  useEffect(() => {
    const families = { system: '"Segoe UI", "Microsoft YaHei", system-ui, sans-serif', sans: 'Arial, "Microsoft YaHei", sans-serif', serif: 'Georgia, "SimSun", serif' };
    document.documentElement.style.setProperty('--rc-font-family', families[fontFamily]);
    document.documentElement.style.fontSize = `${fontSize}px`;
  }, [fontFamily, fontSize]);
}
