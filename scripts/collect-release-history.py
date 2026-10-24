"""Collect byte-verified historic GitHub assets; never execute their contents."""
import argparse,hashlib,json,os,shutil,urllib.request
from pathlib import Path

def digest(path):
 with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def main():
 p=argparse.ArgumentParser();p.add_argument('snapshot',type=Path);args=p.parse_args()
 root=Path(__file__).resolve().parent.parent;history=json.loads(args.snapshot.read_text(encoding='utf-8'));cache=root/'artifacts/history-assets';cache.mkdir(exist_ok=True)
 by_name={}
 for folder,dirs,files in os.walk(root/'artifacts',followlinks=False):
  dirs[:]=[x for x in dirs if not (Path(folder)/x).is_symlink() and not (Path(folder)/x).is_junction()]
  for name in files:by_name.setdefault(name,[]).append(Path(folder)/name)
 opener=urllib.request.build_opener(urllib.request.ProxyHandler({'https':os.environ.get('RESEARCHCLIP_RELEASE_PROXY','http://127.0.0.1:7897')}));items=[]
 for release in history:
  tag=release['tag_name'];assert '/' not in tag and '\\' not in tag
  for asset in release['assets']:
   name=asset['name'];assert Path(name).name==name and name not in ('.','..')
   expected=asset.get('digest');assert expected and expected.startswith('sha256:');sha=expected[7:];assert len(sha)==64
   local=cache/sha
   if local.exists():assert local.stat().st_size==asset['size'] and digest(local)==sha
   else:
    local_match=next((f for f in by_name.get(name,[]) if not f.is_symlink() and f.stat().st_size==asset['size'] and digest(f)==sha),None)
    if local_match:
     try:os.link(local_match,local)
     except OSError:shutil.copyfile(local_match,local)
    else:
     url=asset['browser_download_url'];assert url.startswith('https://github.com/wpy-sketch/ResearchClip/releases/download/')
     temporary=cache/(sha+'.partial');assert not temporary.exists(),'Retain interrupted data for inspection; use explicit clean staging'
     with opener.open(urllib.request.Request(url,headers={'User-Agent':'ResearchClip-History'}),timeout=120) as response,temporary.open('xb') as output:shutil.copyfileobj(response,output,1024*1024)
     assert temporary.stat().st_size==asset['size'] and digest(temporary)==sha;temporary.rename(local)
   items.append({'tag':tag,'name':name,'size':asset['size'],'sha256':sha,'source':asset['browser_download_url'],'assetId':asset['id']})
  print('Verified '+tag,flush=True)
 (cache/'index.json').write_text(json.dumps({'releases':len(history),'assets':items},ensure_ascii=False,indent=2),encoding='utf-8')
 print(json.dumps({'result':'PASS','assets':len(items),'bytes':sum(x['size'] for x in items)}),flush=True)
if __name__=='__main__':main()
