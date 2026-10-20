import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { PinView } from './features/pins/PinView';
import {ScrollController} from './features/capture/ScrollCapture';
import { App } from './app/App';
import './styles.css';
import { WindowPicker } from './features/capture/WindowPicker';
import { CaptureOverlay } from './features/capture/CaptureOverlay';

const windowPicker = new URLSearchParams(location.search).get('windowPicker');
if(windowPicker)document.documentElement.dataset.windowPicker='true';
const scrollControl = new URLSearchParams(location.search).get('scrollControl');
if(scrollControl)document.documentElement.dataset.scrollControl='true';
const desktopPin = new URLSearchParams(location.search).has('desktopPin');
if (desktopPin) document.documentElement.dataset.desktopPin = 'true';
const overlaySession = new URLSearchParams(location.search).get('captureOverlay');
if (overlaySession) document.documentElement.dataset.captureOverlay = 'true';
createRoot(document.getElementById('root')!).render(
  // Overlay owns a one-shot frozen frame. A separate entry avoids StrictMode effect replay.
  windowPicker ? <WindowPicker sessionId={windowPicker}/> : scrollControl ? <ScrollController sessionId={scrollControl}/> : desktopPin ? <PinView /> : overlaySession ? <CaptureOverlay sessionId={overlaySession} /> : <StrictMode><App /></StrictMode>,
);
