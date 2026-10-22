"""Stage verified compatible companion files and an offline guide; never overwrite."""
import argparse
import hashlib
import json
import re
import posixpath
import shutil
import zipfile
from pathlib import Path, PurePosixPath


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_member(name):
    if '\\' in name or name.startswith('/') or '..' in PurePosixPath(name).parts or ':' in name:
        raise ValueError('Unsafe companion member')


def validate_component(path, expected):
    if path.name != expected['name'] or path.stat().st_size != expected['size'] or sha(path) != expected['sha256']:
        raise ValueError('Companion differs from the verified release: ' + path.name)
    if path.suffix == '.zip':
        with zipfile.ZipFile(path) as archive:
            if archive.testzip() is not None:
                raise ValueError('Corrupt companion archive')
            for name in archive.namelist():
                validate_member(name)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source', type=Path, default=Path('artifacts/v313-final'))
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    config = json.loads((root / 'docs/RELEASE-COMPONENTS.json').read_text(encoding='utf-8'))
    version = config['desktopVersion']
    assert json.loads((root / 'package.json').read_text(encoding='utf-8'))['version'] == version
    source = args.source.resolve()
    # Validate all inputs before creating staging output.
    for item in config['files']:
        if Path(item['name']).name != item['name']:
            raise ValueError('Unsafe component name')
        validate_component(source / item['name'], item)
    out = root / 'artifacts' / ('companions-v' + version.replace('.', ''))
    out.mkdir()  # An existing staging folder is an error.
    for item in config['files']:
        shutil.copyfile(source / item['name'], out / item['name'])
    index = config['offlineResources']
    (out / 'OFFLINE-RESOURCE-INDEX.json').write_text(json.dumps(index, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    (out / 'MODEL-RESOURCES.txt').write_text(
        f'ResearchClip v{version}: optional LOCAL Qwen model/CPU runtime, unchanged from v3.1.2.\n'
        'Already installed similarity-ai: reuse it with the NEW desktop application.\n'
        'First installation: https://github.com/wpy-sketch/ResearchClip/releases/tag/v3.1.2\n'
        'See OFFLINE-RESOURCE-INDEX.json for exact URLs, sizes and SHA256 digests.\n'
        'Original volumes .001-.020 and .025 are direct assets; .021-.024 each have 8 transport parts.\n'
        'Download all transport parts, Restore-Transport-Volumes.ps1 and transport manifests/README; restore four original volumes first.\n'
        'Then use Assemble-Offline.ps1 with all 25 original volumes and offline-manifest.json as documented.\n'
        f'Use ONLY the extracted similarity-ai directory with the v{version} EXE/installer. The old archive EXE is still v3.1.2.\n'
        'The lightweight installer does not contain Python/MathOCR weights or Qwen weights. See docs/RELEASE-COMPONENTS.md in the guide ZIP.\n'
        'Check all hashes. Do not skip missing/corrupt files. Inference is local and results require human review.\n', encoding='utf-8')
    guide = (root / 'docs/USER_GUIDE.md').read_text(encoding='utf-8')
    names = {'docs/USER_GUIDE.md', 'docs/RELEASE-COMPONENTS.md', 'docs/RELEASE-COMPONENTS.json', f'docs/qa/RELEASE-{version}.md', 'browser-extension/README.md', 'browser-extension/PROTOCOL.md', 'docs/qa/RELEASE-1.0.2.md'}
    names.add('docs/ANNOTATION-ANCHORS.md')
    names.update(['docs/COMMAND-WORKFLOWS.md', 'docs/SEMANTIC-SEARCH.md', 'semantic-search/README.md', 'semantic-search/models.json'])
    names.update(str(p.relative_to(root)).replace('\\', '/') for p in (root / 'semantic-search/licenses').iterdir() if p.is_file())
    names.update('docs/' + name for name in re.findall(r'!\[[^\]]*\]\((images/[^)]+)\)', guide))
    names.update(str(p.relative_to(root)).replace('\\', '/') for p in (root / 'docs/examples').iterdir() if p.is_file())
    with zipfile.ZipFile(out / f'ResearchClip-Guide-v{version}.zip', 'x', compression=zipfile.ZIP_DEFLATED) as archive:
        for name in sorted(names):
            if name.endswith('.md'):
                def link(match):
                    label, href = match.groups()
                    if re.match(r'^(?:[a-zA-Z][a-zA-Z0-9+.-]*:|#|/)', href):
                        return match.group(0)
                    target = posixpath.normpath(posixpath.join(posixpath.dirname(name), href.split('#')[0]))
                    if target in names:
                        return match.group(0)
                    assert not target.startswith('../') and (root / target).is_file(), 'Invalid guide link: ' + href
                    fragment = '#' + href.split('#', 1)[1] if '#' in href else ''
                    return f'[{label}](https://github.com/wpy-sketch/ResearchClip/blob/v{version}/{target}{fragment})'
                content = re.sub(r'(?<!!)\[([^\]]+)\]\(([^)]+)\)', link, (root / name).read_text(encoding='utf-8'))
                archive.writestr(name, content)
            else:
                archive.write(root / name, name)
        archive.writestr('README.txt', 'Open docs/USER_GUIDE.md with a Markdown reader. Keep docs/images and docs/examples together.\nSee docs/RELEASE-COMPONENTS.md for companion and optional offline-model installation.\nExamples are fictional exercises, not research results.\n')
    assets = [{'name':p.name, 'size':p.stat().st_size, 'sha256':sha(p)} for p in sorted(out.iterdir())]
    (out / 'manifest.json').write_text(json.dumps({'version':version, 'assets':assets}, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'version':version, 'companions':len(assets), 'images':len(re.findall(r'!\[', guide))}))


if __name__ == '__main__':
    main()
