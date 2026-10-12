import { invoke, isTauri } from '@tauri-apps/api/core';
import type { RuntimeStatus, WorkspaceStatus } from '../types/workspace';

export function parseWorkspaceStatus(value: unknown): WorkspaceStatus {
  if (!value || typeof value !== 'object') throw new Error('桌面端返回了无效的工作区信息');
  const record = value as Record<string, unknown>;
  const keys = ['appName', 'appVersion', 'platform', 'dataDir', 'databasePath', 'sqliteVersion'];
  if (keys.some((key) => typeof record[key] !== 'string' || !record[key])
    || !Number.isSafeInteger(record.schemaVersion) || Number(record.schemaVersion) < 1) {
    throw new Error('桌面端工作区信息与通信协议不兼容');
  }
  return value as WorkspaceStatus;
}

export async function getRuntimeStatus(): Promise<RuntimeStatus> {
  if (!isTauri()) return { mode: 'preview' };
  const result = await invoke<unknown>('get_workspace_status');
  return { mode: 'desktop', workspace: parseWorkspaceStatus(result) };
}

export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === 'string') return error;
  if (error && typeof error === 'object' && 'message' in error && typeof error.message === 'string') {
    return error.message;
  }
  return '无法读取本地工作区，请重新检查';
}
