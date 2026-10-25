"""Manually dispatched release transport only; never publishes or executes attachments."""
import argparse,base64,hashlib,json,os,re,ssl,urllib.request,urllib.error,urllib.parse,zlib
from concurrent.futures import ThreadPoolExecutor,as_completed
from pathlib import Path
from release_asset_io import read_assets,verified,interrupted_upload
ROOT=Path(__file__).resolve().parent.parent
BASE='https://api.github.com/repos/wpy-sketch/ResearchClip'
def sha(path):
 with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def validate_source_receipt(source,tag,commit):
 lines=source.decode('utf-8').splitlines();assert lines and lines[0]=='ResearchClip '+tag and 'Commit: '+commit in lines
def reconstruct(recipe,output,load):
 assert recipe['schemaVersion']==1 and 0<recipe['archive']['size']<=5*1024**3 and len(recipe['segments'])<=1000
 assert sum(x['size'] for x in recipe['segments'])==recipe['archive']['size']
 with output.open('xb') as target:
  for segment in recipe['segments']:
   start=target.tell()
   if segment['kind']=='literal':
    data=base64.b64decode(segment['data'],validate=True);assert len(data)==segment['size'] and len(data)<=1024*1024;target.write(data)
   else:
    assert segment['kind']=='asset' and segment['compression'] in (0,8)
    item=segment['source'];source=load(item);assert source.stat().st_size==item['size'] and sha(source)==item['sha256']
    compressor=zlib.compressobj(6,zlib.DEFLATED,-15) if segment['compression']==8 else None;digest=hashlib.sha256()
    with source.open('rb') as stream:
     while chunk:=stream.read(1024*1024):
      encoded=compressor.compress(chunk) if compressor else chunk;target.write(encoded);digest.update(encoded)
    if compressor:
     encoded=compressor.flush();target.write(encoded);digest.update(encoded)
    assert digest.hexdigest()==segment['encodedSha256'],'Compression differs: '+item['tag']+'/'+item['name']
   assert target.tell()-start==segment['size'],'Encoded length differs'
   print('Reconstructed segment',target.tell(),'/',recipe['archive']['size'],flush=True)
 assert output.stat().st_size==recipe['archive']['size'] and sha(output)==recipe['archive']['sha256'],'Exact archive digest differs'
