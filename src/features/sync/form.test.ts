import {describe,it,expect} from 'vitest';
import {connectionIssue,normalizeConfig,sameConfig,sameConnection,syncDefaults} from './form';
import {type SyncStatus} from './api';
const config={server:'https://example.com/dav/',folder:'researchclip-test',projects:['a'.repeat(32),'b'.repeat(32)],unclassified:true};
const status:SyncStatus={config,credentialAvailable:true,pending:0,conflicts:[],job:{phase:'idle',message:'',writable:false,progress:{completed:0,total:0,uploaded:0,downloaded:0,conflicts:0,skipped:0},remoteProjects:[]}};
describe('WebDAV editable connection form',()=>{
 it('validates empty first save with an actionable field target',()=>{expect(connectionIssue(syncDefaults,'','',null)?.field).toBe('WebDAV服务器');});
 it('only reuses credentials belonging to the exact saved connection',()=>{expect(connectionIssue(config,'','',status)).toBeNull();expect(connectionIssue({...config,folder:'researchclip-other'},'','',status)?.message).toContain('尚无可用凭据');expect(sameConnection({...config,server:'https://other.example/'},config)).toBe(false);});
 it('requires paired credentials and explains unavailable OS credentials',()=>{expect(connectionIssue(config,'user','',status)?.field).toBe('WebDAV应用密码');expect(connectionIssue(config,'','secret',status)?.field).toBe('WebDAV用户名');expect(connectionIssue(config,'','',{...status,credentialAvailable:false})?.message).toContain('凭据');});
 it.each(['http://example.com/','https://user:password@example.com/','https://example.com/?token=x','https://example.com/#secret','https://example.com/no-slash'])('rejects unsafe server %s',server=>{expect(connectionIssue({...config,server},'user','secret',null)?.field).toBe('WebDAV服务器');});
 it('normalizes input and ignores project selection ordering without changing identity',()=>{expect(sameConfig(config,{...config,server:' '+config.server+' ',projects:[...config.projects].reverse()})).toBe(true);expect(normalizeConfig({...config,folder:' researchclip-test '}).folder).toBe(config.folder);});
 it('checks directory boundaries and duplicate scope IDs',()=>{expect(connectionIssue({...config,folder:'../unsafe'},'user','secret',null)?.field).toBe('WebDAV专属目录');expect(connectionIssue({...config,projects:['a'.repeat(32),'a'.repeat(32)]},'user','secret',null)).not.toBeNull();});
});
