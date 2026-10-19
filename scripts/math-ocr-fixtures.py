import json,pathlib,io,argparse
import matplotlib
matplotlib.use('Agg')
matplotlib.rcParams['mathtext.fontset']='stix'
parser=argparse.ArgumentParser();parser.add_argument('output');args=parser.parse_args()
from matplotlib.mathtext import math_to_image
from matplotlib.font_manager import FontProperties,findfont
from PIL import Image,ImageDraw,ImageFont
root=pathlib.Path(args.output);root.mkdir(exist_ok=False)
items=[('quadratic',r'V(x)=x^T P x'),('fraction',r'\frac{dV}{dt}<0'),('scripts',r'x_i^{2}+y_{k+1}'),('greek',r'\alpha+\beta=\lambda\gamma'),('integral',r'\int_0^1 x^2\,dx=\frac{1}{3}'),('sum',r'\sum_{i=1}^{n}i=\frac{n(n+1)}{2}'),('norm',r'\frac{\|x-p\|}{v_P}=\frac{\|x-e\|}{v_E}'),('brackets',r'\left(\frac{a+b}{c+d}\right)^2'),('lowres',r'V(x)=x^T P x')]
manifest=[]
for kind,source in items:
 b=io.BytesIO();math_to_image('$'+source+'$',b,prop=FontProperties(size=28),dpi=180,format='png',color='black');b.seek(0);im=Image.open(b).convert('RGB');white=Image.new('RGB',(im.width+50,im.height+40),'white');white.paste(im,(25,20))
 if kind=='lowres':white=white.resize((max(1,white.width//4),max(1,white.height//4)))
 name=kind+'.png';white.save(root/name);manifest.append({'kind':kind,'file':name,'latex':source})
font=ImageFont.truetype(findfont('STIXGeneral'),52)
im=Image.new('RGB',(310,190),'white');d=ImageDraw.Draw(im);d.line([(60,20),(40,20),(40,170),(60,170)],fill='black',width=3);d.line([(250,20),(270,20),(270,170),(250,170)],fill='black',width=3)
for (x,y,t)in[(90,20,'1'),(185,20,'2'),(90,100,'3'),(185,100,'4')]:d.text((x,y),t,font=font,fill='black')
im.save(root/'matrix.png');manifest.append({'kind':'matrix','file':'matrix.png','latex':r'\begin{bmatrix}1&2\\3&4\end{bmatrix}'})
# Aligned two-line controlled formula layout; labels are mathematical source, not recognition output.
lines=[]
for s in [r'\dot{x}=Ax+Bu',r'y=Cx']:
 b=io.BytesIO();math_to_image('$'+s+'$',b,prop=FontProperties(size=28),dpi=180,format='png',color='black');b.seek(0);lines.append(Image.open(b).convert('RGB'))
im=Image.new('RGB',(max(x.width for x in lines)+50,sum(x.height for x in lines)+65),'white');y=20
for line in lines:im.paste(line,(25,y));y+=line.height+20
im.save(root/'multiline.png');manifest.append({'kind':'multiline','file':'multiline.png','latex':r'\begin{aligned}\dot{x}&=Ax+Bu\\y&=Cx\end{aligned}'})
(root/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf8');print('created',len(manifest),'controlled math images')
