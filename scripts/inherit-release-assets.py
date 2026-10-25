"""Inherit public release bytes without recursively repacking the multi-GB archive."""
import argparse,hashlib,json,os,subprocess,urllib.request,zipfile
from pathlib import Path
from release_asset_io import read_assets
ROOT=Path(__file__).resolve().parent.parent
def sha(path):
 with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def records(old,previous,previous_version="4.1.2"):
 result=[]
 for item in old:
  assert Path(item['name']).name==item['name'] and '/' not in item['tag']
  result.append({**item,'representation':item.get('representation') or {'archive':'ResearchClip-History-through-v4.1.1.zip','member':item['tag']+'/'+item['name'],'transport':'HISTORY-TRANSPORT.json'}})
 for item in previous:
  assert Path(item['name']).name==item['name']
  representation={'asset':f'ResearchClip-v{previous_version}-Release-Metadata.zip','member':f'v{previous_version}/'+item['name']} if item['name'] in ('SOURCE.txt','SHA256SUMS.txt') else {'asset':item['name']}
  result.append({**item,'tag':'v'+previous_version,'representation':representation})
 assert len({(x['tag'],x['name']) for x in result})==len(result)
 return result
def main():
 p=argparse.ArgumentParser();p.add_argument('version',nargs='?',default='4.1.3',choices=['4.1.3','4.1.4','4.1.5','4.1.6','4.1.7','4.1.8','4.1.9']);args=p.parse_args();version=args.version;short=version.replace('.','');previous,count,old_count,release_count={'4.1.3':('4.1.2',237,226,32),'4.1.4':('4.1.3',243,463,33),'4.1.5':('4.1.4',249,706,34),'4.1.6':('4.1.5',255,955,35),'4.1.7':('4.1.6',261,1210,36),'4.1.8':('4.1.7',267,1471,37),'4.1.9':('4.1.8',273,1738,38)}[version];prev=previous.replace('.','')
 prior=ROOT/f'artifacts/publish-v{prev}';publication=json.loads((prior/'publication.json').read_text(encoding='utf-8'));manifest=json.loads((prior/'manifest.json').read_text(encoding='utf-8'))
 assert publication.get('published') and len(publication.get('anonymousVerified',[]))==len(manifest['assets'])==count and publication.get('historyAfter',{}).get('result')=='PASS','Finish previous publication first'
 credential=subprocess.run(['git','-c','credential.interactive=false','credential','fill'],input='protocol=https\nhost=github.com\n\n',text=True,capture_output=True,check=True).stdout
 token=dict(x.split('=',1) for x in credential.splitlines() if '=' in x)['password']
 opener=urllib.request.build_opener(urllib.request.ProxyHandler({'https':os.environ.get('RESEARCHCLIP_RELEASE_PROXY','http://127.0.0.1:7897')}));base='https://api.github.com/repos/wpy-sketch/ResearchClip'
 def request(url):
  assert url.startswith(base+'/');req=urllib.request.Request(url,headers={'Authorization':'Bearer '+token,'User-Agent':'ResearchClip-Release-Inheritance'})
  with opener.open(req,timeout=60) as response:return json.load(response)
 releases=request(base+'/releases?per_page=100');assert len(releases)==release_count
 actual=next(r for r in releases if r['tag_name']=='v'+previous);assert not actual['draft'] and not actual['prerelease'] and actual['id']==publication['id']
 actual['assets']=read_assets(request,base,actual['id']);assert len(actual['assets'])==count
 expected={x['name']:x for x in manifest['assets']}
 for asset in actual['assets']:
  item=expected[asset['name']];assert asset['size']==item['size'] and asset['digest']=='sha256:'+item['sha256'] and asset['state']=='uploaded'
 frozen=json.loads((ROOT/f'docs/qa/evidence/v{prev}-prior-releases.json').read_text(encoding='utf-8'))
 assert {r['id'] for r in releases if r['tag_name']!='v'+previous}=={r['id'] for r in frozen}
 minimal=lambda r:{'id':r['id'],'tag':r['tag_name'],'draft':r['draft'],'prerelease':r['prerelease'],'commit':r['target_commitish'],'assets':[{k:a[k] for k in ('id','name','size','digest')} for a in r['assets']]}
 for old in frozen:
  current=next(r for r in releases if r['id']==old['id'])
  if len(old['assets'])>100:current['assets']=read_assets(request,base,current['id'])
  assert minimal(current)==old,'Prior public release changed'
 snapshot=frozen+[minimal(actual)];snapshot_path=ROOT/f'docs/qa/evidence/v{short}-prior-releases.json';assert not snapshot_path.exists()
 out=ROOT/f'artifacts/companions-v{short}';out.mkdir()
 for item in manifest['assets']:
  source=prior/item['name'];assert source.stat().st_size==item['size'] and sha(source)==item['sha256']
  if item['name'] not in ('SOURCE.txt','SHA256SUMS.txt'):os.link(source,out/item['name'])
 metadata=out/f'ResearchClip-v{previous}-Release-Metadata.zip'
 with zipfile.ZipFile(metadata,'x',compression=zipfile.ZIP_DEFLATED) as archive:
  for name in ('SOURCE.txt','SHA256SUMS.txt'):archive.write(prior/name,'v'+previous+'/'+name)
 with zipfile.ZipFile(metadata) as archive:
  assert archive.testzip() is None
  for name in ('SOURCE.txt','SHA256SUMS.txt'):assert hashlib.sha256(archive.read('v'+previous+'/'+name)).hexdigest()==expected[name]['sha256']
 older=json.loads((prior/('V'+prev+'-HISTORY-INVENTORY.json' if version in ('4.1.4','4.1.5','4.1.6','4.1.7','4.1.8','4.1.9') else 'HISTORY-CONTENTS.json')).read_text(encoding='utf-8'))['assets'];assert len(older)==old_count
 coverage=records(older,manifest['assets'],previous);assert len(coverage)==old_count+count
 for item in coverage:
  r=item['representation']
  if 'asset'in r:assert (out/r['asset']).is_file()
  else:assert (out/r['transport']).is_file()
 (out/f'V{short}-HISTORY-INVENTORY.json').write_text(json.dumps({'schemaVersion':1,'desktopVersion':version,'releases':release_count,'sourceAssets':len(coverage),'assets':coverage},ensure_ascii=False,indent=2),encoding='utf-8')
 (out/f'RELEASE-HISTORY-v{version}.txt').write_text(f'ResearchClip v{version} includes exact bytes of all {len(coverage)} attachments from {release_count} prior public releases.\nThrough v4.1.1: use unchanged 223 history volumes and Assemble-Release-History.ps1 as described in HISTORY-README.txt.\nLater versions: previous binaries/guide/companions/transport files are attached directly. Colliding SOURCE.txt and SHA256SUMS.txt retain exact bytes/names under each version in its Release-Metadata.zip.\nV{short}-HISTORY-INVENTORY.json maps every original tag/name/size/hash to its direct asset or archive member.\nHISTORY-TRANSPORT.json intentionally retains desktopVersion4.1.2 because archive through v4.1.1 and matching assembler are unchanged. Current SOURCE.txt and SHA256SUMS.txt belong to v{version}.\nOld executables remain old versions; do not execute them to install current app. Only public release files, never user research data.\n',encoding='utf-8')
 assets=[{'name':p.name,'size':p.stat().st_size,'sha256':sha(p)} for p in sorted(out.iterdir())];assert len(assets)==count+1
 (out/'manifest.json').write_text(json.dumps({'version':version,'history':{'releases':release_count,'sourceAssets':len(coverage),'throughVersion':previous,'inheritedVolumes':223},'assets':assets},indent=2),encoding='utf-8')
 snapshot_path.write_text(json.dumps(snapshot,ensure_ascii=False,indent=2),encoding='utf-8');print(f'PASS: {len(coverage)} historical attachments represented in {len(assets)} companions; previous bytes verified; snapshot frozen')
if __name__=='__main__':main()
