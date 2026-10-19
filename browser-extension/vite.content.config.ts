import {defineConfig} from 'vite';
export default defineConfig({build:{target:'chrome120',emptyOutDir:false,lib:{entry:'src/content.ts',name:'ResearchClipContent',formats:['iife'],fileName:()=> 'content.js'}}});
