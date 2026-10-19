import {defineConfig} from 'vite';
import {copyFileSync} from 'node:fs';
export default defineConfig({plugins:[{name:'mv3-manifest',closeBundle(){copyFileSync('manifest.json','dist/manifest.json');}}],build:{target:'chrome120',rollupOptions:{input:'popup.html'}}});
