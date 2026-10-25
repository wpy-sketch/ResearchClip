param([Parameter(Mandatory)][string]$FilePath,[Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]{1,48}$')][string]$RunId,[switch]$OwnedWindowOnly,[switch]$Paper,[switch]$DocumentNavigation,[switch]$Citations)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$source=[IO.Path]::GetFullPath((Join-Path $repo $FilePath))
$allowed=[IO.Path]::GetFullPath((Join-Path $repo 'artifacts'))+[IO.Path]::DirectorySeparatorChar
if (!$source.StartsWith($allowed,[StringComparison]::OrdinalIgnoreCase) -or !(Test-Path -LiteralPath $source -PathType Leaf)) { throw 'Only an existing test-owned Markdown file is allowed.' }
$out=Join-Path $repo "artifacts/plain-reader-$RunId"
if (Test-Path -LiteralPath $out) { throw 'Use a new evidence directory.' }
New-Item -ItemType Directory -Path $out | Out-Null
$report=[ordered]@{mode='actual Windows Notepad, isolated copy; no user editor is closed';result='running'}
$owner=$null;$handle=[IntPtr]::Zero
try {
 if (!$OwnedWindowOnly -and @(Get-Process -Name Notepad -ErrorAction SilentlyContinue).Count -gt 0) { throw 'Existing Notepad session found; refuse to disturb user tabs. Manual editor acceptance remains required.' }
 if ($OwnedWindowOnly -and ![IO.Path]::GetFileName($source).StartsWith('plain-editor-check-')) { throw 'Resume only the previously created exact test file.' }
 $target=Join-Path $out "plain-editor-check-$RunId.md"
 [IO.File]::Copy($source,$target,$false)
 $expected=[IO.File]::ReadAllText($target,[Text.UTF8Encoding]::new($false,$true))
 $report.fileSha256=(Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash
 Add-Type -AssemblyName UIAutomationClient
 Add-Type -AssemblyName UIAutomationTypes
 Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
using System.Collections.Generic;
public static class ResearchClipNotepadWindow {
 delegate bool Callback(IntPtr window,IntPtr data);
 [DllImport("user32.dll")] static extern bool EnumWindows(Callback callback,IntPtr data);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr window,StringBuilder text,int count);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window,out uint pid);
 [DllImport("user32.dll")] static extern bool EnumChildWindows(IntPtr parent,Callback callback,IntPtr data);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr window,StringBuilder text,int count);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr SendMessageTimeout(IntPtr window,uint msg,IntPtr wParam,IntPtr lParam,uint flags,uint timeout,out IntPtr result);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr SendMessageTimeout(IntPtr window,uint msg,IntPtr wParam,StringBuilder lParam,uint flags,uint timeout,out IntPtr result);
 [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr window,uint msg,IntPtr wParam,IntPtr lParam);
 public static string[] Texts(IntPtr parent) {var texts=new List<string>();EnumChildWindows(parent,(h,p)=>{var kind=new StringBuilder(256);GetClassName(h,kind,kind.Capacity);if(kind.ToString().IndexOf("edit",StringComparison.OrdinalIgnoreCase)>=0){IntPtr length;SendMessageTimeout(h,14,IntPtr.Zero,IntPtr.Zero,2,1000,out length);long n=length.ToInt64();if(n>0&&n<2097152){var value=new StringBuilder((int)n+1);IntPtr done;SendMessageTimeout(h,13,new IntPtr(value.Capacity),value,2,1000,out done);texts.Add(value.ToString());}}return true;},IntPtr.Zero);return texts.ToArray();}
 public static IntPtr Find(string exactFile) {IntPtr found=IntPtr.Zero;EnumWindows((h,p)=>{var name=new StringBuilder(1024);GetWindowText(h,name,name.Capacity);if(name.ToString().Contains(exactFile)){found=h;return false;}return true;},IntPtr.Zero);return found;}
}
'@
 $started=Get-Date
 $windowFile=$target
 if ($OwnedWindowOnly) {
  $windowFile=$source
  $roots=@(Get-CimInstance Win32_Process -Filter "Name='Notepad.exe'" | Where-Object { $_.CommandLine.Contains($source) })
  if ($roots.Count -ne 1) { throw 'Original test-owned Notepad launcher could not be proven.' }
  $allowedIds=@([uint32]$roots[0].ProcessId)
  $allowedIds+=@(Get-CimInstance Win32_Process -Filter "Name='Notepad.exe'" | Where-Object { $_.ParentProcessId -eq $roots[0].ProcessId } | ForEach-Object { [uint32]$_.ProcessId })
  $started=$roots[0].CreationDate
 } else { $launcher=Start-Process -FilePath (Join-Path $env:windir 'System32/notepad.exe') -ArgumentList ('"'+$target+'"') -PassThru -WindowStyle Hidden }
 $deadline=(Get-Date).AddSeconds(20)
 while ((Get-Date) -lt $deadline) {
  $handle=[ResearchClipNotepadWindow]::Find([IO.Path]::GetFileName($windowFile))
  if ($handle -ne [IntPtr]::Zero) { break }
  Start-Sleep -Milliseconds 200
 }
 if ($handle -eq [IntPtr]::Zero) { throw 'Owned Notepad file window unavailable; ordinary-editor UI check not verified.' }
 [uint32]$ownerId=0;[void][ResearchClipNotepadWindow]::GetWindowThreadProcessId($handle,[ref]$ownerId)
 $owner=Get-Process -Id $ownerId
 if ($owner.ProcessName -ne 'Notepad' -or $owner.StartTime -lt $started.AddSeconds(-2)) { throw 'Refuse to inspect an unowned editor process.' }
 if ($OwnedWindowOnly -and $ownerId -notin $allowedIds) { throw 'File window is not owned by the verified test launcher.' }
 $window=[Windows.Automation.AutomationElement]::FromHandle($handle)
 $conditions=[Windows.Automation.OrCondition]::new([Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::ControlTypeProperty,[Windows.Automation.ControlType]::Document),[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::ControlTypeProperty,[Windows.Automation.ControlType]::Edit))
 $matched=$false;$needles=if($Citations){@('混合科研练习','\newcommand','assets/researchclip-','researchclip://literature/','rc-bibliography:v1:start','rc-citation-source: eq:source')}elseif($DocumentNavigation){@('混合科研练习','\newcommand','assets/researchclip-','RC_FIND_TOKEN','researchclip://document/')}elseif($Paper){@('数学论文练习',':::theorem','rc-label: eq:matrix','rc-ref:eq:derivation','\newcommand','| 参数 |')}else{@('混合科研练习','\newcommand','assets/researchclip-','fixture_text','| 参数 |')}
 foreach ($text in [ResearchClipNotepadWindow]::Texts($handle)) { if (@($needles | Where-Object { !$text.Contains($_) }).Count -eq 0) { $matched=$true;$report.textProvider='actual owned Win32 RichEdit WM_GETTEXT';break } }
 while ((Get-Date) -lt $deadline -and !$matched) {
  foreach ($node in $window.FindAll([Windows.Automation.TreeScope]::Descendants,$conditions)) {
   $pattern=$null;$text=$null
   if ($node.TryGetCurrentPattern([Windows.Automation.TextPattern]::Pattern,[ref]$pattern)) { $text=$pattern.DocumentRange.GetText(-1) }
   elseif ($node.TryGetCurrentPattern([Windows.Automation.ValuePattern]::Pattern,[ref]$pattern)) { $text=$pattern.Current.Value }
   if ($null -ne $text -and @($needles | Where-Object { !$text.Contains($_) }).Count -eq 0) { $matched=$true;break }
  }
  if (!$matched) { Start-Sleep -Milliseconds 200 }
 }
 if (!$matched) { throw 'Expected Chinese, formula, table, image URL and code were not verified in the editor UI.' }
 if ([IO.File]::ReadAllText($target,[Text.UTF8Encoding]::new($false,$true)) -cne $expected) { throw 'Reader modified its owned file unexpectedly.' }
 $report.result='PASS';$report.checks=@('Notepad UI exposes actual Chinese Markdown, LaTeX, table, relative image URL and code as readable text.','Read-only editor check preserves the exact UTF-8 fixture bytes.')
} catch { $report.result='FAIL';$report.failure=$_.Exception.Message }
finally {
 if ($null -ne $owner -and !$owner.HasExited -and $handle -ne [IntPtr]::Zero) {
  if ([ResearchClipNotepadWindow]::Find([IO.Path]::GetFileName($windowFile)) -eq $handle) { [void][ResearchClipNotepadWindow]::PostMessage($handle,16,[IntPtr]::Zero,[IntPtr]::Zero) }
 }
 $report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $out 'report.json') -Encoding utf8
 $report | ConvertTo-Json -Depth 8
}
if ($report.result -ne 'PASS') { exit 1 }
