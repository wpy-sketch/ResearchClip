import {beforeEach,describe,it,expect,vi} from 'vitest';
vi.mock('../links/api',()=>({linkCall:vi.fn()}));
import {linkCall} from '../links/api';
import {resolveAnchor,saveAnchor,anchorPage} from './api';
const a={anchorId:'a'.repeat(32),screenshotId:'b'.repeat(32),imageRevision:'c'.repeat(64),x:5,y:8,width:20,height:10,title:'区域',description:'说明',revision:1};
beforeEach(()=>vi.mocked(linkCall).mockReset());
describe('scoped anchor IPC contracts',()=>{
 it('requires resolve responses to match both requested IDs',async()=>{vi.mocked(linkCall).mockResolvedValue(a);expect(await resolveAnchor(a.screenshotId,a.anchorId)).toEqual(a);await expect(resolveAnchor('d'.repeat(32),a.anchorId)).rejects.toThrow('目标不匹配');await expect(resolveAnchor(a.screenshotId,'d'.repeat(32))).rejects.toThrow('目标不匹配');});
 it('rejects a save response for another original or screenshot',async()=>{const input={...a,anchorId:null,revision:null};vi.mocked(linkCall).mockResolvedValue(a);expect(await saveAnchor(input)).toEqual(a);vi.mocked(linkCall).mockResolvedValue({...a,imageRevision:'e'.repeat(64)});await expect(saveAnchor(input)).rejects.toThrow('不一致');});
 it('does not accept cross-screenshot, duplicate or out-of-image page entries',async()=>{const page={imageRevision:a.imageRevision,widthPx:100,heightPx:80,anchors:[a]};vi.mocked(linkCall).mockResolvedValue(page);expect((await anchorPage(a.screenshotId)).anchors).toEqual([a]);for(const anchors of [[a,a],[{...a,screenshotId:'d'.repeat(32)}],[{...a,x:99}]]){vi.mocked(linkCall).mockResolvedValue({...page,anchors});await expect(anchorPage(a.screenshotId)).rejects.toThrow();}});
});
