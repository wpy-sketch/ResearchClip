import { create } from 'zustand';

export type NavigationView = 'screenshots' | 'favorites' | 'tags' | 'settings' | 'editor' | 'card' | 'compare' | 'guide' | 'graph';
export type Theme = 'dark' | 'light';
export type ViewMode = 'grid' | 'list';

/** UI only. No persistence middleware, localStorage, file writes or database calls. */
interface UiState {
  activeView: NavigationView;
  theme: Theme;
  viewMode: ViewMode;
  fontFamily: 'system' | 'sans' | 'serif';
  fontSize: number;
  showWelcome: boolean;
  overlay: 'search' | 'capture' | null;
  searchQuery: string;
  windowThemeError: string | null;
  navigate: (view: NavigationView) => void;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
  setViewMode: (mode: ViewMode) => void;
  dismissWelcome: () => void;
  openGuide: () => void;
  openSearch: () => void;
  openCapture: () => void;
  closeOverlay: () => void;
  setSearchQuery: (query: string) => void;
  setWindowThemeError: (error: string | null) => void;
}

export const useUiStore = create<UiState>((set) => ({
  activeView: 'screenshots', theme: 'dark', viewMode: 'grid', fontFamily: 'system', fontSize: 16, showWelcome: true,
  overlay: null, searchQuery: '', windowThemeError: null,
  navigate: (activeView) => set({ activeView, overlay: null }),
  setTheme: (theme) => set({ theme }),
  toggleTheme: () => set((state) => ({ theme: state.theme === 'dark' ? 'light' : 'dark' })),
  setViewMode: (viewMode) => set({ viewMode }),
  dismissWelcome: () => set({ showWelcome: false }),
  openGuide: () => set({ activeView: 'guide', showWelcome: true, overlay: null }),
  openSearch: () => set({ overlay: 'search' }),
  openCapture: () => set({ overlay: 'capture' }),
  closeOverlay: () => set({ overlay: null }),
  setSearchQuery: (searchQuery) => set({ searchQuery }),
  setWindowThemeError: (windowThemeError) => set({ windowThemeError }),
}));
