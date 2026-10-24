param()
$ErrorActionPreference = 'Stop'
$historyRoot = [IO.Path]::GetFullPath($PSScriptRoot)
function Ordinary([string]$path) {
  $item = Get-Item -LiteralPath $path -Force
  if ($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'Expected an ordinary release file.' }
  return $item
}
function Digest([string]$path) {
  $null = Ordinary $path
  $stream = [IO.File]::OpenRead($path)
  $algorithm = [Security.Cryptography.SHA256]::Create()
  try { return ([BitConverter]::ToString($algorithm.ComputeHash($stream))).Replace('-','').ToLowerInvariant() }
  finally { $stream.Dispose(); $algorithm.Dispose() }
}
$manifestPath = Join-Path $historyRoot 'HISTORY-TRANSPORT.json'
if ((Ordinary $manifestPath).Length -gt 1048576) { throw 'Release transport manifest exceeds budget.' }
$manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json
if ($manifest.schemaVersion -ne 1 -or $manifest.desktopVersion -ne '4.1.2' -or $manifest.throughVersion -ne '4.1.1' -or $manifest.releases -ne 31 -or $manifest.sourceAssets -ne 226) { throw 'Unsupported historic release manifest.' }
$archive = $manifest.archive
if ($archive.name -cne 'ResearchClip-History-through-v4.1.1.zip' -or $archive.sha256 -notmatch '^[a-f0-9]{64}$' -or $archive.size -le 0 -or $archive.size -gt 8589934592 -or @($manifest.parts).Count -lt 1 -or @($manifest.parts).Count -gt 512) { throw 'Unsafe release archive metadata.' }
$target = Join-Path $historyRoot $archive.name
if (Test-Path -LiteralPath $target) {
  if ((Ordinary $target).Length -eq $archive.size -and (Digest $target) -ceq $archive.sha256) { Write-Output 'Existing historic release archive verified; no file was changed.'; exit 0 }
  throw 'Existing target differs; refusing overwrite.'
}
$sequence = 0
$total = 0L
foreach ($part in $manifest.parts) {
  $sequence++
  if ($part.name -cne ($archive.name + '.' + $sequence.ToString('000')) -or $part.sha256 -notmatch '^[a-f0-9]{64}$' -or $part.size -le 0 -or $part.size -gt 16777216) { throw 'Unsafe or unordered release volume.' }
  $source = Join-Path $historyRoot $part.name
  if ((Ordinary $source).Length -ne $part.size -or (Digest $source) -cne $part.sha256) { throw ('Missing or damaged volume: ' + $part.name) }
  $total += $part.size
}
if ($total -ne $archive.size) { throw 'Release volume total differs from archive size.' }
$temporary = $target + '.new-' + [Guid]::NewGuid().ToString('N')
$output = [IO.File]::Open($temporary,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
try {
  foreach ($part in $manifest.parts) {
    $historyPartInput = [IO.File]::OpenRead((Join-Path $historyRoot $part.name))
    try { $historyPartInput.CopyTo($output,1048576) } finally { $historyPartInput.Dispose() }
  }
  $output.Flush($true)
} finally { $output.Dispose() }
if ((Digest $temporary) -cne $archive.sha256) { throw 'Reconstructed archive hash differs; inspect the retained .new file.' }
[IO.File]::Move($temporary,$target)
Write-Output 'Historic release archive reconstructed and SHA-256 verified. Extract it into a new folder yourself; no old executable was run.'
