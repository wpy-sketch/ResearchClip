import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import packageInfo from './package.json' with { type: 'json' };

export default defineConfig({
  plugins: [react(), tailwindcss()],
  define: { __APP_VERSION__: JSON.stringify(packageInfo.version) },
  clearScreen: false,
  server: {
    host: '127.0.0.1', port: 1420, strictPort: true,
    watch: { ignored: ['**/src-tauri/**', '**/.tools/**', '**/artifacts/**'] },
  },
  build: { target: ['es2022', 'chrome105', 'safari15.4'] },
  test: { environment: 'node', include: ['src/**/*.test.ts'], clearMocks: true },
});
