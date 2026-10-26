"""Disposable PDF.js crop calibration, never an application PDF export."""
from pathlib import Path
import sys,json,hashlib
root=Path(__file__).resolve().parent.parent
sys.path.insert(0,str(root/'artifacts/dock-evaluation/pdf419-tools'))
from reportlab.pdfgen import canvas
extended='--extended' in sys.argv
out=root/('artifacts/dock-evaluation/pdf423-excerpt-fixture-v2' if extended else 'artifacts/dock-evaluation/pdf423-excerpt-fixture');out.mkdir(exist_ok=False)
p=out/'PDF摘录练习.pdf';base=out/'calibration-base.pdf' if extended else p;c=canvas.Canvas(str(base),pagesize=(600,800),invariant=1)
colors=[(220,50,40),(240,190,30),(30,80,210),(30,150,90)]
for page in range(3 if extended else 2):
 c.setPageSize((700,900) if page==2 else (600,800))
 c.setFont('Helvetica',20);c.setFillColorRGB(.1,.2,.4);c.drawString(70,730,f'Excerpt calibration page {page+1}')
 for (x,y),rgb in zip([(90,750),(140,750),(90,725),(140,725)] if page==2 else [(100,600),(200,600),(100,550),(200,550)],colors):
  c.setFillColorRGB(*(v/255 for v in rgb));c.rect(x,y,50 if page==2 else 100,25 if page==2 else 50,fill=1,stroke=0)
 c.setFillColorRGB(.1,.2,.4);c.setFont('Helvetica',14);c.drawString(100,510,'Select only the four color blocks. Add your own note below.')
 c.showPage()
c.save()
pages=[{'width':600,'height':800,'rotation':0}]*2
if extended:
 from pypdf import PdfReader,PdfWriter
 from pypdf.generic import NameObject,NumberObject,RectangleObject
 reader=PdfReader(base);writer=PdfWriter();writer.pdf_header='%PDF-1.7'
 for i,page in enumerate(reader.pages):
  if i==2:page.cropbox=RectangleObject([40,50,640,850]);page[NameObject('/UserUnit')]=NumberObject(2);page.rotate(90)
  writer.add_page(page)
 with p.open('xb') as stream:writer.write(stream)
 pages.append({'width':1200,'height':1600,'rotation':90,'cropBox':[40,50,640,850],'userUnit':2})
(out/'manifest.json').write_text(json.dumps({'file':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'pageWidth':600,'pageHeight':800,'pages':pages,'rectangle':{'x':100,'y':150,'width':200,'height':100},'colors':colors}),encoding='utf-8')
print(f'Created new {len(pages)}-page excerpt calibration PDF')
