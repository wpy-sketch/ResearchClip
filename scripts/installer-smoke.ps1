param(
  [Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]{1,64}$')][string]$RunId,
  [string]$TargetDir = 'src-tauri/target/release-audit'
)
$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$targetRoot = [IO.Path]::GetFullPath((Join-Path $projectRoot $TargetDir))
$allowedTargetRoot = [IO.Path]::GetFullPath((Join-Path $projectRoot 'src-tauri/target'))
$allowedTarget = $allowedTargetRoot + [IO.Path]::DirectorySeparatorChar
if (!$targetRoot.Equals($allowedTargetRoot, [StringComparison]::OrdinalIgnoreCase) -and !$targetRoot.StartsWith($allowedTarget, [StringComparison]::OrdinalIgnoreCase)) { throw 'Build target must stay in this workspace target directory.' }
$runRoot = Join-Path $projectRoot "artifacts/installer-$RunId"
if (Test-Path -LiteralPath $runRoot) { throw 'Use a new run ID; existing evidence is never overwritten.' }
New-Item -ItemType Directory -Path $runRoot | Out-Null
$installRoot = [IO.Path]::GetFullPath((Join-Path $runRoot '安装验收 Test'))
if (!$installRoot.StartsWith($runRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe installation path.' }
$version = (Get-Content -Raw -LiteralPath (Join-Path $projectRoot 'package.json') | ConvertFrom-Json).version
$installer = Join-Path $targetRoot "release/bundle/nsis/ResearchClip_${version}_x64-setup.exe"
$installedExe = Join-Path $installRoot 'researchclip.exe'
$uninstaller = Join-Path $installRoot 'uninstall.exe'
$backupExe = Join-Path $installRoot 'researchclip.original.exe'
$probe = Join-Path $projectRoot 'src-tauri/target/debug/examples/installer_guard_probe.exe'
$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\ResearchClip'
$metadataKey = 'HKCU:\Software\ResearchClip\ResearchClip'
$report = [ordered]@{version=$version; mode='actual NSIS, isolated install path, read-only user-data hashes, process fixture for busy guard'; checks=[Collections.Generic.List[string]]::new(); commands=[Collections.Generic.List[string]]::new(); result='running'}
$holder = $null
$installed = $false
$swapped = $false
function Assert-Check([bool]$condition, [string]$message) { if (!$condition) { throw $message } }
function Data-Snapshot {
  $entries = [Collections.Generic.SortedDictionary[string,string]]::new([StringComparer]::Ordinal)
  foreach ($kind in @('ApplicationData','LocalApplicationData')) {
    $dataRoot = Join-Path ([Environment]::GetFolderPath($kind)) 'io.researchclip.desktop'
    $entries.Add("$kind/exists", [string](Test-Path -LiteralPath $dataRoot))
    if (!(Test-Path -LiteralPath $dataRoot)) { continue }
    $dirs = [Collections.Generic.Queue[string]]::new(); $dirs.Enqueue($dataRoot)
    while ($dirs.Count) {
      $dir = $dirs.Dequeue()
      foreach ($entry in Get-ChildItem -Force -LiteralPath $dir) {
        Assert-Check (($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) -eq 0) 'Refuse to follow user-data reparse paths during audit.'
        # WebView browser caches change independently of setup. Core files and root existence
        # remain strict; a user-selected app-data wipe would still fail these assertions.
        if ($kind -eq 'LocalApplicationData' -and $entry.PSIsContainer -and $entry.FullName -eq (Join-Path $dataRoot 'EBWebView')) { continue }
        if ($entry.PSIsContainer) { $dirs.Enqueue($entry.FullName) }
        else {
          $relative = [IO.Path]::GetRelativePath($dataRoot,$entry.FullName)
          $entries.Add("$kind/$relative", (Get-FileHash -LiteralPath $entry.FullName -Algorithm SHA256).Hash)
        }
      }
    }
  }
  return ,$entries
}
function Run-Setup([string]$path, [string]$arguments) {
  $report.commands.Add("$path $arguments")
  # Launch the exact image directly, without ShellExecute/hidden-shell relaunch.
  # NSIS default uninstall itself launches a temporary child; inspect actual files
  # and registration below rather than treating its launcher exit as completion.
  $start = [Diagnostics.ProcessStartInfo]::new($path)
  $start.Arguments = $arguments
  $start.UseShellExecute = $false
  $start.CreateNoWindow = $true
  $process = [Diagnostics.Process]::Start($start)
  if (!$process.WaitForExit(60000)) { throw 'Owned setup process timed out; do not force-close applications or claim successful cleanup.' }
  $code=$process.ExitCode; $process.Dispose(); return $code
}
function Stop-Holder {
  if ($null -ne $script:holder -and !$script:holder.HasExited) {
    $script:holder.StandardInput.WriteLine('exit')
    $script:holder.StandardInput.Flush()
    if (!$script:holder.WaitForExit(10000)) { throw 'The controlled process fixture did not stop normally.' }
  }
  if($null -ne $script:holder){$script:holder.Dispose()}
  $script:holder = $null
}
try {
  Assert-Check ($IsWindows) 'Windows is required.'
  Assert-Check (!(Test-Path -LiteralPath $uninstallKey)) 'Existing ResearchClip installation found; refuse to replace it.'
  Assert-Check (!(Test-Path -LiteralPath 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\ResearchClip')) 'Existing machine-wide installation found; use a clean test machine.'
  Assert-Check (!(Test-Path -LiteralPath $metadataKey)) 'Existing installer metadata found; refuse to replace it.'
  Assert-Check (@(Get-Process -Name researchclip -ErrorAction SilentlyContinue).Count -eq 0) 'Please exit existing ResearchClip processes before testing installation.'
  Assert-Check (Test-Path -LiteralPath $installer -PathType Leaf) 'Build the NSIS package first.'
  Assert-Check (Test-Path -LiteralPath $probe -PathType Leaf) 'Build installer_guard_probe first.'
  $generated = Get-Content -Raw -LiteralPath (Join-Path $targetRoot 'release/nsis/x64/installer.nsi')
  Assert-Check ($generated.Contains('installer-hooks.nsh')) 'The generated installer is missing the running-app guard.'
  Assert-Check ((Get-Content -Raw -LiteralPath (Join-Path $projectRoot 'src-tauri/windows/installer-hooks.nsh')).Contains('!macroundef CheckIfAppIsRunning')) 'The safe running-app guard has changed; review before executing.'
  $before = Data-Snapshot
  $before | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 -LiteralPath (Join-Path $runRoot 'data-before.json')
  $report.userDataFilesHashed = $before.Count - 2
  $report.installerSha256 = (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash
  $report.signatureStatus = [string](Get-AuthenticodeSignature -LiteralPath $installer).Status
  $code = Run-Setup $installer "/S /NS /D=$installRoot"
  Assert-Check ($code -eq 0) "Installer returned $code."
  $installed = $true
  Assert-Check ((Test-Path -LiteralPath $installedExe) -and (Test-Path -LiteralPath $uninstaller)) 'Installation did not produce both executables.'
  # Tauri patches UNK to NSS while packaging and restores the standalone EXE afterwards.
  # Account for exactly that unique installed-runtime marker; hash every other byte unchanged.
  $sourceBytes = [IO.File]::ReadAllBytes((Join-Path $targetRoot 'release/researchclip.exe'))
  $sourceText = [Text.Encoding]::ASCII.GetString($sourceBytes)
  $marker = '__TAURI_BUNDLE_TYPE_VAR_UNK'
  $offset = $sourceText.IndexOf($marker,[StringComparison]::Ordinal)
  Assert-Check ($offset -ge 0 -and $sourceText.IndexOf($marker,$offset+1,[StringComparison]::Ordinal) -lt 0) 'Expected one verified Tauri UNK bundle marker.'
  [Text.Encoding]::ASCII.GetBytes('NSS').CopyTo($sourceBytes,$offset+$marker.Length-3)
  $expectedHash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($sourceBytes))
  $report.installedPayloadSha256 = $expectedHash
  Assert-Check ((Get-FileHash -LiteralPath $installedExe -Algorithm SHA256).Hash -eq $expectedHash) 'Installed executable differs from bundled executable.'
  # Read PE resources in a short-lived process: icon/version APIs can retain image mappings.
  if ([Version]$version -ge [Version]'4.1.1') {
    $notice = Join-Path $installRoot 'licenses/flexlayout-react-MIT.txt'
    Assert-Check ((Test-Path -LiteralPath $notice) -and ((Get-FileHash -LiteralPath $notice -Algorithm SHA256).Hash -eq (Get-FileHash -LiteralPath (Join-Path $projectRoot 'docs/licenses/flexlayout-react-MIT.txt') -Algorithm SHA256).Hash)) 'Installed FlexLayout MIT notice missing or changed.'
  }
  $metadata = & (Get-Process -Id $PID).Path -NoProfile -File (Join-Path $PSScriptRoot 'installer-metadata.ps1') -InstalledPath $installedExe -Version $version
  Assert-Check ($LASTEXITCODE -eq 0) 'Installed metadata/icon inspection failed.'
  $report.metadata = $metadata | ConvertFrom-Json
  $record = Get-ItemProperty -LiteralPath $uninstallKey
  Assert-Check ($record.DisplayVersion -eq $version -and $record.InstallLocation.Trim('"') -eq $installRoot) 'Uninstall registration points at a different install/version.'
  $report.checks.Add('Actual per-user silent installation at a Unicode/space path; full payload SHA-256 matches with only the verified Tauri UNK-to-NSS marker, plus version, publisher, icon and uninstall registration.')
  $sentinel = Join-Path $installRoot 'user-owned.keep'
  [IO.File]::WriteAllText($sentinel,'ResearchClip test-owned user file: must survive uninstall.')
  $sentinelHash = (Get-FileHash -LiteralPath $sentinel -Algorithm SHA256).Hash
  # Swap only this new, verified, workspace-owned install file. The fixture opens no user data.
  Move-Item -LiteralPath $installedExe -Destination $backupExe
  $swapped = $true
  [IO.File]::Copy($probe,$installedExe,$false)
  $start = [Diagnostics.ProcessStartInfo]::new($installedExe)
  $start.UseShellExecute = $false; $start.CreateNoWindow = $true
  $start.RedirectStandardInput = $true; $start.RedirectStandardOutput = $true
  $holder = [Diagnostics.Process]::Start($start)
  $ready = $holder.StandardOutput.ReadLineAsync()
  Assert-Check ($ready.Wait(10000) -and $ready.Result -eq 'INSTALLER_GUARD_READY') 'Process fixture did not start.'
  $blocked = Run-Setup $uninstaller "/S _?=$installRoot"
  $report.busyUninstallExit = $blocked
  Assert-Check ($blocked -ne 0 -and !$holder.HasExited -and (Test-Path -LiteralPath $installedExe) -and (Test-Path -LiteralPath $uninstallKey)) 'Busy uninstall did not refuse safely.'
  $blockedInstall = Run-Setup $installer "/S /NS /D=$installRoot"
  $report.busyInstallExit = $blockedInstall
  Assert-Check ($blockedInstall -ne 0 -and !$holder.HasExited -and (Test-Path -LiteralPath $uninstallKey)) 'Busy install did not refuse safely.'
  $report.checks.Add('Actual installer and uninstaller refuse a running image at the installed executable path; process fixture remains alive and files/registration remain intact.')
  Stop-Holder
  Remove-Item -LiteralPath $installedExe
  Move-Item -LiteralPath $backupExe -Destination $installedExe
  $swapped = $false
  Assert-Check ((Get-FileHash -LiteralPath $installedExe -Algorithm SHA256).Hash -eq $expectedHash) 'Production executable was not restored correctly.'
  $quiet = & (Get-Process -Id $PID).Path -NoProfile -File (Join-Path $PSScriptRoot 'installer-quiescence.ps1') -InstalledPath $installedExe
  Assert-Check ($LASTEXITCODE -eq 0) 'Owned test resources are still busy; do not test a successful uninstall yet.'
  $report.quiescence = $quiet | ConvertFrom-Json
  $code = Run-Setup $uninstaller '/S'
  Assert-Check ($code -eq 0) "Uninstaller launcher returned $code."
  $wait = [Diagnostics.Stopwatch]::StartNew()
  while ((Test-Path -LiteralPath $uninstallKey) -or (Test-Path -LiteralPath $installedExe) -or (Test-Path -LiteralPath $uninstaller)) {
    if ($wait.Elapsed.TotalSeconds -gt 30) { throw 'Uninstall did not remove owned application files and uninstall registration.' }
    Start-Sleep -Milliseconds 100
  }
  $installed = $false
  Assert-Check ((Get-FileHash -LiteralPath $sentinel -Algorithm SHA256).Hash -eq $sentinelHash) 'Uninstall removed or changed an unrelated user file.'
  $after = Data-Snapshot
  $after | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 -LiteralPath (Join-Path $runRoot 'data-after.json')
  Assert-Check (($before | ConvertTo-Json -Compress) -eq ($after | ConvertTo-Json -Compress)) 'User data changed during installation/uninstallation.'
  $report.checks.Add('Actual default silent uninstall removes application files/registration, preserves an unrelated install-folder file and all pre-existing AppData file hashes. No production app was launched against user data.')
  if (Test-Path -LiteralPath $metadataKey) {
    $key = Get-Item -LiteralPath $metadataKey
    Assert-Check ($key.GetValue('') -eq $installRoot -and $key.SubKeyCount -eq 0) 'Unexpected installer metadata; preserve it for review.'
    Remove-Item -LiteralPath $metadataKey
  }
  $report.checks.Add('Only newly created, verified test installer metadata was cleaned up; user data and evidence retained.')
  $report.result = 'passed'
} catch {
  $report.result = 'failed'; $report.failure = $_.Exception.Message
} finally {
  try {
    Stop-Holder
    if ($swapped -and (Test-Path -LiteralPath $backupExe)) {
      Remove-Item -LiteralPath $installedExe
      Move-Item -LiteralPath $backupExe -Destination $installedExe
    }
  } catch { $report.cleanupFailure = $_.Exception.Message; $report.result = 'failed' }
  $report.installationStillPresent = $installed
  $report | ConvertTo-Json -Depth 6 | Set-Content -Encoding utf8 -LiteralPath (Join-Path $runRoot 'report.json')
  $report | ConvertTo-Json -Depth 6
}
if ($report.result -ne 'passed') { exit 1 }
