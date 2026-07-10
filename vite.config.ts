import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 3000,
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
