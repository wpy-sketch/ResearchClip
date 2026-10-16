param([Parameter(Mandatory)][string]$InstalledPath,[Parameter(Mandatory)][string]$Version)
$ErrorActionPreference='Stop'
$root=(Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../artifacts')).Path+[IO.Path]::DirectorySeparatorChar
$target=[IO.Path]::GetFullPath($InstalledPath)
if(!$target.StartsWith($root,[StringComparison]::OrdinalIgnoreCase)-or [IO.Path]::GetFileName($target)-ne 'researchclip.exe'){throw 'Only the owned test installation can be inspected.'}
$info=(Get-Item -LiteralPath $target).VersionInfo
if($info.ProductName-ne 'ResearchClip'-or !$info.ProductVersion.StartsWith($Version)-or $info.CompanyName-ne 'ResearchClip'){throw 'Incorrect installed version/product/publisher.'}
Add-Type -AssemblyName System.Drawing
$icon=[Drawing.Icon]::ExtractAssociatedIcon($target)
if($null-eq $icon){throw 'Installed icon missing.'}
$icon.Dispose()
@{product=$info.ProductName;version=$info.ProductVersion;publisher=$info.CompanyName;hasIcon=$true}|ConvertTo-Json -Compress
