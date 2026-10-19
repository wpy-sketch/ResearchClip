param(
 [Parameter(Mandatory)][string]$Python,
 [Parameter(Mandatory)][string]$Destination
)
$ErrorActionPreference='Stop'
$runtimePython=(Resolve-Path -LiteralPath $Python).Path
$target=[IO.Path]::GetFullPath($Destination)
if(Test-Path -LiteralPath $target){throw 'Destination already exists. Use a new directory; existing runtimes are never deleted or overwritten.'}
if([IO.Path]::GetFileName($runtimePython) -ine 'python.exe'){throw 'Select an installed Python python.exe.'}
& $runtimePython -I -c 'import sys;sys.exit(0 if sys.version_info >= (3,12) and sys.version_info < (4,0) else 1)'
if($LASTEXITCODE -ne 0){throw 'Python 3.12 or newer is required. Windows x64 Python 3.12 is the tested runtime.'}
& $runtimePython -m venv $target
if($LASTEXITCODE -ne 0){throw 'Unable to create the runtime. Partial files are retained for inspection.'}
$venvPython=Join-Path $target 'Scripts/python.exe'
& $venvPython -m pip install --disable-pip-version-check --only-binary=:all: --index-url https://pypi.org/simple -r (Join-Path $PSScriptRoot 'requirements-lock.txt')
if($LASTEXITCODE -ne 0){throw 'Dependency installation failed. No model was downloaded. Keep the folder and inspect the pip error.'}
& $venvPython -I -c 'import onnxruntime,numpy,PIL,tokenizers;print("Local MathOCR dependencies ready")'
if($LASTEXITCODE -ne 0){throw 'Native runtime import failed.'}
Write-Output "Select this executable in ResearchClip Settings: $venvPython"
Write-Output 'Models are not installed automatically. Use the explicit download/confirm action in ResearchClip.'
