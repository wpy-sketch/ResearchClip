import {it,expect,vi} from 'vitest';
vi.mock('@tauri-apps/api/core',()=>({invoke:vi.fn(),isTauri:()=>false}));
import {parseZoteroSetup} from './zoteroApi';
const good={executablePath:'D:\\ResearchClip\\researchclip.exe',apiUrl:'http://127.0.0.1:23119/api/',readOnly:true,apiKeyRequired:false};
it('accepts an explicit native program path and read-only local API without requiring a cloud key',()=>expect(parseZoteroSetup(good)).toEqual(good));
it('rejects remote URLs, unknown protocols and writable contracts',()=>{for(const v of [{...good,apiUrl:'https://api.zotero.org/'},{...good,apiUrl:'file:///path'},{...good,readOnly:false},{...good,apiKeyRequired:true},{...good,executablePath:''},{...good,apiUrl:'http://127.0.0.1:99999/api/'},{...good,executablePath:'unsafe\npath'}])expect(()=>parseZoteroSetup(v)).toThrow();});
