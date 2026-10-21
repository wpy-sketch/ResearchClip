import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import packageInfo from './package.json' with { type: 'json' };
import { fileURLToPath } from 'node:url';

export default defineConfig({
  plugins: [react(), tailwindcss()],
  // CommonMark's browser entity decoder requires document; use its installed
  // pure implementation so the same parser also runs in a DOM-free Worker.
  resolve: { alias: { 'decode-named-character-reference': fileURLToPath(import.meta.resolve('decode-named-character-reference')) } },
  define: { __APP_VERSION__: JSON.stringify(packageInfo.version) },
  clearScreen: false,
  server: {
    host: '127.0.0.1', port: 1420, strictPort: true,
    watch: { ignored: ['**/src-tauri/**', '**/.tools/**', '**/artifacts/**'] },
  },
  build: { assetsInlineLimit: (filePath) => /\.(woff2?|ttf|otf)$/i.test(filePath) ? false : undefined, target: ['es2022', 'chrome105', 'safari15.4'] },
  test: { environment: 'node', include: ['src/**/*.test.{ts,tsx}'], clearMocks: true },
});
