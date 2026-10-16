param([Parameter(Mandatory=$true)][int]$TargetProcessId, [ValidateSet('inspect','cancel','save','confirm','open')][string]$Action = 'inspect', [string]$TestPath)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$dialogCondition = [System.Windows.Automation.AndCondition]::new(
  [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ProcessIdProperty,$TargetProcessId),
  [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ClassNameProperty,'#32770'))
$dialogs = [System.Windows.Automation.AutomationElement]::RootElement.FindAll([System.Windows.Automation.TreeScope]::Subtree,$dialogCondition)
$owned = @($dialogs | Where-Object { $_.Current.Name -in @('保存截图 PNG','选择截图默认保存目录','保存可编辑项目','打开可编辑项目','导出标注 PNG') })
if ($owned.Count -ne 1) { throw 'Expected exactly one app-owned system file dialog' }
$dialog = $owned[0]
$elements = $dialog.FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition)
if ($Action -eq 'inspect') {
  @($elements | Where-Object { $_.Current.ControlType -in @([System.Windows.Automation.ControlType]::Edit,[System.Windows.Automation.ControlType]::Button,[System.Windows.Automation.ControlType]::ComboBox) } | ForEach-Object { @{type=$_.Current.ControlType.ProgrammaticName; id=$_.Current.AutomationId; name=$_.Current.Name} }) | ConvertTo-Json -Depth 3
  exit 0
}
if ($Action -eq 'cancel') {
  $cancel = @($elements | Where-Object { $_.Current.ControlType -eq [System.Windows.Automation.ControlType]::Button -and ($_.Current.AutomationId -eq '2' -or $_.Current.Name -match '^(取消|Cancel)') })
  if ($cancel.Count -ne 1) { throw 'Expected native Cancel button' }
  $cancel[0].GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
  exit 0
}
if ($Action -eq 'confirm') {
  $confirm = @($elements | Where-Object { $_.Current.ControlType -eq [System.Windows.Automation.ControlType]::Button -and $_.Current.AutomationId -eq '1' })
  if ($confirm.Count -ne 1) { throw 'Expected native primary file dialog button' }
  $confirm[0].GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
  exit 0
}
$artifactRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../artifacts')).Path
$parentPath = (Resolve-Path -LiteralPath (Split-Path -Parent $TestPath)).Path
if (-not $parentPath.StartsWith($artifactRoot + [IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase) -or [IO.Path]::GetExtension($TestPath) -notin @('.png','.json')) { throw 'File helper only accepts an artifacts PNG or JSON' }
if ($Action -eq 'open') {
  if (-not (Test-Path -LiteralPath $TestPath -PathType Leaf)) { throw 'Expected existing test project' }
  $names = @([IO.Path]::GetFileName($TestPath),[IO.Path]::GetFileNameWithoutExtension($TestPath))
  $items = @($elements | Where-Object { $_.Current.Name -in $names -and $_.Current.ControlType -in @([System.Windows.Automation.ControlType]::ListItem,[System.Windows.Automation.ControlType]::DataItem) })
  if ($items.Count -ne 1) { throw ('Expected one project file item in owned dialog; found '+$items.Count) }
  $items[0].GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern).Select()
  $open = @($elements | Where-Object { $_.Current.ControlType -eq [System.Windows.Automation.ControlType]::Button -and $_.Current.AutomationId -eq '1' })
  if ($open.Count -ne 1) { throw 'Expected native Open button' }
  $open[0].GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
  exit 0
}
$edits = @($elements | Where-Object { $_.Current.ControlType -eq [System.Windows.Automation.ControlType]::Edit -and $_.Current.AutomationId -eq '1001' })
if ($edits.Count -ne 1) { throw 'Expected the observed native filename Edit 1001' }
$edits[0].GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue($TestPath)
$save = @($elements | Where-Object { $_.Current.ControlType -eq [System.Windows.Automation.ControlType]::Button -and $_.Current.AutomationId -eq '1' })
if ($save.Count -ne 1) { throw 'Expected native Save button' }
$save[0].GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
