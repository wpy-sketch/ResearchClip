import { beforeEach, describe, expect, it } from 'vitest';
import { useUiStore } from './ui-store';

beforeEach(() => useUiStore.setState(useUiStore.getInitialState(), true));

describe('workspace UI session state', () => {
  it('starts in the dark grid view with the guide visible', () => {
    expect(useUiStore.getState()).toMatchObject({ activeView: 'screenshots', theme: 'dark', viewMode: 'grid', showWelcome: true, overlay: null, searchQuery: '' });
  });

  it('keeps appearance and view choice while navigating and closes overlays', () => {
    const actions = useUiStore.getState();
    actions.setTheme('light'); actions.setViewMode('list'); actions.openSearch();
    actions.navigate('favorites');
    expect(useUiStore.getState()).toMatchObject({ activeView: 'favorites', theme: 'light', viewMode: 'list', overlay: null });
    actions.toggleTheme();
    expect(useUiStore.getState().theme).toBe('dark');
  });

  it('can restore the dismissed guide from another page', () => {
    const actions = useUiStore.getState();
    actions.dismissWelcome(); actions.navigate('tags'); actions.openCapture(); actions.openGuide();
    expect(useUiStore.getState()).toMatchObject({ activeView: 'guide', showWelcome: true, overlay: null });
  });

  it('keeps search text in the session while switching a single overlay', () => {
    const actions = useUiStore.getState();
    actions.openSearch(); actions.setSearchQuery('质谱研究'); actions.openCapture();
    expect(useUiStore.getState()).toMatchObject({ overlay: 'capture', searchQuery: '质谱研究' });
    actions.closeOverlay(); actions.setSearchQuery('');
    expect(useUiStore.getState()).toMatchObject({ overlay: null, searchQuery: '' });
  });
});
