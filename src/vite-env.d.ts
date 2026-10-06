/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_LEY_UPDATER_ENABLED?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
