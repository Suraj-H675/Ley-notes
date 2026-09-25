import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import '@fontsource-variable/geist/wght.css';
import '@fontsource-variable/geist-mono/wght.css';
import './shared/styles/index.css';
import { App } from './app/App';

const rootEl = document.getElementById('root');
if (!rootEl) throw new Error('No #root element found in desktop index.html');

createRoot(rootEl).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
