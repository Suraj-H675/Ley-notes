import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';
import { VitePWA } from 'vite-plugin-pwa';

export default defineConfig({
  plugins: [
    react(),
    VitePWA({
      registerType: 'autoUpdate',
      includeAssets: ['favicon.svg'],
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
          { src: '/favicon.svg', sizes: 'any', type: 'image/svg+xml', purpose: 'any' },
          { src: '/favicon.svg', sizes: 'any', type: 'image/svg+xml', purpose: 'maskable' },
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
      '@': path.resolve(__dirname, './src'),
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
      input: path.resolve(__dirname, 'index.html'),
    },
  },
});
