import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';
import { fileURLToPath } from 'node:url';

const configDir = fileURLToPath(new URL('.', import.meta.url));

export default defineConfig({
  root: path.resolve(configDir, 'desktop'),
  publicDir: path.resolve(configDir, 'public'),
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(configDir, './src'),
    },
  },
  server: {
    port: 3000,
    strictPort: true,
    fs: {
      allow: [configDir],
    },
  },
  build: {
    outDir: path.resolve(configDir, 'dist-desktop'),
    emptyOutDir: true,
  },
});
