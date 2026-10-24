"""Publish a new verified local desktop build without overwriting old releases."""
import argparse, hashlib, json, os, shutil, subprocess, urllib.request, urllib.error, urllib.parse, ssl, time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('version');p.add_argument('--prepare',action='store_true');p.add_argument('--defer-raw',action='store_true',help='Prepare only a draft and defer the raw EXE to verified transport; never publish.');p.add_argument('--installer-run-id');p.add_argument('--target-dir',default='src-tauri/target');p.add_argument('--defer-anonymous',action='store_true',help='Publish verified attachments, then await actual anonymous CI download verification.');p.add_argument('--defer-inherited',action='store_true',help='Stage only new v4.1.3 assets for verified CI inheritance; never publish.');a=p.parse_args()
root=Path(__file__).resolve().parent.parent
buildroot=(root/a.target_dir).resolve();assert buildroot.is_relative_to((root/'src-tauri/target').resolve()),'Build target must stay inside workspace target'
if a.installer_run_id:
    import re
    assert re.fullmatch('v'+a.version.replace('.','')+'-[a-z0-9-]{1,48}',a.installer_run_id),'Unsafe installer evidence ID'
assert a.version in ('3.1.4','3.1.5','3.1.6','3.1.7','3.1.8','3.1.9','4.0.0','4.1.0','4.1.1','4.1.2','4.1.3')
if a.version=='4.1.3':
    previous=json.loads((root/'artifacts/publish-v412/publication.json').read_text(encoding='utf-8'))
    assert previous.get('published') and len(previous.get('anonymousVerified',[]))==237 and previous.get('historyAfter',{}).get('result')=='PASS','Complete v4.1.2 publication and verification first'
