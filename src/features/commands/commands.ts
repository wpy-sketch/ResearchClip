import {executeUserCommand} from './platform/runtime';
import {useUiStore} from '../../app/ui-store';
import {useDocumentStore} from '../document/store';
import {useScrollStore} from '../capture/scroll-store';
import {useOcrStore} from '../ocr/store';
import {useMathStore} from '../math-ocr/store';
import {currentImage} from './runtime';
import {useCaptureStore} from '../capture/store';
import {openCaptureEditor} from '../../app/open-editor';
import type {Searchable} from './model';
export interface Command extends Searchable {execute:()=>void|Promise<void>}
const requiredImage=async()=>{const image=await currentImage();if(!image)throw Error('请先在素材库打开图片编辑器或素材卡片，或完成一次截图，再执行此命令。');return image;};
export const commands:Command[]=[
 {id:'capture',title:'新建截图',keywords:'capture screenshot 区域 全屏',execute:()=>useUiStore.getState().openCapture()},
 {id:'scroll',title:'滚动长截图',keywords:'scroll 长网页',execute:()=>useScrollStore.getState().open()},
 {id:'library',title:'打开素材库',keywords:'library 图片 卡片',execute:()=>useUiStore.getState().navigate('screenshots')},
 {id:'markdown',title:'新建 Markdown',keywords:'new document 笔记 文档',execute:()=>useDocumentStore.getState().request('blank')},
 {id:'recent',title:'打开最近文档',keywords:'recent document markdown 最近',execute:()=>{}},
 {id:'ocr',title:'识别图片文字',keywords:'ocr text 文字识别',execute:async()=>{const image=await requiredImage(),s=useOcrStore.getState();if(s.source||s.phase!=='idle')throw Error('已有文字识别面板，请先处理或关闭。');await s.open(async()=>image.blob,image.name);if(useOcrStore.getState().error)throw Error(useOcrStore.getState().error!);}},
 {id:'formula',title:'识别数学公式',keywords:'formula math latex 公式识别',execute:async()=>{const image=await requiredImage(),s=useMathStore.getState();if(s.source||s.phase!=='idle')throw Error('已有公式识别面板，请先处理或关闭。');await s.open(async()=>image.blob,image.name,image.screenshotId);if(useMathStore.getState().error)throw Error(useMathStore.getState().error!);}},
 {id:'insert-image',title:'将截图插入 Markdown',keywords:'insert image 图片 插入',execute:async()=>{const image=await requiredImage(),s=useDocumentStore.getState();if(!s.document)throw Error('请先新建或打开 Markdown，图片尚未插入。');if(useUiStore.getState().activeView!=='editor'&&!image.screenshotId){const preview=useCaptureStore.getState().preview;if(preview)openCaptureEditor(preview);}const ok=useUiStore.getState().activeView==='editor'?await s.insertEditor():image.screenshotId?await s.addImages([image.screenshotId]):false;if(!ok)throw Error(useDocumentStore.getState().error??'图片未插入，请核对当前文档与图片。');}},
 {id:'open-resource-image',title:'打开当前素材面板',keywords:'image resource 图片 编辑器 打开',execute:async()=>{const image=await requiredImage();if(!image.screenshotId)throw Error('请先将当前图片存入素材库，再使用资源命令打开。');await executeUserCommand('document.open',{resource:{kind:'screenshot',id:image.screenshotId}});}},
 {id:'settings',title:'打开设置',keywords:'settings preferences 配置',execute:()=>useUiStore.getState().navigate('settings')},
];
