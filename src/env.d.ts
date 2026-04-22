/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_CYBER_LINK_RUNTIME?: 'mock' | 'tauri';
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