out=root/'artifacts'/('publish-v'+a.version.replace('.',''));out.mkdir(exist_ok=True)
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
assert not a.defer_inherited or a.version=='4.1.3'
workers=int(os.environ.get('RESEARCHCLIP_RELEASE_WORKERS','4'));assert 1<=workers<=8
commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
if a.prepare:
    assert json.loads((root/'package.json').read_text(encoding='utf-8'))['version']==a.version,'Current source version differs'
    assert not any(out.iterdir()),'Refusing nonempty release staging folder'
    companions=[]
    if a.version in ('3.1.6','3.1.7','3.1.8','3.1.9','4.0.0'):
        folder=root/'artifacts'/('companions-v'+a.version.replace('.',''))
        cm=json.loads((folder/'manifest.json').read_text(encoding='utf-8'));assert cm['version']==a.version
        qa_folder='installer-v316-pwsh' if a.version=='3.1.6' else 'installer-v319-verified' if a.version=='3.1.9' else f'installer-v{a.version.replace(".","")}-final'
        qa=json.loads((root/'artifacts'/qa_folder/'report.json').read_text(encoding='utf-8-sig'))
        assert qa['result']=='passed' and qa['version']==a.version and not qa['installationStillPresent']
        assert digest(buildroot/f'release/bundle/nsis/ResearchClip_{a.version}_x64-setup.exe')==qa['installerSha256'].lower(),'Installer differs from tested payload'
        required={'ResearchClip-WebClipper-v1.0.2-windows-x64.zip','ResearchClip-MathOCR-Setup-v1.0.4.zip','QWEN-MODEL-MANIFEST.json','MODEL-RESOURCES.txt','OFFLINE-RESOURCE-INDEX.json',f'ResearchClip-Guide-v{a.version}.zip'}
        assert {x['name'] for x in cm['assets']}==required
        for x in cm['assets']:
            assert Path(x['name']).name==x['name'] and not (out/x['name']).exists()
            f=folder/x['name'];assert f.stat().st_size==x['size'] and digest(f)==x['sha256']
            companions.append((f,x['name']))
    if a.version in ('4.1.2','4.1.3'):
        folder=root/('artifacts/companions-v'+a.version.replace('.',''));cm=json.loads((folder/'manifest.json').read_text(encoding='utf-8'))
        assert cm['version']==a.version and cm['history']['sourceAssets']==(226 if a.version=='4.1.2' else 463) and cm['history']['releases']==(31 if a.version=='4.1.2' else 32)
        for x in cm['assets']:
            assert Path(x['name']).name==x['name'];source=folder/x['name']
            assert source.stat().st_size==x['size'] and digest(source)==x['sha256']
            companions.append((source,x['name']))
    if a.version in ('4.1.0','4.1.1','4.1.2','4.1.3'):
        qa_id=a.installer_run_id or ('v'+a.version.replace('.','')+'-final')
        qa=json.loads((root/f'artifacts/installer-{qa_id}/report.json').read_text(encoding='utf-8-sig'))
        assert qa['result']=='passed' and qa['version']==a.version and not qa['installationStillPresent']
        assert digest(buildroot/f'release/bundle/nsis/ResearchClip_{a.version}_x64-setup.exe')==qa['installerSha256'].lower(),'Installer differs from tested payload'
        import zipfile
        guide=out/f'ResearchClip-Guide-v{a.version}.zip'
        if a.version in ('4.1.2','4.1.3'):
            from release_asset_io import write_guide
            write_guide(root,guide,a.version)
        else:
            with zipfile.ZipFile(guide,'x',compression=zipfile.ZIP_DEFLATED) as archive:
                for path in sorted((root/'docs').rglob('*')):
                    if path.is_file() and path.suffix.lower() in ('.md','.json','.png','.jpg','.svg','.txt'):
                        archive.write(path,path.relative_to(root).as_posix())
            with zipfile.ZipFile(guide) as archive:
                assert archive.testzip() is None
        assert (root/f'docs/qa/RELEASE-{a.version}.md').is_file()
    for src,name in [(buildroot/'release/researchclip.exe',f'ResearchClip-v{a.version}-windows-x64.exe'),(buildroot/f'release/bundle/nsis/ResearchClip_{a.version}_x64-setup.exe',f'ResearchClip_{a.version}_x64-setup.exe'),*companions]:
        if a.version in ('4.1.0','4.1.1','4.1.2','4.1.3') and name==f'ResearchClip-v{a.version}-windows-x64.exe':
            import zipfile
            with zipfile.ZipFile(out/name.replace('.exe','.zip'),'x',compression=zipfile.ZIP_DEFLATED,compresslevel=9) as archive:
                archive.write(src,'researchclip.exe')
                if a.version in ('4.1.1','4.1.2','4.1.3'):archive.write(root/'docs/licenses/flexlayout-react-MIT.txt','licenses/flexlayout-react-MIT.txt')
                if a.version=='4.1.3':archive.write(root/'docs/licenses/remark-gfm-NOTICES.txt','licenses/remark-gfm-NOTICES.txt')
            continue
        dest=out/name
        assert not dest.exists(),'Refusing existing release artifact'
        if a.version in ('4.1.2','4.1.3') and src.parent.name=='companions-v'+a.version.replace('.',''):os.link(src,dest)
        else:shutil.copyfile(src,dest)
    assets=[{'name':f.name,'size':f.stat().st_size,'sha256':digest(f)} for f in sorted(out.iterdir()) if f.is_file()]
    (out/'SOURCE.txt').write_text(f'ResearchClip v{a.version}\nRepository: https://github.com/wpy-sketch/ResearchClip\nCommit: {commit}\n'+('All 463 attachments from 32 prior releases through v4.1.2 are included byte-for-byte; see RELEASE-HISTORY-v4.1.3.txt and V413-HISTORY-INVENTORY.json. No user data or credentials included.\n' if a.version=='4.1.3' else 'All 226 attachments from 31 previous releases through v4.1.1 are included in verified history volumes; see HISTORY-README.txt and HISTORY-TRANSPORT.json. Optional companion installers are also attached directly. No user data or credentials included.\n' if a.version in ('4.1.2','4.1.3') else 'Companions: exact WebClipper v1.0.2, MathOCR Setup v1.0.4 and Qwen manifest from v3.1.3; offline model volumes remain on v3.1.2. See MODEL-RESOURCES.txt and OFFLINE-RESOURCE-INDEX.json.\nGuide includes fictional feature exercises and local offline images; see the versioned user guide. No user data or credentials included.\n' if companions else ''),encoding='utf-8')
    f=out/'SOURCE.txt';assets.append({'name':f.name,'size':f.stat().st_size,'sha256':digest(f)})
    (out/'SHA256SUMS.txt').write_text(''.join(x['sha256']+'  '+x['name']+'\n' for x in assets),encoding='utf-8')
    f=out/'SHA256SUMS.txt';assets.append({'name':f.name,'size':f.stat().st_size,'sha256':digest(f)})
    (out/'manifest.json').write_text(json.dumps({'version':a.version,'commit':commit,'assets':assets},indent=2),encoding='utf-8');print('Prepared '+a.version);raise SystemExit(0)
