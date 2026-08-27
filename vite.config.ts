import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';
import { fileURLToPath } from 'node:url';
import { VitePWA } from 'vite-plugin-pwa';

const configDir = fileURLToPath(new URL('.', import.meta.url));

export default defineConfig({
  plugins: [
    react(),
    VitePWA({
      registerType: 'autoUpdate',
      includeAssets: ['favicon.svg', 'icon-192.png', 'icon-512.png'],
      manifest: {
        name: 'Ley — Local-first second brain',
        short_name: 'Ley',
        description: 'A local-first second brain for connected Markdown notes',
        start_url: '/app',
        scope: '/',
        display: 'standalone',
        background_color: '#0d0f12',
        theme_color: '#0d0f12',
        categories: ['productivity', 'utilities'],
        icons: [
          { src: '/icon-192.png', sizes: '192x192', type: 'image/png', purpose: 'any' },
          { src: '/icon-512.png', sizes: '512x512', type: 'image/png', purpose: 'any' },
          { src: '/favicon.svg', sizes: 'any', type: 'image/svg+xml', purpose: 'any' },
        ],
      },
      workbox: {
        navigateFallback: '/index.html',
        globPatterns: ['**/*.{js,css,html,svg,woff2,png,ico}'],
      },
    }),
  ],
  resolve: {
    alias: {
      '@': path.resolve(configDir, './src'),
    },
  },
  server: {
    port: 3000,
    strictPort: true,
    open: true,
    fs: {
      // Reference projects under ref/ have their own configs and shouldn't
      // be scanned by Vite when resolving imports.
      allow: ['..'],
      deny: ['**/ref/**'],
    },
  },
  build: {
    // Reference projects under ref/ shouldn't be scanned during build either.
    rollupOptions: {
      input: path.resolve(configDir, 'index.html'),
    },
  },
});
