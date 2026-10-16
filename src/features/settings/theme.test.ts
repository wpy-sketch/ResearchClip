import {describe,it,expect,vi} from 'vitest';
import {resolveTheme,observeTheme} from './theme';
describe('system theme preference and listener lifecycle',()=>{
  it('resolves explicit preferences independently of OS and follows both OS modes',()=>{expect(resolveTheme('dark',false)).toBe('dark');expect(resolveTheme('light',true)).toBe('light');expect(resolveTheme('system',true)).toBe('dark');expect(resolveTheme('system',false)).toBe('light');});
  it('subscribes once in system mode and removes the exact handler',()=>{const media={matches:false,addEventListener:vi.fn(),removeEventListener:vi.fn()},apply=vi.fn();const stop=observeTheme('system',media,apply);expect(apply).toHaveBeenLastCalledWith('light');const callback=media.addEventListener.mock.calls[0][1];media.matches=true;callback();expect(apply).toHaveBeenLastCalledWith('dark');stop();expect(media.removeEventListener).toHaveBeenCalledExactlyOnceWith('change',callback);});
  it('does not subscribe for an explicit theme',()=>{const media={matches:true,addEventListener:vi.fn(),removeEventListener:vi.fn()},apply=vi.fn();observeTheme('light',media,apply)();expect(apply).toHaveBeenCalledExactlyOnceWith('light');expect(media.addEventListener).not.toHaveBeenCalled();});
});
