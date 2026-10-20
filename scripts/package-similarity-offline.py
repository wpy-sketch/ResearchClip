"""Build a verified, split ZIP64 offline distribution; never replace an output.

Python stdlib only. Model/runtime bytes must match the build's fixed allowlist.
The small NSIS remains standalone; this full archive also bundles the model.
"""
import argparse
import hashlib
import json
from pathlib import Path
import zipfile


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--version', required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--bundle', type=Path, required=True)
    parser.add_argument('--exe', type=Path, required=True)
    parser.add_argument('--installer', type=Path, required=True)
    args = parser.parse_args()
    if not all(c in '0123456789.' for c in args.version):
        raise ValueError('Invalid version')
    root = Path(__file__).resolve().parent.parent
    manifest_path = root / 'src-tauri/resources/similarity-ai-manifest.json'
    manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
    for item in manifest['files']:
        path = args.bundle / item['name']
        assert path.is_file() and not path.is_symlink()
        assert path.stat().st_size == item['size'] and digest(path) == item['sha256'], item['name']
    args.out.mkdir(parents=True, exist_ok=False)
    prefix = f'ResearchClip-v{args.version}-offline'
    sources = [(args.exe, f'{prefix}/ResearchClip.exe'),
               (args.installer, f'{prefix}/{args.installer.name}'),
               (manifest_path, f'{prefix}/MODEL-MANIFEST.json')]
    sources += [(args.bundle / f['name'], f'{prefix}/similarity-ai/{f["name"]}') for f in manifest['files']]
    sources += [(args.bundle / 'LICENSE-OpenMP.txt', f'{prefix}/similarity-ai/LICENSE-OpenMP.txt')]
    with (args.out / 'README-OFFLINE.txt').open('x', encoding='utf-8') as stream:
        stream.write('ResearchClip ' + args.version + '\n'
                     'Run ResearchClip.exe beside similarity-ai, or install the included NSIS and select that folder in Local Qwen.\n'
                     'All inference stays local. No model download or HTTP server is required. WebView2 must be installed.\n'
                     'SHA duplicates and perceptual search work without the model. Qwen advice may be wrong/truncated; verify manually.\n'
                     'Qwen3.5-4B Q4_K_M + F16 vision projector: Apache-2.0; llama.cpp b11748: MIT; OpenMP: see license.\n'
                     'Sources: https://huggingface.co/unsloth/Qwen3.5-4B-GGUF/tree/e87f176479d0855a907a41277aca2f8ee7a09523\n'
                     'https://github.com/ggml-org/llama.cpp/releases/tag/b11748\n'
                     'The full offline distribution includes several GB of resources; the small installer alone does not.\n')
    sources += [(args.out / 'README-OFFLINE.txt', f'{prefix}/README-OFFLINE.txt')]
    archive = args.out / (prefix + '.zip')
    with archive.open('xb') as stream, zipfile.ZipFile(stream, 'w', zipfile.ZIP_DEFLATED, compresslevel=1, allowZip64=True) as z:
        for source, name in sources:
            z.write(source, name)
            print(json.dumps({'packed': name, 'size': source.stat().st_size}), flush=True)
    pieces = []
    with archive.open('rb') as stream:
        number = 1
        while True:
            first = stream.read(1024 * 1024)
            if not first:
                break
            path = args.out / f'{archive.name}.{number:03d}'
            with path.open('xb') as target:
                target.write(first)
                remaining = 1_500_000_000 - len(first)
                while remaining:
                    chunk = stream.read(min(1024 * 1024, remaining))
                    if not chunk:
                        break
                    target.write(chunk)
                    remaining -= len(chunk)
            pieces.append({'name': path.name, 'size': path.stat().st_size, 'sha256': digest(path)})
            number += 1
    report = {'version': args.version, 'archive': archive.name, 'sha256': digest(archive),
              'size': archive.stat().st_size, 'pieces': pieces,
              'entries': [{'name': name, 'size': source.stat().st_size, 'sha256': digest(source)} for source, name in sources]}
    (args.out / 'offline-manifest.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    part_lines = '\n'.join("  @{ Name = '" + p['name'] + "'; Sha = '" + p['sha256'] + "' }" for p in pieces)
    script = '''# Run in Windows PowerShell 5.1+; local files only, no downloads or launches.
$ErrorActionPreference = 'Stop'
function Get-OfflineHash([string]$LiteralPath) {
  $hasher = [Security.Cryptography.SHA256]::Create()
  $file = [IO.File]::OpenRead($LiteralPath)
  try { return ([BitConverter]::ToString($hasher.ComputeHash($file))).Replace('-', '').ToLowerInvariant() } finally { $file.Dispose(); $hasher.Dispose() }
}
$pieces = @(
PARTS
)
$zipPath = Join-Path $PSScriptRoot 'ARCHIVE'
$destination = Join-Path $PSScriptRoot 'extracted'
if ((Test-Path -LiteralPath $zipPath) -or (Test-Path -LiteralPath $destination)) { throw 'Output already exists. Use a new empty folder; nothing was overwritten.' }
foreach ($piece in $pieces) {
  $part = Join-Path $PSScriptRoot $piece.Name
  if ((Get-OfflineHash $part) -ne $piece.Sha) { throw ('Checksum mismatch: ' + $piece.Name) }
}
$stream = [IO.File]::Open($zipPath, [IO.FileMode]::CreateNew)
try { foreach ($piece in $pieces) { $inputStream = [IO.File]::OpenRead((Join-Path $PSScriptRoot $piece.Name)); try { $inputStream.CopyTo($stream) } finally { $inputStream.Dispose() } } } finally { $stream.Dispose() }
if ((Get-OfflineHash $zipPath) -ne 'ARCHIVESHA') { throw 'Archive checksum mismatch; retained for diagnosis, not extracted.' }
[void][Reflection.Assembly]::LoadWithPartialName('System.IO.Compression.FileSystem')
$archive = [IO.Compression.ZipFile]::OpenRead($zipPath)
try {
  $root = [IO.Path]::GetFullPath($destination) + [IO.Path]::DirectorySeparatorChar
  $total = 0L
  foreach ($entry in $archive.Entries) {
    $target = [IO.Path]::GetFullPath((Join-Path $destination $entry.FullName))
    if (-not $target.StartsWith($root, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe archive path' }
    $total += $entry.Length
    if ($entry.Length -gt 4294967296L -or $total -gt 8589934592L) { throw 'Archive exceeds extraction budget' }
  }
} finally { $archive.Dispose() }
[IO.Compression.ZipFile]::ExtractToDirectory($zipPath, $destination)
Write-Output ('Verified and extracted to: ' + $destination)
'''.replace('PARTS', part_lines).replace('ARCHIVESHA', report['sha256']).replace('ARCHIVE', archive.name)
    (args.out / 'Assemble-Offline.ps1').write_text(script, encoding='utf-8-sig')
    print(json.dumps({'result': 'PASS', 'parts': len(pieces), 'bytes': report['size']}), flush=True)


if __name__ == '__main__':
    main()
