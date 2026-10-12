import { beforeEach, describe, expect, it, vi } from 'vitest';
import { getApplicationInfo, requestTestMessage } from '../shared/api/application';
import type { ApplicationInfo, ConnectionTestResponse } from '../shared/types/application';
import { useAppStore } from './store';

vi.mock('../shared/api/application', () => ({ getApplicationInfo: vi.fn(), requestTestMessage: vi.fn() }));
const info: ApplicationInfo = { mode: 'desktop', version: '0.1.0' };
const response: ConnectionTestResponse = { message: 'Rust 后端连接成功', appVersion: '0.1.0' };

beforeEach(() => {
  vi.resetAllMocks();
  useAppStore.setState({ info: null, initializationPhase: 'idle', initializationError: null,
    connectionPhase: 'idle', message: null, connectionError: null });
});

describe('welcome window lifecycle', () => {
  it('initializes once under concurrent calls and StrictMode', async () => {
    let resolve!: (value: ApplicationInfo) => void;
    vi.mocked(getApplicationInfo).mockReturnValue(new Promise((done) => { resolve = done; }));
    const first = useAppStore.getState().initialize();
    await useAppStore.getState().initialize();
    expect(getApplicationInfo).toHaveBeenCalledTimes(1);
    resolve(info);
    await first;
    await useAppStore.getState().initialize();
    expect(getApplicationInfo).toHaveBeenCalledTimes(1);
    expect(useAppStore.getState()).toMatchObject({ info, initializationPhase: 'ready' });
  });
  it('reports initialization errors without inventing a desktop version', async () => {
    vi.mocked(getApplicationInfo).mockRejectedValue('版本不可用');
    await useAppStore.getState().initialize();
    expect(useAppStore.getState()).toMatchObject({ info: null, initializationPhase: 'error', initializationError: '版本不可用' });
  });
  it('prevents native requests in preview mode', async () => {
    useAppStore.setState({ info: { mode: 'preview', version: '0.1.0' } });
    await useAppStore.getState().testConnection();
    expect(requestTestMessage).not.toHaveBeenCalled();
    expect(useAppStore.getState().message).toBeNull();
  });
  it('prevents duplicate requests while a test is pending', async () => {
    useAppStore.setState({ info });
    let resolve!: (value: ConnectionTestResponse) => void;
    vi.mocked(requestTestMessage).mockReturnValue(new Promise((done) => { resolve = done; }));
    const first = useAppStore.getState().testConnection();
    await useAppStore.getState().testConnection();
    expect(requestTestMessage).toHaveBeenCalledTimes(1);
    expect(useAppStore.getState().connectionPhase).toBe('loading');
    resolve(response);
    await first;
    expect(useAppStore.getState()).toMatchObject({ connectionPhase: 'success', message: response.message });
  });
  it('allows retry after a structured backend error', async () => {
    useAppStore.setState({ info });
    vi.mocked(requestTestMessage).mockRejectedValueOnce({ message: '通信错误' }).mockResolvedValueOnce(response);
    await useAppStore.getState().testConnection();
    expect(useAppStore.getState()).toMatchObject({ connectionPhase: 'error', connectionError: '通信错误', message: null });
    await useAppStore.getState().testConnection();
    expect(useAppStore.getState()).toMatchObject({ connectionPhase: 'success', connectionError: null, message: response.message });
  });
  it('rejects a Rust/frontend version mismatch', async () => {
    useAppStore.setState({ info });
    vi.mocked(requestTestMessage).mockResolvedValue({ ...response, appVersion: '0.2.0' });
    await useAppStore.getState().testConnection();
    expect(useAppStore.getState().connectionPhase).toBe('error');
    expect(useAppStore.getState().connectionError).toContain('版本不一致');
    expect(useAppStore.getState().message).toBeNull();
  });
  it('clears a stale success message when the next request fails', async () => {
    useAppStore.setState({ info });
    vi.mocked(requestTestMessage).mockResolvedValueOnce(response).mockRejectedValueOnce(undefined);
    await useAppStore.getState().testConnection();
    await useAppStore.getState().testConnection();
    expect(useAppStore.getState()).toMatchObject({ connectionPhase: 'error', message: null, connectionError: 'Rust 通信失败，请重试。' });
  });
});
