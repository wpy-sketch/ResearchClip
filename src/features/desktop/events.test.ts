import {beforeEach,describe,it,expect,vi} from 'vitest';
import {connectDesktopEvents} from './use-desktop-experience';
const mock=vi.hoisted(()=>({isTauri:vi.fn(()=>true),listen:vi.fn(),refresh:vi.fn(),requestQuit:vi.fn(),navigate:vi.fn(),setState:vi.fn()}));
vi.mock('@tauri-apps/api/core',()=>mock);vi.mock('@tauri-apps/api/event',()=>mock);
vi.mock('./store',()=>({useDesktopStore:{getState:()=>mock,setState:mock.setState}}));vi.mock('../../app/ui-store',()=>({useUiStore:{getState:()=>mock}}));
beforeEach(()=>{vi.clearAllMocks();mock.isTauri.mockReturnValue(true);});
describe('single desktop listener pair and unmount cleanup',()=>{
  it('removes an action listener that resolves after unmount and does not install another',async()=>{let finish:(stop:()=>void)=>void=()=>{};const stop=vi.fn();mock.listen.mockImplementation(()=>new Promise(resolve=>{finish=resolve;}));const cleanup=connectDesktopEvents();cleanup();finish(stop);await Promise.resolve();expect(stop).toHaveBeenCalledOnce();expect(mock.listen).toHaveBeenCalledOnce();expect(mock.refresh).not.toHaveBeenCalled();});
  it('routes only known actions and removes both handlers on repeated mount',async()=>{const stop=vi.fn();mock.listen.mockResolvedValue(stop);const cleanup=connectDesktopEvents();await vi.waitFor(()=>expect(mock.refresh).toHaveBeenCalledOnce());const callback=mock.listen.mock.calls[0][1];callback({payload:'settings'});callback({payload:'exit'});callback({payload:'unexpected'});expect(mock.navigate).toHaveBeenCalledExactlyOnceWith('settings');expect(mock.requestQuit).toHaveBeenCalledOnce();cleanup();expect(stop).toHaveBeenCalledTimes(2);callback({payload:'exit'});expect(mock.requestQuit).toHaveBeenCalledOnce();});
});
