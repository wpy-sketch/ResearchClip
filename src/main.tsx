import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { PinView } from './features/pins/PinView';
import { App } from './app/App';
import './styles.css';
import { CaptureOverlay } from './features/capture/CaptureOverlay';

const desktopPin = new URLSearchParams(location.search).has('desktopPin');
if (desktopPin) document.documentElement.dataset.desktopPin = 'true';
const overlaySession = new URLSearchParams(location.search).get('captureOverlay');
if (overlaySession) document.documentElement.dataset.captureOverlay = 'true';
createRoot(document.getElementById('root')!).render(
  // Overlay owns a one-shot frozen frame. A separate entry avoids StrictMode effect replay.
  desktopPin ? <PinView /> : overlaySession ? <CaptureOverlay sessionId={overlaySession} /> : <StrictMode><App /></StrictMode>,
);
