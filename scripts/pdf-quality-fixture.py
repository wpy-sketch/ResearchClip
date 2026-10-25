"""Disposable lossless raster PDF for clarity QA; never a product export feature."""
from pathlib import Path
import sys,hashlib,json
ROOT=Path(__file__).resolve().parent.parent
sys.path.insert(0,str(ROOT/'artifacts/dock-evaluation/pdf419-tools'))
from PIL import Image
from reportlab.pdfgen import canvas
from reportlab.lib.utils import ImageReader
out=ROOT/'artifacts/dock-evaluation/pdf421-quality-fixture';out.mkdir(exist_ok=False)
w,h=3000,2400
rows=[bytes(255 if ((x//8)^(y//8))&1 else 0 for x in range(w)) for y in range(16)]
image=Image.frombytes('L',(w,h),b''.join(rows[y%16] for y in range(h)))
pdf=out/'清晰度练习 7.2M.pdf'
c=canvas.Canvas(str(pdf),pagesize=(600,480),pageCompression=1,invariant=1)
c.drawImage(ImageReader(image),0,0,width=600,height=480)
c.setFillColorRGB(1,1,1);c.rect(0,440,600,40,fill=1,stroke=0)
c.setFillColorRGB(.1,.2,.4);c.setFont('Helvetica',16);c.drawString(20,455,'ResearchClip clarity exercise: 7.2M lossless source pixels')
c.showPage();c.setFont('Helvetica',24);c.drawString(30,400,'Vector text: high zoom remains sharp');c.save()
(out/'manifest.json').write_text(json.dumps({'file':pdf.name,'width':w,'height':h,'sha256':hashlib.sha256(pdf.read_bytes()).hexdigest(),'pattern':'gray255 iff ((x//8) XOR (y//8)) & 1; full bleed except header y<200 at 5 source pixels per PDF point'}),encoding='utf-8')
print('Created new isolated raster fixture')
