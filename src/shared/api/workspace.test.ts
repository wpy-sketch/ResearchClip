import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { errorMessage, getRuntimeStatus, parseWorkspaceStatus } from './workspace';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: vi.fn() }));
const workspace = {
  appName: 'ResearchClip', appVersion: '0.1.0', platform: 'windows',
  dataDir: 'C:/研究资料', databasePath: 'C:/研究资料/researchclip.sqlite3',
  schemaVersion: 1, sqliteVersion: '3.50.2',
};

describe('workspace IPC boundary', () => {
  beforeEach(() => { vi.mocked(isTauri).mockReturnValue(true); });

  it('browser preview does not invoke Rust or invent database metadata', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    expect(await getRuntimeStatus()).toEqual({ mode: 'preview' });
    expect(invoke).not.toHaveBeenCalled();
  });
  it('uses the registered command and validates its native response', async () => {
    vi.mocked(invoke).mockResolvedValue(workspace);
    expect(await getRuntimeStatus()).toEqual({ mode: 'desktop', workspace });
    expect(invoke).toHaveBeenCalledWith('get_workspace_status');
  });
  it('does not report ready when Rust rejects the query', async () => {
    const error = { code: 'STORAGE_READ', message: 'database unavailable' };
    vi.mocked(invoke).mockRejectedValue(error);
    await expect(getRuntimeStatus()).rejects.toEqual(error);
    expect(errorMessage(error)).toBe('database unavailable');
  });
  it.each([null, {}, { ...workspace, dataDir: '' }, { ...workspace, schemaVersion: 0 },
    { ...workspace, schemaVersion: 1.5 }, { ...workspace, schemaVersion: '1' }])(
    'rejects incompatible IPC diagnostics: %j', (value) => {
      expect(() => parseWorkspaceStatus(value)).toThrow();
    },
  );
  it('keeps useful error messages and handles unknown rejection values', () => {
    expect(errorMessage(new Error('failure'))).toBe('failure');
    expect(errorMessage('plain failure')).toBe('plain failure');
    expect(errorMessage(undefined)).toBe('无法读取本地工作区，请重新检查');
  });
});
