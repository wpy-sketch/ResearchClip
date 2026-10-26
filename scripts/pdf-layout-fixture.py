"""Disposable mixed-page geometry fixture; not an application PDF export feature."""
from pathlib import Path
import sys,json,hashlib
ROOT=Path(__file__).resolve().parent.parent
sys.path.insert(0,str(ROOT/'artifacts/dock-evaluation/pdf419-tools'))
from reportlab.pdfgen import canvas
out=ROOT/'artifacts/dock-evaluation/pdf422-layout-fixture';out.mkdir(exist_ok=False)
p=out/'混合页面练习.pdf';c=canvas.Canvas(str(p),pagesize=(400,600),invariant=1)
for i,(w,h) in enumerate([(400,600),(1000,500),(600,800),(20000,20000)],1):
 c.setPageSize((w,h));c.setFillColorRGB(.1,.25,.5);c.setFont('Helvetica',min(500,w/20));c.drawString(w*.08,h*.85,f'Mixed page {i}: {w} x {h}');c.setStrokeColorRGB(.2,.65,.45);c.setLineWidth(max(2,w/200));c.rect(w*.05,h*.05,w*.9,h*.7);c.showPage()
c.save();(out/'manifest.json').write_text(json.dumps({'file':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'pageSizes':[[400,600],[1000,500],[600,800],[20000,20000]]}),encoding='utf-8')
print('Created new mixed-size PDF fixture')
