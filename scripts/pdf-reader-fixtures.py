"""Generate new disposable PDF inputs for native QA; never a product export feature.

Requires reportlab==4.4.10 and pypdf==6.4.0 in artifacts/dock-evaluation/pdf419-tools.
Uses their documented Canvas/PdfWriter APIs; no user PDF is opened or changed.
"""
import hashlib,json,sys
from io import BytesIO
from pathlib import Path
ROOT=Path(__file__).resolve().parent.parent
sys.path.insert(0,str(ROOT/'artifacts/dock-evaluation/pdf419-tools'))
from reportlab.pdfgen import canvas
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.cidfonts import UnicodeCIDFont
from reportlab.pdfbase.ttfonts import TTFont
from reportlab import __file__ as reportlab_file
from pypdf import PdfWriter,PdfReader
from pypdf.constants import UserAccessPermissions
from PIL import Image

out=Path(sys.argv[1]).resolve()
assert out.is_relative_to(ROOT/'artifacts') and not out.exists()
out.mkdir(parents=True)
pdfmetrics.registerFont(UnicodeCIDFont('STSong-Light'))
pdfmetrics.registerFont(TTFont('FixtureVera',str(Path(reportlab_file).parent/'fonts/Vera.ttf')))
plain=out/'科研阅读练习.pdf';c=canvas.Canvas(str(plain),pagesize=(612,792),invariant=True)
for i in range(8):
 if i==2:c.setPageSize((420,600))
 elif i==3:c.setPageSize((792,612))
 else:c.setPageSize((612,792))
 c.bookmarkPage(f'p{i+1}');c.addOutlineEntry(f'Chapter {i+1}',f'p{i+1}',level=0)
 c.setFillColorRGB(.05,.2,.45);c.setFont('Helvetica',24);c.drawString(45,540,f'ResearchClip PDF Chapter {i+1}')
 c.setFont('FixtureVera',16);c.drawString(45,500,f'Embedded font: Alpha beta SearchTarget {i+1}')
 c.setFont('STSong-Light',18);c.drawString(45,460,'科研阅读练习：目录、文字和缩放')
 c.setFillColorRGB(.1,.65,.5);c.roundRect(45,270,300,130,16,fill=1,stroke=0)
 c.setFillColorRGB(1,1,1);c.setFont('Helvetica',18);c.drawString(70,330,'Actual PDF vector content')
 c.showPage()
c.save()
# A real JPEG2000 image exercises PDF.js's locally bundled OpenJPEG WASM.
image=Image.new('RGB',(128,128),(20,180,110));buffer=BytesIO();image.save(buffer,format='JPEG2000');encoded=buffer.getvalue()
content=b'q 300 0 0 300 50 200 cm /Im1 Do Q BT /F1 18 Tf 50 540 Td (Local JPEG2000 decoder) Tj ET'
objects=[b'<< /Type /Catalog /Pages 2 0 R >>',b'<< /Type /Pages /Kids [3 0 R] /Count 1 >>',b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /XObject << /Im1 4 0 R >> /Font << /F1 5 0 R >> >> /Contents 6 0 R >>',b'<< /Type /XObject /Subtype /Image /Width 128 /Height 128 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /JPXDecode /Length '+str(len(encoded)).encode()+b' >>\nstream\n'+encoded+b'\nendstream',b'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>',b'<< /Length '+str(len(content)).encode()+b' >>\nstream\n'+content+b'\nendstream']
data=b'%PDF-1.7\n';offsets=[0]
for i,obj in enumerate(objects,1):offsets.append(len(data));data+=f'{i} 0 obj\n'.encode()+obj+b'\nendobj\n'
xref=len(data);data+=f'xref\n0 {len(offsets)}\n0000000000 65535 f \n'.encode()+b''.join(f'{offset:010} 00000 n \n'.encode()for offset in offsets[1:])+f'trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF'.encode()
(out/'JPX解码练习.pdf').write_bytes(data)
w=PdfWriter(clone_from=str(plain));w.pages[1].rotate(90);w.add_js('globalThis.__researchclipPdfScriptExecuted=true; app.alert("UNTRUSTED PDF SCRIPT");');w.add_attachment('untrusted.html',b'<script>alert("attachment")</script>');
with (out/'旋转与脚本练习.pdf').open('xb')as f:w.write(f)
for name,user,permissions in [('加密练习.pdf','practice-password',UserAccessPermissions.PRINT|UserAccessPermissions.EXTRACT),('限制复制练习.pdf','',UserAccessPermissions.PRINT)]:
 w=PdfWriter(clone_from=str(plain));w.encrypt(user,owner_password='fixture-owner-only',algorithm='RC4-128',permissions_flag=permissions)
 with (out/name).open('xb')as f:w.write(f)
(out/'损坏练习.pdf').write_bytes(b'%PDF-1.7\ncorrupt object stream without catalog\n%%EOF')
c=canvas.Canvas(str(out/'长文检索练习.pdf'),invariant=True)
for i in range(160):c.setFont('Helvetica',14);c.drawString(50,700,f'Long document SearchTarget {i+1}');c.showPage()
c.save()
# Valid trailing PDF comments exercise actual 1MiB native chunking/progress.
with (out/'长文检索练习.pdf').open('ab')as f:
 for _ in range(128):f.write(b'\n%'+b'padding '*8192)
manifest={p.name:{'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}for p in out.glob('*.pdf')}
(out/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2),encoding='utf-8')
print(json.dumps({'generated':len(manifest),'out':str(out)},ensure_ascii=False))
