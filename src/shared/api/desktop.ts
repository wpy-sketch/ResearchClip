import {invoke} from '@tauri-apps/api/core';
export type DesktopAction='open'|'settings'|'hide'|'exit'|'region'|'fullscreen';
export interface DesktopSnapshot {trayAvailable:boolean;trayCount:number;exitPending:boolean;error:string|null}
export async function getDesktopStatus():Promise<DesktopSnapshot>{const value=await invoke<unknown>('desktop_status');if(!value||typeof value!=='object'||!('trayAvailable'in value)||typeof value.trayAvailable!=='boolean'||!('trayCount'in value)||value.trayCount!==Number(value.trayAvailable)||!('exitPending'in value)||typeof value.exitPending!=='boolean'||!('error'in value)||(value.error!==null&&typeof value.error!=='string'))throw new Error('桌面状态响应无效。');return value as DesktopSnapshot;}
export async function desktopAction(action:DesktopAction){await invoke('desktop_action',{action});}
export async function completeDesktopExit(approved:boolean){await invoke('complete_desktop_exit',{approved});}
