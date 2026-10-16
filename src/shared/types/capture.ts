export interface DisplayInfo {
  id: string;
  name: string;
  widthPx: number;
  heightPx: number;
  xPx: number;
  yPx: number;
  isPrimary: boolean;
}

export interface CapturePreview {
  url: string;
  displayId: string;
  widthPx: number;
  heightPx: number;
  mode?: 'fullscreen' | 'region';
  savedPath?: string;
  captureId?: string;
  blob?: Blob;
  library?: {id:string;revision:number;title:string};
}

export interface OverlayInfo { sessionId: string; display: DisplayInfo }

export interface CaptureInfo { captureId: string; displayId: string; widthPx: number; heightPx: number; mode: 'fullscreen' | 'region' }
