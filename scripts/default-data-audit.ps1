param(
 [Parameter(Mandatory)][ValidateSet('snapshot','verify')][string]$Mode,
 [Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]{1,48}$')][string]$RunId
)
$ErrorActionPreference='Stop'
$root=(Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$outRoot=[IO.Path]::GetFullPath((Join-Path $root 'artifacts/dock-evaluation'))
$before=Join-Path $outRoot ($RunId+'-user-data-before.json')
$resultPath=Join-Path $outRoot ($RunId+'-user-data-integrity.json')
$entries=[Collections.Generic.SortedDictionary[string,string]]::new([StringComparer]::Ordinal)
foreach($kind in @('ApplicationData','LocalApplicationData')) {
 $dataRoot=Join-Path ([Environment]::GetFolderPath($kind)) 'io.researchclip.desktop'
 $entries.Add("$kind/exists",[string](Test-Path -LiteralPath $dataRoot))
 if(!(Test-Path -LiteralPath $dataRoot)){continue}
 $dirs=[Collections.Generic.Queue[string]]::new();$dirs.Enqueue($dataRoot)
 while($dirs.Count){$dir=$dirs.Dequeue();foreach($entry in Get-ChildItem -Force -LiteralPath $dir){
  if(($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0){throw 'Refuse to follow user-data reparse paths.'}
  if($kind -eq 'LocalApplicationData' -and $entry.PSIsContainer -and $entry.FullName -eq (Join-Path $dataRoot 'EBWebView')){continue}
  if($entry.PSIsContainer){$dirs.Enqueue($entry.FullName)}else{$entries.Add("$kind/$([IO.Path]::GetRelativePath($dataRoot,$entry.FullName))",(Get-FileHash -LiteralPath $entry.FullName -Algorithm SHA256).Hash)}
 }}
}
if($Mode -eq 'snapshot'){
 if(Test-Path -LiteralPath $before){throw 'Use a new run ID; previous baseline is retained.'}
 $entries|ConvertTo-Json|Set-Content -Encoding utf8 -LiteralPath $before
 Write-Output ('Read-only baseline files: '+($entries.Count-2));exit 0
}
if(Test-Path -LiteralPath $resultPath){throw 'Previous integrity evidence is retained; use a new run ID.'}
$old=Get-Content -Raw -LiteralPath $before|ConvertFrom-Json -AsHashtable
$changed=0
foreach($key in $old.Keys){if(!$entries.ContainsKey($key) -or $old[$key] -ne $entries[$key]){$changed++}}
foreach($key in $entries.Keys){if(!$old.ContainsKey($key)){$changed++}}
$report=@{mode='read-only default AppData hash comparison; WebView cache excluded; not a backup/restore feature';files=$old.Count-2;changed=$changed;result=$(if($changed -eq 0){'PASS'}else{'FAIL'})}
$report|ConvertTo-Json|Set-Content -Encoding utf8 -LiteralPath $resultPath
$report|ConvertTo-Json -Compress
if($changed -ne 0){throw 'User data differs; investigate without deleting or restoring files.'}
