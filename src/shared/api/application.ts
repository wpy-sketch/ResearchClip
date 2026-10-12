import { getVersion } from '@tauri-apps/api/app';
import { invoke, isTauri } from '@tauri-apps/api/core';
import type { ApplicationInfo, ConnectionTestResponse } from '../types/application';

export async function getApplicationInfo(): Promise<ApplicationInfo> {
  if (!isTauri()) return { mode: 'preview', version: __APP_VERSION__ };
  const version = await getVersion();
  if (typeof version !== 'string' || !version.trim()) throw new Error('桌面端返回了无效的应用版本。');
  return { mode: 'desktop', version };
}

export function parseConnectionResponse(value: unknown): ConnectionTestResponse {
  if (!value || typeof value !== 'object') throw new Error('Rust 返回了无效的通信响应。');
  const response = value as Record<string, unknown>;
  if (typeof response.message !== 'string' || !response.message.trim()
    || typeof response.appVersion !== 'string' || !response.appVersion.trim()) {
    throw new Error('Rust 通信响应与前端协议不兼容。');
  }
  return value as ConnectionTestResponse;
}

export async function requestTestMessage(): Promise<ConnectionTestResponse> {
  if (!isTauri()) throw new Error('Rust 通信只能在 ResearchClip 桌面应用中测试。');
  return parseConnectionResponse(await invoke<unknown>('test_connection'));
}
