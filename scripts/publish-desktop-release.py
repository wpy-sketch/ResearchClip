"""Publish a new verified local desktop build without overwriting old releases."""
import argparse, hashlib, json, os, shutil, subprocess, urllib.request, urllib.error, ssl
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('version');p.add_argument('--prepare',action='store_true');a=p.parse_args()
root=Path(__file__).resolve().parent.parent
assert a.version in ('3.1.4','3.1.5')
out=root/'artifacts'/('publish-v'+a.version.replace('.',''));out.mkdir(exist_ok=True)
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
if a.prepare:
    for src,name in [(root/'src-tauri/target/release/researchclip.exe',f'ResearchClip-v{a.version}-windows-x64.exe'),(root/f'src-tauri/target/release/bundle/nsis/ResearchClip_{a.version}_x64-setup.exe',f'ResearchClip_{a.version}_x64-setup.exe')]:
        dest=out/name
        assert not dest.exists(),'Refusing existing release artifact'
        shutil.copyfile(src,dest)
    assets=[{'name':f.name,'size':f.stat().st_size,'sha256':digest(f)} for f in sorted(out.glob('*.exe'))]
    (out/'SHA256SUMS.txt').write_text(''.join(x['sha256']+'  '+x['name']+'\n' for x in assets),encoding='utf-8')
    (out/'SOURCE.txt').write_text(f'ResearchClip v{a.version}\nRepository: https://github.com/wpy-sketch/ResearchClip\nCommit: {commit}\n',encoding='utf-8')
    for name in ['SHA256SUMS.txt','SOURCE.txt']:
        f=out/name;assets.append({'name':name,'size':f.stat().st_size,'sha256':digest(f)})
    (out/'manifest.json').write_text(json.dumps({'version':a.version,'commit':commit,'assets':assets},indent=2),encoding='utf-8');print('Prepared '+a.version);raise SystemExit(0)
m=json.loads((out/'manifest.json').read_text(encoding='utf-8'));assert m['commit']==commit
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
    try:
        with opener.open(urllib.request.Request(url,method=method,data=data,headers=headers),timeout=60) as r:return json.load(r)
    except urllib.error.HTTPError as e:raise RuntimeError('GitHub HTTP '+str(e.code)+' '+method) from None
assert req('https://api.github.com/user')['login']=='wpy-sketch'
assert req(base+'/git/ref/heads/main')['object']['sha']==commit
assert req(base+'/git/ref/tags/v'+a.version)['object']['sha']==commit
releases=req(base+'/releases?per_page=100');assert not any(r['tag_name']=='v'+a.version for r in releases),'Release already exists; refusing overwrite'
body=(root/f'docs/RELEASE-NOTES-{a.version}.md').read_text(encoding='utf-8').replace('(qa/','(https://github.com/wpy-sketch/ResearchClip/blob/v'+a.version+'/docs/qa/').replace('(USER_GUIDE.md)','(https://github.com/wpy-sketch/ResearchClip/blob/v'+a.version+'/docs/USER_GUIDE.md)')
r=req(base+'/releases','POST',{'tag_name':'v'+a.version,'target_commitish':commit,'name':'ResearchClip v'+a.version,'body':body,'draft':True,'prerelease':False})
state={'version':a.version,'commit':commit,'id':r['id'],'url':r['html_url'],'assets':[]}
def save():(out/'publication.json').write_text(json.dumps(state,ensure_ascii=False,indent=2),encoding='utf-8')
save()
for x in m['assets']:
    f=out/x['name'];assert digest(f)==x['sha256']
    import urllib.parse
    result=req(r['upload_url'].split('{')[0]+'?'+urllib.parse.urlencode({'name':x['name']}),'POST',binary=f.read_bytes())
    assert result['state']=='uploaded' and result['size']==x['size'] and result['digest']=='sha256:'+x['sha256']
    state['assets'].append({'name':x['name'],'size':result['size'],'digest':result['digest'],'url':result['browser_download_url']});save();print('Uploaded '+x['name'],flush=True)
r=req(base+'/releases/'+str(r['id']),'PATCH',{'draft':False,'prerelease':False,'make_latest':'true'});assert not r['draft'] and not r['prerelease'];state['published']=True;save()
print(json.dumps(state,ensure_ascii=False),flush=True)
