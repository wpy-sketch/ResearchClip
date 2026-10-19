import {describe,it,expect} from 'vitest';
import {parseSyncConfig,parseSyncStatus,syncRunning} from './api';
const id='a'.repeat(32),hash='b'.repeat(64);
const config={server:'https://example.org/dav/',folder:'researchclip-sync-v2',projects:[id],unclassified:false};
const status={config,credentialAvailable:true,pending:1,conflicts:[{id,resourceKey:`screenshot/${id}`,reason:'双方修改',localManifest:hash,remoteManifest:hash}],job:{phase:'conflicts',message:'需要处理',writable:true,progress:{completed:1,total:1,uploaded:0,downloaded:0,conflicts:1,skipped:0},remoteProjects:[{id,name:'科研'}]}};
describe('WebDAV bounded IPC',()=>{
 it('parses scoped config and progress without a credential field',()=>{expect(parseSyncStatus(status).conflicts[0].resourceKey).toBe(`screenshot/${id}`);expect(parseSyncConfig(config)).toEqual(config);expect(syncRunning('syncing')).toBe(true);expect(syncRunning('stopped')).toBe(false);expect(parseSyncStatus(status)).not.toHaveProperty('password');});
 it('matches Rust directory bounds exactly',()=>{expect(parseSyncConfig({...config,folder:'researchclip-a'}).folder).toHaveLength(14);expect(parseSyncConfig({...config,folder:'researchclip-'+ 'a'.repeat(67)}).folder).toHaveLength(80);expect(()=>parseSyncConfig({...config,folder:'researchclip-'+ 'a'.repeat(68)})).toThrow();});
 it('rejects unsafe config and resource identities',()=>{for(const value of [{...config,server:'http://example.org/'},{...config,folder:'../zotero'},{...config,projects:['../../x']}])expect(()=>parseSyncConfig(value)).toThrow();expect(()=>parseSyncStatus({...status,conflicts:[{...status.conflicts[0],resourceKey:'screenshot/../../outside'}]})).toThrow();});
 it('rejects malformed jobs, hashes and unbounded collections',()=>{expect(()=>parseSyncStatus({...status,job:{...status.job,phase:'unknown'}})).toThrow();expect(()=>parseSyncStatus({...status,job:{...status.job,progress:{...status.job.progress,total:-1}}})).toThrow();expect(()=>parseSyncStatus({...status,conflicts:[{...status.conflicts[0],remoteManifest:'bad'}]})).toThrow();expect(()=>parseSyncStatus({...status,conflicts:Array.from({length:5001},()=>status.conflicts[0])})).toThrow();});
});
