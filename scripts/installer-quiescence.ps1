param([Parameter(Mandatory)][string]$InstalledPath)
$ErrorActionPreference='Stop'
$allowed=(Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../artifacts')).Path+[IO.Path]::DirectorySeparatorChar
$target=[IO.Path]::GetFullPath($InstalledPath)
if(!$target.StartsWith($allowed,[StringComparison]::OrdinalIgnoreCase)-or [IO.Path]::GetFileName($target)-ne 'researchclip.exe'){throw 'Only the owned test image can be probed.'}
# Same read-only API as the shipped guard. No shutdown/restart operation is called.
Add-Type -TypeDefinition @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class InstallerResourceProbe {
 [DllImport("rstrtmgr.dll",CharSet=CharSet.Unicode)] public static extern uint RmStartSession(out uint session,uint flags,StringBuilder key);
 [DllImport("rstrtmgr.dll",CharSet=CharSet.Unicode)] public static extern uint RmRegisterResources(uint session,uint files,string[] paths,uint apps,IntPtr processes,uint services,IntPtr names);
 [DllImport("rstrtmgr.dll")] public static extern uint RmGetList(uint session,out uint needed,ref uint count,IntPtr info,out uint reboot);
 [DllImport("rstrtmgr.dll")] public static extern uint RmEndSession(uint session);
}
'@
$session=0u
$code=[InstallerResourceProbe]::RmStartSession([ref]$session,0,[Text.StringBuilder]::new(33))
if($code-ne 0){throw "Restart Manager session failed: $code"}
try{
 $code=[InstallerResourceProbe]::RmRegisterResources($session,1,[string[]]@($target),0,[IntPtr]::Zero,0,[IntPtr]::Zero)
 if($code-ne 0){throw "Resource registration failed: $code"}
 $codes=[Collections.Generic.List[uint32]]::new()
 for($i=0;$i-lt 20;$i++){
  $needed=0u;$count=0u;$reboot=0u
  $code=[InstallerResourceProbe]::RmGetList($session,[ref]$needed,[ref]$count,[IntPtr]::Zero,[ref]$reboot)
  $codes.Add($code)
  if($code-eq 0){@{result='PASS';codes=$codes;needed=$needed}|ConvertTo-Json -Compress;exit 0}
  if($code-ne 234){throw "Resource query failed: $code"}
  Start-Sleep -Milliseconds 250
 }
 throw 'Owned application resources remain in use; preserve the installation.'
}finally{[InstallerResourceProbe]::RmEndSession($session)|Out-Null}
