"""Accept only a successful trusted CI run whose anonymous bytes match the local manifest."""
import argparse,hashlib,json,os,re,subprocess,urllib.request,urllib.error,urllib.parse
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('version',choices=['4.1.2','4.1.3','4.1.4','4.1.5','4.1.6','4.1.7','4.1.8','4.1.9','4.2.0','4.2.1','4.2.2','4.2.3','4.2.4','4.2.5','4.2.6','4.2.7']);p.add_argument('run',type=int);p.add_argument('job',type=int);a=p.parse_args();root=Path(__file__).resolve().parent.parent
out=root/('artifacts/publish-v'+a.version.replace('.',''));manifest=json.loads((out/'manifest.json').read_text(encoding='utf-8'));publication=json.loads((out/'publication.json').read_text(encoding='utf-8'));assert publication.get('published') and manifest['commit']==publication['commit']
c=subprocess.run(['git','-c','credential.interactive=false','credential','fill'],input='protocol=https\nhost=github.com\n\n',text=True,capture_output=True,check=True).stdout;token=dict(x.split('=',1) for x in c.splitlines() if '=' in x)['password'];base='https://api.github.com/repos/wpy-sketch/ResearchClip'
class NoRedirect(urllib.request.HTTPRedirectHandler):
 def redirect_request(self,*args,**kwargs):return None
proxy=urllib.request.ProxyHandler({'https':os.environ.get('RESEARCHCLIP_RELEASE_PROXY','http://127.0.0.1:7897')});opener=urllib.request.build_opener(proxy,NoRedirect());public=urllib.request.build_opener(proxy)
def req(path):
 request=urllib.request.Request(base+path,headers={'Authorization':'Bearer '+token,'Accept':'application/vnd.github+json','User-Agent':'ResearchClip-CI-Receipt'})
 try:
  with opener.open(request,timeout=60) as r:return json.load(r)
 except urllib.error.HTTPError as e:
  if e.code==302 and path.endswith('/logs'):return e.headers['Location']
  raise RuntimeError('GitHub HTTP '+str(e.code)) from None
run=req('/actions/runs/'+str(a.run));job=req('/actions/jobs/'+str(a.job));assert run['conclusion']=='success' and run['status']=='completed' and run['event']=='workflow_dispatch' and run['workflow_id']==394922334 and run['path']=='.github/workflows/release-transport.yml' and run['head_repository']['full_name']=='wpy-sketch/ResearchClip'
assert job['run_id']==a.run and job['name']=='public-verification' and job['conclusion']=='success';assert re.fullmatch('[a-f0-9]{40}',run['head_sha']);subprocess.run(['git','merge-base','--is-ancestor',run['head_sha'],'HEAD'],cwd=root,check=True)
url=req('/actions/jobs/'+str(a.job)+'/logs');host=urllib.parse.urlsplit(url);assert host.scheme=='https' and (host.hostname.endswith('.blob.core.windows.net') or host.hostname.endswith('.githubusercontent.com'))
with public.open(urllib.request.Request(url,headers={'User-Agent':'ResearchClip-CI-Receipt'}),timeout=60) as r:logs=r.read(4*1024*1024).decode('utf-8')
matches=[]
for line in logs.splitlines():
 at=line.find('{"result": "PASS", "mode": "verify')
 if at>=0:matches.append(json.loads(line[at:]))
assert len(matches)==1;result=matches[0];assert result['mode']=='verify'+a.version.replace('.','') and result['tag']=='v'+a.version and result['releaseId']==publication['id'] and result['sourceCommit']==manifest['commit'] and result['workflowCommit']==run['head_sha']
fields=lambda items:sorted((x['name'],x['size'],x['sha256']) for x in items);assert fields(result['publicVerifiedAssets'])==fields(manifest['assets']);assert len({x['name'] for x in result['publicVerifiedAssets']})==len(manifest['assets'])
receipt={**result,'workflowRun':a.run,'workflowJob':a.job,'workflowUrl':run['html_url'],'logSha256':hashlib.sha256(logs.encode()).hexdigest()};evidence=root/f'docs/qa/evidence/v{a.version.replace(".","")}-public-verification.json';assert not evidence.exists();evidence.write_text(json.dumps(receipt,ensure_ascii=False,indent=2),encoding='utf-8')
publication.update({'anonymousVerified':sorted(x['name'] for x in result['publicVerifiedAssets']),'anonymousVerificationEnvironment':result['verificationEnvironment'],'anonymousVerificationRun':run['html_url'],'anonymousVerificationJob':a.job,'anonymousVerificationWorkflowCommit':run['head_sha']});(out/'publication.json').write_text(json.dumps(publication,ensure_ascii=False,indent=2),encoding='utf-8');print('PASS: accepted',len(manifest['assets']),'actual anonymous CI downloads for',a.version,'from',run['html_url'])
