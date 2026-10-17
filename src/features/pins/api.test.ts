import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { renderComposite } from '../editor/export';
import { createPin, loadPin, parsePinInfo, pinAction, pinEditor } from './api';
import type { EditorSession } from '../editor/types';
vi.mock('@tauri-apps/api/core',()=>({invoke:vi.fn(),isTauri:vi.fn()}));
vi.mock('../editor/export',()=>({renderComposite:vi.fn()}));
const png=()=>new Blob([Uint8Array.from(atob('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAC0lEQVR42mP8/x8AAwMCAO+a0h8AAAAASUVORK5CYII='),c=>c.charCodeAt(0))],{type:'image/png'});
beforeEach(()=>{vi.resetAllMocks();vi.mocked(isTauri).mockReturnValue(true);});
describe('desktop pins IPC',()=>{
  it('uploads independent PNG bytes, never a file path',async()=>{vi.mocked(invoke).mockResolvedValue('pin-12');expect(await createPin(png())).toBe('pin-12');expect(invoke).toHaveBeenCalledWith('create_pin',expect.any(ArrayBuffer));});
  it('rejects browser, invalid image and invalid native identifiers',async()=>{vi.mocked(isTauri).mockReturnValue(false);await expect(createPin(png())).rejects.toThrow('桌面');vi.mocked(isTauri).mockReturnValue(true);await expect(createPin(new Blob(['bad']))).rejects.toThrow();vi.mocked(invoke).mockResolvedValue('main');await expect(createPin(png())).rejects.toThrow('标识');});
  it('pins the existing annotation composite rather than original image',async()=>{const original=new Blob(['original']),composite=png(),document={schemaVersion:1,image:{widthPx:320,heightPx:200},objects:[]} as const;const session={image:{blob:original},document} as unknown as EditorSession;vi.mocked(renderComposite).mockResolvedValue(composite);vi.mocked(invoke).mockResolvedValue('pin-1');await pinEditor(session);expect(renderComposite).toHaveBeenCalledWith(original,document);expect(invoke).toHaveBeenCalledWith('create_pin',await composite.arrayBuffer());});
  it('scopes actions to caller without accepting another label or path',async()=>{vi.mocked(invoke).mockResolvedValue({widthPx:320,heightPx:200,opacity:50});expect((await pinAction('opacity',50))?.opacity).toBe(50);expect(invoke).toHaveBeenCalledWith('pin_action',{action:'opacity',value:50});vi.mocked(invoke).mockResolvedValue(null);expect(await pinAction('close')).toBeNull();});
  it('rejects bad native metadata and reports clipboard errors',async()=>{for(const value of [null,{}, {widthPx:0,heightPx:1,opacity:100},{widthPx:1,heightPx:1,opacity:NaN}])expect(()=>parsePinInfo(value)).toThrow();vi.mocked(invoke).mockRejectedValue(new Error('剪贴板不可用'));await expect(pinAction('copy')).rejects.toThrow('剪贴板');});
  it('loads validated binary image as owned blob URL',async()=>{vi.mocked(invoke).mockResolvedValue(await png().arrayBuffer());const url=await loadPin();expect(url.startsWith('blob:')).toBe(true);URL.revokeObjectURL(url);vi.mocked(invoke).mockResolvedValue(new ArrayBuffer(1));await expect(loadPin()).rejects.toThrow();});
});
