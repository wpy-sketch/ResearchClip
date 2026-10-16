import {useEffect} from 'react';
import {isTauri} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import {useDesktopStore} from './store';
import {useUiStore} from '../../app/ui-store';
export function connectDesktopEvents(){
  if(!isTauri())return()=>{};let active=true;let stopAction:(()=>void)|undefined,stopNotice:(()=>void)|undefined;
  void (async()=>{
    try{
      const action=await listen<string>('desktop-action',event=>{if(!active)return;if(event.payload==='settings')useUiStore.getState().navigate('settings');if(event.payload==='exit')void useDesktopStore.getState().requestQuit();});
      if(!active){action();return;}stopAction=action;
      const notice=await listen<string>('desktop-notice',event=>{if(active&&typeof event.payload==='string')useDesktopStore.setState({error:event.payload});});
      if(!active){notice();return;}stopNotice=notice;
      await useDesktopStore.getState().refresh();
    }catch{if(active)useDesktopStore.setState({error:'桌面事件监听失败，请保留未保存内容并重新启动。'});}
  })();
  return()=>{active=false;stopAction?.();stopNotice?.();};
}
export function useDesktopExperience(){useEffect(connectDesktopEvents,[]);}