m=json.loads((out/'manifest.json').read_text(encoding='utf-8'));commit=m['commit'];assert m['version']==a.version
assert subprocess.check_output(['git','rev-parse','refs/tags/v'+a.version],cwd=root,text=True).strip()==commit
credential=subprocess.run(['git','-c','credential.interactive=false','credential','fill'],input='protocol=https\nhost=github.com\n\n',text=True,capture_output=True,check=True,env={**os.environ,'GCM_INTERACTIVE':'Never'}).stdout
fields=dict(s.split('=',1) for s in credential.splitlines() if '=' in s);token=fields.get('password');assert token,'Existing Git credential unavailable'
class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self,*args,**kwargs):return None
opener=urllib.request.build_opener(urllib.request.ProxyHandler({'https':os.environ['RESEARCHCLIP_RELEASE_PROXY']} if os.environ.get('RESEARCHCLIP_RELEASE_PROXY') else {}),urllib.request.HTTPSHandler(context=ssl.create_default_context()),NoRedirect())
base='https://api.github.com/repos/wpy-sketch/ResearchClip'
def req(url,method='GET',value=None,binary=None):
    headers={'Authorization':'Bearer '+token,'Accept':'application/vnd.github+json','User-Agent':'ResearchClip-Release','X-GitHub-Api-Version':'2022-11-28'}
    data=binary
    if value is not None:data=json.dumps(value,ensure_ascii=False).encode();headers['Content-Type']='application/json'
    if binary is not None:headers['Content-Type']='application/octet-stream'
    for attempt in range(3 if method=='GET' else 1):
        try:
            assert urllib.parse.urlsplit(url).hostname in ('api.github.com','uploads.github.com')
            with opener.open(urllib.request.Request(url,method=method,data=data,headers=headers),timeout=600 if binary else 30) as r:
                response=r.read();return json.loads(response) if response else None
        except urllib.error.HTTPError as e:raise RuntimeError('GitHub HTTP '+str(e.code)+' '+method) from None
        except (urllib.error.URLError,ConnectionError,TimeoutError):
            if method!='GET' or attempt==2:raise
            time.sleep(1)
assert req('https://api.github.com/user')['login']=='wpy-sketch'
assert req(base+'/git/ref/tags/v'+a.version)['object']['sha']==commit
releases=req(base+'/releases?per_page=100');existing=next((r for r in releases if r['tag_name']=='v'+a.version),None)
def check_history(releases):
    if a.version not in ('3.1.6','3.1.7','3.1.8','3.1.9','4.0.0','4.1.2','4.1.3'):return None
    before=json.loads((root/f'docs/qa/evidence/v{a.version.replace(".","")}-prior-releases.json').read_text(encoding='utf-8'))
    if a.version in ('4.1.2','4.1.3'):assert {x['id'] for x in releases if x['tag_name']!='v'+a.version}=={x['id'] for x in before},'New historical release appeared; refresh the archive before publication'
    for old in before:
        current=next((x for x in releases if x['id']==old['id']),None)
        assert current and current['tag_name']==old['tag'] and current['draft']==old['draft'] and current['prerelease']==old['prerelease'] and current['target_commitish']==old['commit'],'Historical release changed'
        fields=lambda assets:sorted((x['id'],x['name'],x['size'],x['digest']) for x in assets)
        assert fields(__import__('release_asset_io').read_assets(req,base,current['id']) if len(old['assets'])>100 else current['assets'])==fields(old['assets']),'Historical release attachments changed'
    return {'releases':len(before),'assets':sum(len(x['assets']) for x in before),'result':'PASS'}
