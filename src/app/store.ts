import { create } from 'zustand';
import { getApplicationInfo, requestTestMessage } from '../shared/api/application';
import { errorMessage } from '../shared/api/errors';
import type { ApplicationInfo } from '../shared/types/application';

interface AppState {
  info: ApplicationInfo | null;
  initializationPhase: 'idle' | 'loading' | 'ready' | 'error';
  initializationError: string | null;
  connectionPhase: 'idle' | 'loading' | 'success' | 'error';
  message: string | null;
  connectionError: string | null;
  initialize: () => Promise<void>;
  testConnection: () => Promise<void>;
}

export const useAppStore = create<AppState>((set, get) => ({
  info: null,
  initializationPhase: 'idle',
  initializationError: null,
  connectionPhase: 'idle',
  message: null,
  connectionError: null,
  initialize: async () => {
    if (['loading', 'ready'].includes(get().initializationPhase)) return;
    set({ initializationPhase: 'loading', initializationError: null });
    try {
      set({ info: await getApplicationInfo(), initializationPhase: 'ready' });
    } catch (error) {
      set({ initializationPhase: 'error', initializationError: errorMessage(error, '无法读取应用版本，请重启应用。') });
    }
  },
  testConnection: async () => {
    if (get().connectionPhase === 'loading' || get().info?.mode !== 'desktop') return;
    set({ connectionPhase: 'loading', message: null, connectionError: null });
    try {
      const response = await requestTestMessage();
      if (response.appVersion !== get().info?.version) throw new Error('前后端应用版本不一致，请检查版本配置。');
      set({ connectionPhase: 'success', message: response.message });
    } catch (error) {
      set({ connectionPhase: 'error', connectionError: errorMessage(error, 'Rust 通信失败，请重试。') });
    }
  },
}));
