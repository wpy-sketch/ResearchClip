param([Parameter(Mandatory)][string]$OutputDirectory)
$ErrorActionPreference='Stop'
$root=[IO.Path]::GetFullPath($OutputDirectory)
if(Test-Path -LiteralPath $root){throw 'Use a new fixture directory; never overwrite existing images.'}
New-Item -ItemType Directory -Path $root | Out-Null
Add-Type -AssemblyName System.Drawing
$manifest=[Collections.Generic.List[object]]::new()
for($i=1;$i -le 30;$i++){
 $kind=if($i -le 10){'Chinese'}elseif($i -le 20){'English'}else{'mixed'}
 $keyword=if($kind -eq 'English'){'Research'}else{'科研'}
 $lines=switch($kind){
 'Chinese' {@("科研控制理论与实验记录 第 $i 页",'中文图片支持全文检索，请对照原图检查。')}
 'English' {@("Research control theory experiment $i",'Local image search and stable systems.')}
 'mixed' {@("科研 Research experiment $i",'中文 English control model 测试内容。')}
 }
 $image=[Drawing.Bitmap]::new(1400,300)
 $graphics=[Drawing.Graphics]::FromImage($image)
 $font=[Drawing.Font]::new('Microsoft YaHei',38,[Drawing.FontStyle]::Regular,[Drawing.GraphicsUnit]::Pixel)
 try{
  $graphics.Clear([Drawing.Color]::White)
  $graphics.TextRenderingHint=[Drawing.Text.TextRenderingHint]::AntiAliasGridFit
  $graphics.DrawString($lines[0],$font,[Drawing.Brushes]::Black,40,55)
  $graphics.DrawString($lines[1],$font,[Drawing.Brushes]::Black,40,140)
  $filename='text-{0:d2}.png' -f $i
  $image.Save((Join-Path $root $filename),[Drawing.Imaging.ImageFormat]::Png)
  $manifest.Add([ordered]@{file=$filename;kind=$kind;keyword=$keyword;text=($lines -join "`n")})
 }finally{$font.Dispose();$graphics.Dispose();$image.Dispose()}
}
[IO.File]::WriteAllText((Join-Path $root 'manifest.json'),(ConvertTo-Json -InputObject $manifest -Depth 4),[Text.UTF8Encoding]::new($false))
Write-Output "Created $($manifest.Count) owned OCR images. No screen or user content was read."