history=check_history(releases)
prior=json.loads((out/'publication.json').read_text(encoding='utf-8')) if (out/'publication.json').exists() else None
if prior:existing=req(base+'/releases/'+str(prior['id']));assert existing['tag_name']=='v'+a.version
if existing:assert prior and prior['id']==existing['id'] and prior['commit']==commit,'Unrecognized existing release; refusing overwrite'
body=(root/f'docs/RELEASE-NOTES-{a.version}.md').read_text(encoding='utf-8').replace('(qa/','(https://github.com/wpy-sketch/ResearchClip/blob/v'+a.version+'/docs/qa/').replace('(USER_GUIDE.md)','(https://github.com/wpy-sketch/ResearchClip/blob/v'+a.version+'/docs/USER_GUIDE.md)').replace('(INTERNAL-LINKS.md)','(https://github.com/wpy-sketch/ResearchClip/blob/v'+a.version+'/docs/INTERNAL-LINKS.md)')
body=body.replace('(SEMANTIC-SEARCH.md)','(https://github.com/wpy-sketch/ResearchClip/blob/v'+a.version+'/docs/SEMANTIC-SEARCH.md)')
if a.version in ('4.1.0','4.1.1','4.1.2','4.1.3'):
    import re,posixpath
    def absolute_doc_link(match):
        target=match.group(1)
        if '://' in target or target.startswith('#') or target.startswith('mailto:'):
            return match.group(0)
        return '('+f'https://github.com/wpy-sketch/ResearchClip/blob/v{a.version}/'+posixpath.normpath('docs/'+target)+')'
    body=re.sub(r'(?<=\])\(([^)]+)\)',absolute_doc_link,body)
r=existing or req(base+'/releases','POST',{'tag_name':'v'+a.version,'target_commitish':commit,'name':'ResearchClip v'+a.version,'body':body,'draft':True,'prerelease':False})
state=prior or {'version':a.version,'commit':commit,'id':r['id'],'url':r['html_url'],'assets':[]}
if history:state['historyBefore']=history
def save():(out/'publication.json').write_text(json.dumps(state,ensure_ascii=False,indent=2),encoding='utf-8')
save()
if a.version in ('4.1.2','4.1.3'):
    from release_asset_io import read_assets,transfers,verified,interrupted_upload
    current_assets=read_assets(req,base,r['id'])
    for asset in current_assets:
        if asset['state']=='starter':
            expected=next((x for x in m['assets'] if x['name']==asset['name']),None)
            assert expected and interrupted_upload(expected,asset,bool(r['draft'] and prior and prior['id']==r['id'])),'Unrecognized incomplete asset; preserve for audit'
            req(base+'/releases/assets/'+str(asset['id']),'DELETE')
    current_assets=read_assets(req,base,r['id'])
    def upload(x):
        assert r['draft'],'Cannot modify published assets'
        f=out/x['name'];assert digest(f)==x['sha256']
        return req(r['upload_url'].split('{')[0]+'?'+urllib.parse.urlencode({'name':x['name']}),'POST',binary=f.read_bytes())
    transfer_expected=m['assets']
    if a.defer_inherited:
        previous_manifest=json.loads((root/'artifacts/publish-v412/manifest.json').read_text(encoding='utf-8'))
        inherited={x['name'] for x in previous_manifest['assets'] if x['name'] not in ('SOURCE.txt','SHA256SUMS.txt')}
        assert len(inherited)==235
        for asset in current_assets:verified(next(x for x in m['assets'] if x['name']==asset['name']),asset)
        transfer_expected=[x for x in m['assets'] if x['name'] not in inherited];assert len(transfer_expected)==8
        current_assets=[x for x in current_assets if x['name'] not in inherited]
    for x,result in transfers(transfer_expected,current_assets,upload,workers=workers):
        state['assets'].append({'name':x['name'],'size':result['size'],'digest':result['digest'],'url':result['browser_download_url']});save();print('Uploaded '+x['name'],flush=True)
    current_assets=read_assets(req,base,r['id'])
    if a.defer_inherited:
        assert r['draft'];state['awaitingInheritance']=True;save();print('Draft staged; CI inheritance and final verification/publication still required',flush=True);raise SystemExit(0)
    assert len(current_assets)==len(m['assets'])
    for x in m['assets']:verified(x,next(asset for asset in current_assets if asset['name']==x['name']))
