import { useEffect } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { parseCaptureInfo } from '../../shared/api/capture';
import { errorMessage } from '../../shared/api/errors';
import { useUiStore } from '../../app/ui-store';
import { useCaptureStore } from './store';

// Handles React StrictMode and disposal before an asynchronous listen() resolves.
export function attachListener(subscribe: (handler: (value: unknown) => void) => Promise<UnlistenFn>, handler: (value: unknown) => void, failed: (error: unknown) => void) {
  let active = true; let unlisten: UnlistenFn | undefined;
  void subscribe((value) => { if (active) handler(value); }).then((cleanup) => { if (active) unlisten = cleanup; else cleanup(); }).catch((error) => { if (active) failed(error); });
  return () => { active = false; unlisten?.(); unlisten = undefined; };
}
export function useCaptureWorkflow() {
  useEffect(() => {
    if (!isTauri()) return;
    const failed = (error: unknown) => useCaptureStore.setState({ phase: 'idle', error: errorMessage(error, '截图通知不可用，请重新启动应用。') });
    return attachListener((handler) => getCurrentWindow().listen<unknown>('capture-workflow', (event) => handler(event.payload)), (value) => {
      if (!value || typeof value !== 'object' || !('kind' in value)) { failed(new Error('截图通知格式无效。')); return; }
      if (value.kind === 'started') useCaptureStore.setState({ phase: 'capturing', error: null, notice: null });
      else if (value.kind === 'ready' && 'capture' in value) {
        try { const info = parseCaptureInfo(value.capture); useUiStore.getState().navigate('screenshots'); void useCaptureStore.getState().acceptNative(info).then(() => useCaptureStore.getState().refreshDisplays()); }
        catch (error) { failed(error); }
      } else if (value.kind === 'failed' && 'code' in value && 'message' in value) {
        useCaptureStore.setState({ phase: 'idle', error: value.code === 'CANCELLED' ? null : errorMessage(value, '快捷键截图失败。') });
        if (value.code !== 'CANCELLED') useUiStore.getState().navigate('screenshots');
      } else failed(new Error('截图通知格式无效。'));
    }, failed);
  }, []);
  // Recover a result if it arrived before the main listener mounted (e.g. reload).
  useEffect(() => {
    if (!isTauri()) return;
    let active = true;
    void invoke<unknown>('current_capture').then((info) => { if (active && info) return useCaptureStore.getState().acceptNative(parseCaptureInfo(info)); }).catch((error) => { if (active) useCaptureStore.setState({ error: errorMessage(error, '截图状态读取失败。') }); });
    return () => { active = false; };
  }, []);
}
