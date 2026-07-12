/**
 * Entry point. Mounts App into #root.
 */

import { lazy, StrictMode, Suspense } from 'react';
import { createRoot } from 'react-dom/client';
import '@fontsource-variable/geist/wght.css';
import '@fontsource-variable/geist-mono/wght.css';
import './shared/styles/index.css';

const isNative = '__TAURI_INTERNALS__' in window;
const isLanding = !isNative && window.location.pathname === '/';
const Root = lazy(() => isLanding
  ? import('./website/LandingPage').then((module) => ({ default: module.LandingPage }))
  : import('./app/App').then((module) => ({ default: module.App })));

const rootEl = document.getElementById('root');
if (!rootEl) throw new Error('No #root element found in index.html');

createRoot(rootEl).render(
  <StrictMode>
    <Suspense fallback={<div className="h-full bg-background" />}><Root /></Suspense>
  </StrictMode>,
);
