import { beforeEach, describe, expect, it, vi } from 'vitest';
import { getVersion } from '@tauri-apps/api/app';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { getApplicationInfo, parseConnectionResponse, requestTestMessage } from './application';

vi.mock('@tauri-apps/api/app', () => ({ getVersion: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: vi.fn() }));
const response = { message: 'ResearchClip：Rust 后端连接成功。', appVersion: '0.1.0' };

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(isTauri).mockReturnValue(true);
  vi.mocked(getVersion).mockResolvedValue('0.1.0');
});

describe('application IPC boundary', () => {
  it('gets the actual desktop version through the official API', async () => {
    expect(await getApplicationInfo()).toEqual({ mode: 'desktop', version: '0.1.0' });
    expect(getVersion).toHaveBeenCalledTimes(1);
  });
  it('shows the build version in preview without calling native APIs', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    expect(await getApplicationInfo()).toEqual({ mode: 'preview', version: __APP_VERSION__ });
    expect(getVersion).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalled();
  });
  it('rejects an empty native version', async () => {
    vi.mocked(getVersion).mockResolvedValue('');
    await expect(getApplicationInfo()).rejects.toThrow('无效的应用版本');
  });
  it('uses the registered Rust command and its validated response', async () => {
    vi.mocked(invoke).mockResolvedValue(response);
    expect(await requestTestMessage()).toEqual(response);
    expect(invoke).toHaveBeenCalledWith('test_connection');
  });
  it('cannot fake a Rust message in browser preview', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    await expect(requestTestMessage()).rejects.toThrow('桌面应用');
    expect(invoke).not.toHaveBeenCalled();
  });
  it('preserves native command failures for the UI', async () => {
    const error = { code: 'IPC_FAILED', message: '通信失败' };
    vi.mocked(invoke).mockRejectedValue(error);
    await expect(requestTestMessage()).rejects.toEqual(error);
  });
  it.each([null, {}, 'success', { ...response, message: '' },
    { ...response, appVersion: 1 }, { message: response.message, app_version: '0.1.0' }])(
    'rejects an incompatible response: %j', (value) => {
      expect(() => parseConnectionResponse(value)).toThrow();
    },
  );
});
