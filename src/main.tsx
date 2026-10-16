import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './app/App';
import './styles.css';
import { CaptureOverlay } from './features/capture/CaptureOverlay';

const overlaySession = new URLSearchParams(location.search).get('captureOverlay');
if (overlaySession) document.documentElement.dataset.captureOverlay = 'true';
createRoot(document.getElementById('root')!).render(
  // Overlay owns a one-shot frozen frame. A separate entry avoids StrictMode effect replay.
  overlaySession ? <CaptureOverlay sessionId={overlaySession} /> : <StrictMode><App /></StrictMode>,
);
