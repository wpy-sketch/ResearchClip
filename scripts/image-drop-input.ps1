param([Parameter(Mandatory=$true)][int]$TargetProcessId,[Parameter(Mandatory=$true)][double]$RelativeX,[Parameter(Mandatory=$true)][double]$RelativeY,[Parameter(Mandatory=$true)][string]$FilesJson)
$ErrorActionPreference='Stop'
if ($RelativeX -le 0 -or $RelativeX -ge 1 -or $RelativeY -le 0 -or $RelativeY -ge 1) { throw 'Invalid client coordinates' }
$testFiles = ConvertFrom-Json -InputObject $FilesJson
$artifactRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../artifacts')) + [IO.Path]::DirectorySeparatorChar
foreach ($testFile in $testFiles) {
  $resolvedFile = [IO.Path]::GetFullPath($testFile)
  if (-not $resolvedFile.StartsWith($artifactRoot,[StringComparison]::OrdinalIgnoreCase) -or -not (Test-Path -LiteralPath $resolvedFile -PathType Leaf)) { throw 'Drag source must be a test-owned artifacts file' }
}
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies System.Windows.Forms,System.Drawing -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Windows.Forms;
using System.Drawing;
public class ResearchClipNativeDrop {
  [StructLayout(LayoutKind.Sequential)] public struct Point { public int X,Y; }
  [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left,Top,Right,Bottom; }
  [DllImport("user32.dll")] static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr window);
  [DllImport("user32.dll")] static extern bool IsChild(IntPtr parent,IntPtr child);
  [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr window,IntPtr after,int x,int y,int width,int height,uint flags);
  [DllImport("user32.dll")] static extern bool ShowWindow(IntPtr window,int action);
  [DllImport("user32.dll")] static extern bool SetCursorPos(int x,int y);
  [DllImport("user32.dll")] static extern void mouse_event(uint flags,uint dx,uint dy,uint data,UIntPtr extra);
  [DllImport("user32.dll")] static extern bool GetClientRect(IntPtr window,out Rect rect);
  [DllImport("user32.dll")] static extern bool ClientToScreen(IntPtr window,ref Point point);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window,out uint process);
  [DllImport("user32.dll")] static extern IntPtr WindowFromPoint(Point point);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr window,System.Text.StringBuilder buffer,int count);
  [DllImport("user32.dll")] static extern IntPtr GetAncestor(IntPtr window,uint flags);
  public static string Run(int pid,double x,double y,string[] files) {
    SetProcessDPIAware();var process=System.Diagnostics.Process.GetProcessById(pid);IntPtr target=process.MainWindowHandle;
    if(target==IntPtr.Zero)throw new Exception("Owned target window missing");ShowWindow(target,9);SetWindowPos(target,new IntPtr(-1),0,0,0,0,0x13);SetForegroundWindow(target);Rect bounds;if(!GetClientRect(target,out bounds))throw new Exception("Client rect unavailable");
    var point=new Point{X=(int)((bounds.Right-bounds.Left)*x),Y=(int)((bounds.Bottom-bounds.Top)*y)};ClientToScreen(target,ref point);
    var form=new Form{Text="ResearchClip controlled file drag",Size=new Size(180,100),StartPosition=FormStartPosition.Manual,Location=new System.Drawing.Point(20,20),TopMost=true};
    var label=new Label{Text="Controlled test file drag",Dock=DockStyle.Fill,TextAlign=ContentAlignment.MiddleCenter};form.Controls.Add(label);
    DragDropEffects effect=DragDropEffects.None;string error=null;System.Threading.Thread worker=null;
    label.MouseDown+=(sender,e)=>{try {var data=new DataObject(DataFormats.FileDrop,files);effect=label.DoDragDrop(data,DragDropEffects.Copy);}catch(Exception ex){error=ex.Message;}finally{form.Close();}};
    form.Shown+=(sender,e)=>{var start=label.PointToScreen(new System.Drawing.Point(60,30));worker=new System.Threading.Thread(()=>{
      try {System.Threading.Thread.Sleep(350);mouse_event(4,0,0,0,UIntPtr.Zero);SetCursorPos(start.X,start.Y);mouse_event(2,0,0,0,UIntPtr.Zero);System.Threading.Thread.Sleep(500);
        uint owner;GetWindowThreadProcessId(GetAncestor(WindowFromPoint(point),2),out owner);if(owner!=pid&&!IsChild(target,WindowFromPoint(point))){var c=new System.Text.StringBuilder(128);GetClassName(GetAncestor(WindowFromPoint(point),2),c,128);throw new Exception("Drop point obscured: expected "+pid+", actual "+owner+", helper "+System.Diagnostics.Process.GetCurrentProcess().Id+", class "+c);}
        SetCursorPos(point.X,point.Y);System.Threading.Thread.Sleep(1000);
      } catch(Exception ex) {error=ex.Message;} finally {mouse_event(4,0,0,0,UIntPtr.Zero);System.Threading.Thread.Sleep(500);if(!form.IsDisposed)try{form.BeginInvoke((Action)(()=>form.Close()));}catch(InvalidOperationException){}}
    });worker.IsBackground=true;worker.Start();};
    try{Application.Run(form);}finally{mouse_event(4,0,0,0,UIntPtr.Zero);SetWindowPos(target,new IntPtr(-2),0,0,0,0,0x13);if(worker!=null)worker.Join(3000);form.Dispose();}
    if(error!=null)throw new Exception(error);return effect.ToString();
  }
}
'@
$result = [ResearchClipNativeDrop]::Run($TargetProcessId,$RelativeX,$RelativeY,[string[]]$testFiles)
if ($result -ne 'Copy') { throw "Native drop was not accepted: $result" }
Write-Output 'NATIVE_OLE_DROP_COPY'