else:
    assert all(asset['name'] in {x['name'] for x in m['assets']} for asset in r['assets']),'Unexpected draft assets; refusing publication'
    for x in sorted(m['assets'],key=lambda x:x['size']):
        f=out/x['name'];assert digest(f)==x['sha256']
        import urllib.parse
        current=next((asset for asset in r['assets'] if asset['name']==x['name']),None)
        if current and current['state']=='starter' and current['size']==0:
            assert r['draft'];req(base+'/releases/assets/'+str(current['id']),'DELETE');current=None
        if current:
            assert current['state']=='uploaded' and current['size']==x['size'] and current['digest']=='sha256:'+x['sha256'],'Existing asset differs; refusing overwrite'
            print('Verified existing '+x['name'],flush=True);continue
        assert r['draft'],'Cannot modify published assets'
        if a.defer_raw and x['name']==f'ResearchClip-v{a.version}-windows-x64.exe':
            print('Deferred raw EXE; draft will remain unpublished',flush=True);continue
        result=req(r['upload_url'].split('{')[0]+'?'+urllib.parse.urlencode({'name':x['name']}),'POST',binary=f.read_bytes())
        assert result['state']=='uploaded' and result['size']==x['size'] and result['digest']=='sha256:'+x['sha256']
        state['assets'].append({'name':x['name'],'size':result['size'],'digest':result['digest'],'url':result['browser_download_url']});save();print('Uploaded '+x['name'],flush=True)
if a.defer_raw:
    assert r['draft'];state['deferredRaw']=True;save();print('Draft prepared; final publication and anonymous verification still required',flush=True);raise SystemExit(0)
state.pop('deferredRaw',None);state.pop('awaitingInheritance',None)
if r['draft']:r=req(base+'/releases/'+str(r['id']),'PATCH',{'tag_name':'v'+a.version,'target_commitish':commit,'draft':False,'prerelease':False,'make_latest':'true'})
assert not r['draft'] and not r['prerelease'];state['published']=True;state['url']=r['html_url'];assert r['tag_name']=='v'+a.version;save()
release=req(base+'/releases/'+str(r['id']))
if a.version in ('4.1.2','4.1.3'):release['assets']=read_assets(req,base,r['id'])
assert len(release['assets'])==len(m['assets'])
state['assets']=[{'name':asset['name'],'size':asset['size'],'digest':asset['digest'],'url':asset['browser_download_url']} for asset in release['assets']];save()
if a.defer_anonymous:
    print('Published; actual anonymous CI verification and final history audit still required',flush=True);raise SystemExit(0)
public=urllib.request.build_opener(urllib.request.ProxyHandler({'https':os.environ['RESEARCHCLIP_RELEASE_PROXY']} if os.environ.get('RESEARCHCLIP_RELEASE_PROXY') else {}),urllib.request.HTTPSHandler(context=ssl.create_default_context()))
if a.version in ('4.1.2','4.1.3'):
    from concurrent.futures import ThreadPoolExecutor,as_completed
    verified_names=set(state.get('anonymousVerified',[]));assert verified_names<={x['name'] for x in m['assets']}
    def anonymous(x):
        remote=next(asset for asset in release['assets'] if asset['name']==x['name'])
        for attempt in range(3):
            try:
                with public.open(urllib.request.Request(remote['browser_download_url'],headers={'User-Agent':'ResearchClip-Anonymous-Verify'}),timeout=180) as response:
                    data=response.read();assert response.status==200
                assert len(data)==x['size'] and hashlib.sha256(data).hexdigest()==x['sha256'];return x['name']
            except (urllib.error.URLError,ConnectionError,TimeoutError):
                if attempt==2:raise
                time.sleep(2)
    with ThreadPoolExecutor(max_workers=workers) as pool:
        futures=[pool.submit(anonymous,x) for x in m['assets'] if x['name'] not in verified_names]
        for future in as_completed(futures):
            name=future.result();verified_names.add(name);state['anonymousVerified']=sorted(verified_names);save();print('Anonymous hash verified '+name,flush=True)
else:
    verified=[]
    for x in m['assets']:
        remote=next(asset for asset in release['assets'] if asset['name']==x['name'])
        with public.open(urllib.request.Request(remote['browser_download_url'],headers={'User-Agent':'ResearchClip-Anonymous-Verify'}),timeout=60) as response:
            data=response.read();assert response.status==200
        assert len(data)==x['size'] and hashlib.sha256(data).hexdigest()==x['sha256']
        verified.append(x['name']);state['anonymousVerified']=verified;save();print('Anonymous hash verified '+x['name'],flush=True)
history=check_history(req(base+'/releases?per_page=100'))
if history:state['historyAfter']=history;save()
print(json.dumps(state,ensure_ascii=False),flush=True)
