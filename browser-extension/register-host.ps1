param([ValidateSet('Register','Unregister')][string]$Action='Register',[string]$HostPath)
$ErrorActionPreference='Stop'
if([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT){throw 'Windows is required.'}
$id=[IO.File]::ReadAllText((Join-Path $PSScriptRoot 'protocol/extension-id.txt')).Trim()
if($id -notmatch '^[a-p]{32}$'){throw 'Invalid fixed extension ID.'}
$folder=Join-Path $PSScriptRoot 'native-messaging'
$manifest=[IO.Path]::GetFullPath((Join-Path $folder 'io.researchclip.clipper.json'))
$views=@([Microsoft.Win32.RegistryView]::Registry32,[Microsoft.Win32.RegistryView]::Registry64)
$keys=@('Software\Google\Chrome\NativeMessagingHosts\io.researchclip.clipper','Software\Microsoft\Edge\NativeMessagingHosts\io.researchclip.clipper')
# Preflight every view before modifying any registration. Never replace another install.
foreach($view in $views){$hive=[Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::CurrentUser,$view);try{foreach($key in $keys){$item=$hive.OpenSubKey($key);if($null-ne $item){try{if($item.GetValue('') -ne $manifest -or $item.SubKeyCount -ne 0){throw 'Existing host registration belongs to another installation; no changes made.'}}finally{$item.Dispose()}}}}finally{$hive.Dispose()}}
if($Action-eq 'Register'){
 if(!$HostPath){$HostPath=Join-Path $PSScriptRoot 'bin/researchclip-native-host.exe';if(!(Test-Path -LiteralPath $HostPath)){$HostPath=Join-Path $PSScriptRoot 'target/release/researchclip-native-host.exe'}}
 $exe=(Resolve-Path -LiteralPath $HostPath).Path
 if([IO.Path]::GetFileName($exe)-ne 'researchclip-native-host.exe'){throw 'Unexpected host executable name.'}
 foreach($target in @($PSScriptRoot,$exe,$folder,$manifest)){if(Test-Path -LiteralPath $target){if((Get-Item -Force -LiteralPath $target).Attributes -band [IO.FileAttributes]::ReparsePoint){throw 'Reparse paths are rejected.'}}}
 $json=@{name='io.researchclip.clipper';description='ResearchClip local web clipper';path=$exe;type='stdio';allowed_origins=@("chrome-extension://$id/")}|ConvertTo-Json -Depth 4
 [IO.Directory]::CreateDirectory($folder)|Out-Null
 if(Test-Path -LiteralPath $manifest){$old=[IO.File]::ReadAllText($manifest)|ConvertFrom-Json;if(@($old.PSObject.Properties).Count-ne 5-or $old.name-ne 'io.researchclip.clipper'-or $old.description-ne 'ResearchClip local web clipper'-or $old.path-ne $exe-or $old.type-ne 'stdio'-or @($old.allowed_origins).Count-ne 1-or $old.allowed_origins[0]-ne "chrome-extension://$id/"){throw 'Existing host manifest differs; use its existing location or a new folder. It was not overwritten.'}}
 else{$bytes=[Text.Encoding]::UTF8.GetBytes($json);$file=[IO.File]::Open($manifest,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write);try{$file.Write($bytes,0,$bytes.Length);$file.Flush($true)}finally{$file.Dispose()}}
 foreach($view in $views){$hive=[Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::CurrentUser,$view);try{foreach($key in $keys){$item=$hive.CreateSubKey($key);try{$item.SetValue('',$manifest,[Microsoft.Win32.RegistryValueKind]::String)}finally{$item.Dispose()}}}finally{$hive.Dispose()}}
 Write-Output 'Registered for this user in Chrome/Edge, 32/64-bit views. Enable clipping in the ResearchClip desktop settings.'
}else{
 foreach($view in $views){$hive=[Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::CurrentUser,$view);try{foreach($key in $keys){$hive.DeleteSubKey($key,$false)}}finally{$hive.Dispose()}}
 Write-Output 'Removed only this folder''s native host registrations. Host, documents and images were not deleted.'
}
