"""Package exact previously public attachments. This is a release archive, not app data backup."""
import hashlib,json,re,shutil,zipfile
from pathlib import Path

ROOT=Path(__file__).resolve().parent.parent
def sha(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

def main():
    index=json.loads((ROOT/'artifacts/history-assets/index.json').read_text(encoding='utf-8'))
    assert index['releases']==31 and len(index['assets'])==226
    assert {x['tag'] for x in index['assets']} >= {'v4.1.1','v4.1.0','v3.1.2'}
    out=ROOT/'artifacts/companions-v412';out.mkdir()
    contents=out/'HISTORY-CONTENTS.json';contents.write_text(json.dumps(index,ensure_ascii=False,indent=2),encoding='utf-8')
    archive=out/'ResearchClip-History-through-v4.1.1.zip'
    with zipfile.ZipFile(archive,'x',allowZip64=True) as target:
        target.write(contents,contents.name,compress_type=zipfile.ZIP_DEFLATED)
        for item in index['assets']:
            assert re.fullmatch(r'v[0-9a-z.-]+',item['tag']) and Path(item['name']).name==item['name']
            source=ROOT/'artifacts/history-assets'/item['sha256']
            assert source.is_file() and source.stat().st_size==item['size'] and sha(source)==item['sha256']
            compress=zipfile.ZIP_DEFLATED if Path(item['name']).suffix.lower() in ('.exe','.txt','.json','.ps1','.md','.cmd','.bat') else zipfile.ZIP_STORED
            target.write(source,item['tag']+'/'+item['name'],compress_type=compress,compresslevel=6)
    with zipfile.ZipFile(archive) as target:
        assert len(target.infolist())==len(index['assets'])+1
        for item in index['assets']:
            info=target.getinfo(item['tag']+'/'+item['name']);assert info.file_size==item['size']
            with target.open(info) as stream:assert hashlib.file_digest(stream,'sha256').hexdigest()==item['sha256']
    print('All 226 historic archive members verified against GitHub SHA-256',flush=True)
    assets=[]
    def asset(path):assets.append({'name':path.name,'size':path.stat().st_size,'sha256':sha(path)})
    for name in ['ResearchClip-WebClipper-v1.0.2-windows-x64.zip','ResearchClip-MathOCR-Setup-v1.0.4.zip','QWEN-MODEL-MANIFEST.json','MODEL-RESOURCES.txt','OFFLINE-RESOURCE-INDEX.json']:
        entry=next(x for x in index['assets'] if x['tag']=='v4.0.0' and x['name']==name);source=ROOT/'artifacts/history-assets'/entry['sha256'];destination=out/name
        shutil.copyfile(source,destination);assert sha(destination)==entry['sha256'];asset(destination)
    parts=[]
    with archive.open('rb') as stream:
        while True:
            data=stream.read(16*1024*1024)
            if not data:break
            destination=out/(archive.name+'.'+str(len(parts)+1).zfill(3))
            with destination.open('xb') as output:output.write(data)
            info={'name':destination.name,'size':len(data),'sha256':hashlib.sha256(data).hexdigest()};parts.append(info);assets.append(info)
    transport={'schemaVersion':1,'desktopVersion':'4.1.2','throughVersion':'4.1.1','releases':31,'sourceAssets':226,'sourceBytes':sum(x['size'] for x in index['assets']),'archive':{'name':archive.name,'size':archive.stat().st_size,'sha256':sha(archive)},'parts':parts}
    manifest=out/'HISTORY-TRANSPORT.json';manifest.write_text(json.dumps(transport,ensure_ascii=False,indent=2),encoding='utf-8');asset(manifest);asset(contents)
    assembler=out/'Assemble-Release-History.ps1';shutil.copyfile(ROOT/'scripts/assemble-release-history.ps1',assembler);asset(assembler)
    readme=out/'HISTORY-README.txt';readme.write_text('ResearchClip v4.1.2 historical release attachments\n\nAll 226 attachments from 31 releases through v4.1.1 are included byte-for-byte.\nDownload every ResearchClip-History-through-v4.1.1.zip.NNN volume, HISTORY-TRANSPORT.json and Assemble-Release-History.ps1 into a new folder.\nRun PowerShell -NoProfile -File .\\Assemble-Release-History.ps1. The script verifies each SHA-256 and reconstructs a ZIP without overwriting files.\nExtract that ZIP yourself into a new directory. Each version has its own folder, preserving conflicting names such as SOURCE.txt and SHA256SUMS.txt.\nHISTORY-CONTENTS.json records original URLs, asset IDs, sizes and SHA-256. Historical executables are not the new desktop application.\nThe v3.1.2 folder contains its original offline model volumes and transport parts. Follow its own verified assembly instructions to obtain the model runtime; do not run its older desktop EXE instead of v4.1.2.\nWebClipper, MathOCR setup and model indexes are also provided directly as release assets. Existing installations can reuse their components.\nThis archive contains only previously public release files, no user research data. No old attachment is executed during packaging.\n',encoding='utf-8');asset(readme)
    (out/'manifest.json').write_text(json.dumps({'version':'4.1.2','history':transport,'assets':assets},ensure_ascii=False,indent=2),encoding='utf-8')
    print(json.dumps({'result':'PASS','historicAssets':226,'parts':len(parts),'archiveBytes':archive.stat().st_size,'releaseCompanions':len(assets)}),flush=True)
if __name__=='__main__':main()
