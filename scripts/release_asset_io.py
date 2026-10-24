"""Bounded concurrent transfer helpers; all callers supply immutable expected digests."""
from concurrent.futures import ThreadPoolExecutor,as_completed
import re
import posixpath,zipfile,urllib.parse
from pathlib import PurePosixPath

def write_guide(root,path,version):
    names={p.relative_to(root).as_posix() for p in (root/'docs').rglob('*') if p.is_file() and p.suffix.lower() in ('.md','.json','.png','.jpg','.svg','.txt','.bib')}
    names.update(['browser-extension/README.md','browser-extension/PROTOCOL.md','math-ocr/README.md','math-ocr/models.json','math-ocr/LICENSE-RapidLaTeXOCR','math-ocr/LICENSE-LaTeX-OCR','semantic-search/README.md','semantic-search/models.json'])
    names.update(p.relative_to(root).as_posix() for p in (root/'semantic-search/licenses').iterdir() if p.is_file())
    with zipfile.ZipFile(path,'x',compression=zipfile.ZIP_DEFLATED) as archive:
        for name in sorted(names):
            if name.endswith('.md'):
                def link(match):
                    label,href=match.groups()
                    if re.match(r'^(?:[a-zA-Z][a-zA-Z0-9+.-]*:|#|/)',href):return match.group(0)
                    target=posixpath.normpath(posixpath.join(posixpath.dirname(name),urllib.parse.unquote(href.split('#')[0])))
                    if target in names:return match.group(0)
                    assert not target.startswith('../') and (root/target).is_file(),'Unresolved offline guide link: '+href
                    fragment='#'+href.split('#',1)[1] if '#' in href else ''
                    return f'[{label}](https://github.com/wpy-sketch/ResearchClip/blob/v{version}/{target}{fragment})'
                archive.writestr(name,re.sub(r'(?<!!)\[([^\]]+)\]\(([^)]+)\)',link,(root/name).read_text(encoding='utf-8')))
            else:archive.write(root/name,name)
        archive.writestr('README.txt','Open docs/USER_GUIDE.md. Keep docs/images and docs/examples together. Source-code links open the exact GitHub version. Companion setup is described in docs/RELEASE-COMPONENTS.md.\n')
    with zipfile.ZipFile(path) as archive:
        assert archive.testzip() is None
        for name in archive.namelist():
            if not name.endswith('.md'):continue
            for href in re.findall(r'\[[^\]]*\]\(([^)]+)\)',archive.read(name).decode('utf-8')):
                if re.match(r'^(?:[a-zA-Z][a-zA-Z0-9+.-]*:|#|/)',href):continue
                target=posixpath.normpath(posixpath.join(posixpath.dirname(name),urllib.parse.unquote(href.split('#')[0])))
                assert target in archive.namelist(),'Broken packaged guide link: '+name+': '+href

def verified(expected,actual):
    assert actual['state']=='uploaded' and actual['size']==expected['size'] and actual['digest']=='sha256:'+expected['sha256'],'Remote release asset differs; refusing overwrite'
    return actual

def read_assets(request,base,release_id):
    result=[]
    for page in range(1,12):
        batch=request(f'{base}/releases/{release_id}/assets?per_page=100&page={page}')
        assert isinstance(batch,list) and len(batch)<=100
        result.extend(batch)
        if len(batch)<100:break
    else:raise AssertionError('Release assets exceed transfer budget')
    assert len(result)<=1000 and len({x['name'] for x in result})==len(result)
    return result

def transfers(expected,existing,upload,workers=4):
    assert 1<=workers<=4 and len(expected)<=1000
    names=set()
    for item in expected:
        name=item['name'];assert PurePosixPath(name).name==name and '\\' not in name and ':' not in name and name not in ('.','..') and re.fullmatch(r'[a-f0-9]{64}',item['sha256']) and item['size']>0
        assert name not in names,'Duplicate expected release filename';names.add(name)
    assert all(x['name'] in names for x in existing),'Unexpected assets in owned draft'
    by_name={x['name']:x for x in existing};assert len(by_name)==len(existing)
    pending=[]
    for item in expected:
        if item['name'] in by_name:verified(item,by_name[item['name']])
        else:pending.append(item)
    with ThreadPoolExecutor(max_workers=workers) as pool:
        futures={pool.submit(upload,item):item for item in pending}
        try:
            for future in as_completed(futures):
                item=futures[future];yield item,verified(item,future.result())
        except BaseException:
            for future in futures:future.cancel()
            raise