def main():
 p=argparse.ArgumentParser();p.add_argument('mode',choices=['local-reconstruct','reconstruct412','inherit413','verify412','verify413','inherit414','verify414','inherit415','verify415','inherit416','verify416']);p.add_argument('--target-release',type=int);p.add_argument('--target-commit');p.add_argument('--run-id',default='ci');args=p.parse_args();assert re.fullmatch('[a-z0-9-]{1,60}',args.run_id)
 recipe=json.loads((ROOT/'docs/qa/evidence/v412-archive-recipe.json').read_text(encoding='utf-8'));out=ROOT/'artifacts'/('release-transport-'+args.run_id);out.mkdir(parents=True)
 if args.mode=='local-reconstruct':
  reconstruct(recipe,out/'history.zip',lambda item:ROOT/'artifacts/history-assets'/item['sha256']);print('PASS exact local archive');return
 assert args.target_release and re.fullmatch('[a-f0-9]{40}',args.target_commit or '')
 token=os.environ['GH_TOKEN'];headers={'Authorization':'Bearer '+token,'Accept':'application/vnd.github+json','User-Agent':'ResearchClip-Release-Transport','X-GitHub-Api-Version':'2022-11-28'}
 class NoRedirect(urllib.request.HTTPRedirectHandler):
  def redirect_request(self,*args,**kwargs):return None
 opener=urllib.request.build_opener(urllib.request.HTTPSHandler(context=ssl.create_default_context()),NoRedirect())
 def request(url,method='GET',binary=None):
  assert urllib.parse.urlsplit(url).hostname in ('api.github.com','uploads.github.com')
  req=urllib.request.Request(url,method=method,data=binary,headers={**headers,**({'Content-Type':'application/octet-stream'} if binary is not None else {})})
  try:
   with opener.open(req,timeout=300) as response:body=response.read();return json.loads(body) if body else None
  except urllib.error.HTTPError as error:raise RuntimeError('GitHub '+method+' HTTP '+str(error.code)) from None
 release=request(BASE+'/releases/'+str(args.target_release));tag='v'+{'412':'4.1.2','413':'4.1.3','414':'4.1.4','415':'4.1.5','416':'4.1.6'}[args.mode[-3:]]
 assert request(BASE+'/git/ref/tags/'+tag)['object']['sha']==args.target_commit
 assert not release['prerelease'] and release['tag_name']==tag and release['target_commitish']==args.target_commit
 if args.mode.startswith('verify'):
  assert not release['draft'];assets=read_assets(request,BASE,release['id']);assert len(assets)=={'v4.1.2':237,'v4.1.3':243,'v4.1.4':249,'v4.1.5':255,'v4.1.6':261}[tag]
  def anonymous(item,keep=False):
   url=item['browser_download_url'];assert url.startswith('https://github.com/wpy-sketch/ResearchClip/releases/download/'+tag+'/') and item['state']=='uploaded'
   digest=hashlib.sha256();total=0;pieces=[]
   with urllib.request.urlopen(urllib.request.Request(url,headers={'User-Agent':'ResearchClip-Anonymous-CI-Verify'}),timeout=180) as response:
    assert response.status==200
    while chunk:=response.read(1024*1024):
     total+=len(chunk);assert total<=item['size'];digest.update(chunk)
     if keep:assert total<=1024*1024;pieces.append(chunk)
   assert total==item['size'] and 'sha256:'+digest.hexdigest()==item['digest']
   return b''.join(pieces) if keep else {'name':item['name'],'size':total,'sha256':digest.hexdigest()}
  sums=next(x for x in assets if x['name']=='SHA256SUMS.txt');checks={}
  for line in anonymous(sums,True).decode('utf-8').splitlines():
   digest,name=line.split('  ',1);assert re.fullmatch('[a-f0-9]{64}',digest) and Path(name).name==name and '\\' not in name and name not in checks;checks[name]=digest
  assert set(checks)=={x['name'] for x in assets if x['name']!='SHA256SUMS.txt'}
  for item in assets:
   if item['name']!='SHA256SUMS.txt':assert item['digest']=='sha256:'+checks[item['name']]
  validate_source_receipt(anonymous(next(x for x in assets if x['name']=='SOURCE.txt'),True),tag,args.target_commit)
  if tag=='v4.1.2':
   frozen=json.loads((ROOT/'docs/qa/evidence/v412-upload-manifest.json').read_text(encoding='utf-8'));assert sorted((x['name'],x['size'],x['sha256']) for x in frozen['assets'])==sorted((x['name'],x['size'],x['digest'][7:]) for x in assets)
  results=[]
  with ThreadPoolExecutor(max_workers=4) as pool:
   for future in as_completed([pool.submit(anonymous,x) for x in assets]):
    result=future.result();results.append(result);print('Anonymous hash verified',result['name'],flush=True)
  result={'result':'PASS','mode':args.mode,'releaseId':release['id'],'tag':tag,'sourceCommit':args.target_commit,'workflowCommit':os.environ.get('GITHUB_SHA'),'publicVerifiedAssets':sorted(results,key=lambda x:x['name']),'verificationEnvironment':'GitHub-hosted Ubuntu24.04, anonymous public HTTP, no Authorization on downloads'}
  (out/'verification.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
  if os.environ.get('GITHUB_STEP_SUMMARY'):Path(os.environ['GITHUB_STEP_SUMMARY']).write_text('PASS: anonymous public bytes verified for '+str(len(results))+' assets of '+tag+'\n',encoding='utf-8')
  print(json.dumps(result),flush=True);return
 assert release['draft'],'Only the exact authorized draft is writable'
 cache=out/'cache';cache.mkdir()
 def load(item):
  assert re.fullmatch('[a-f0-9]{64}',item['sha256']) and 0<item['size']<=512*1024**2
  url=item['source'];assert url.startswith('https://github.com/wpy-sketch/ResearchClip/releases/download/')
  path=cache/item['sha256']
  if path.exists():assert sha(path)==item['sha256'];return path
  with urllib.request.urlopen(urllib.request.Request(url,headers={'User-Agent':'ResearchClip-Public-Release-Transport'}),timeout=180) as response,path.open('xb') as target:
   total=0
   while chunk:=response.read(1024*1024):
    total+=len(chunk);assert total<=item['size'];target.write(chunk)
  assert path.stat().st_size==item['size'] and sha(path)==item['sha256'];return path
 if args.mode=='reconstruct412':
  assert args.target_release==recipe['releaseId'] and args.target_commit==recipe['sourceCommit']
  manifest=json.loads((ROOT/'docs/qa/evidence/v412-upload-manifest.json').read_text(encoding='utf-8'));archive=out/'history.zip';reconstruct(recipe,archive,load)
  expected=[x for x in manifest['assets'] if x['name'].startswith('ResearchClip-History-through-v4.1.1.zip.')];assert len(expected)==223
  def payload(item):
   number=int(item['name'].rsplit('.',1)[1]);assert 1<=number<=223
   with archive.open('rb') as stream:stream.seek((number-1)*16777216);return stream.read(item['size'])
 else:
  previous,previous_count,previous_release,snapshot={'inherit413':('v4.1.2',237,recipe['releaseId'],'413'),'inherit414':('v4.1.3',243,422602338,'414'),'inherit415':('v4.1.4',249,422719495,'415'),'inherit416':('v4.1.5',255,422763684,'416')}[args.mode]
  source=request(BASE+'/releases/tags/'+previous);assert not source['draft'] and not source['prerelease'] and source['id']==previous_release
  frozen=json.loads((ROOT/('docs/qa/evidence/v'+snapshot+'-prior-releases.json')).read_text(encoding='utf-8'));old=next(x for x in frozen if x['tag']==previous);current=read_assets(request,BASE,source['id']);assert len(current)==len(old['assets'])==previous_count
  assert sorted((x['id'],x['name'],x['size'],x['digest']) for x in current)==sorted((x['id'],x['name'],x['size'],x['digest']) for x in old['assets'])
  expected=[{'name':x['name'],'size':x['size'],'sha256':x['digest'][7:],'source':x['browser_download_url']} for x in current if x['name'] not in ('SOURCE.txt','SHA256SUMS.txt')];assert len(expected)==previous_count-2
  def payload(item):return load(item).read_bytes()
 existing=read_assets(request,BASE,release['id']);by_name={x['name']:x for x in existing}
 for item in expected:
  actual=by_name.get(item['name'])
  if actual and actual['state']=='starter':
   assert interrupted_upload(item,actual,True);request(BASE+'/releases/assets/'+str(actual['id']),'DELETE');del by_name[item['name']]
  elif actual:verified(item,actual)
 def upload(item):
  if item['name'] in by_name:return item['name']
  data=payload(item);assert len(data)==item['size'] and hashlib.sha256(data).hexdigest()==item['sha256']
  result=request(release['upload_url'].split('{')[0]+'?'+urllib.parse.urlencode({'name':item['name']}),'POST',data);verified(item,result);return item['name']
 with ThreadPoolExecutor(max_workers=4) as pool:
  for future in as_completed([pool.submit(upload,x) for x in expected]):print('Verified transport',future.result(),flush=True)
 final={x['name']:x for x in read_assets(request,BASE,release['id'])}
 for item in expected:verified(item,final[item['name']])
 result={'result':'PASS','mode':args.mode,'releaseId':release['id'],'tag':tag,'sourceCommit':args.target_commit,'assets':len(expected),'archiveSha256':recipe['archive']['sha256'] if args.mode=='reconstruct412' else None,'zlibRuntime':zlib.ZLIB_RUNTIME_VERSION,'published':False}
 (out/'report.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
 if os.environ.get('GITHUB_STEP_SUMMARY'):Path(os.environ['GITHUB_STEP_SUMMARY']).write_text('Verified release transport\n\n```json\n'+json.dumps(result,indent=2)+'\n```\n',encoding='utf-8')
 print(json.dumps(result),flush=True)
if __name__=='__main__':main()
