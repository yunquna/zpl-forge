from pathlib import Path
import pymupdf,zxingcpp,json,sys
from PIL import Image
r=Path(sys.argv[1]);report=[]
expected={2:b'[)>\x1e01\x1d96336091062\x1d840\x1d002\x1d1Z12345678\x1dUPSN\x1e\x04',3:b'[)>\x1e01\x1d96K1A0B1\x1d124\x1d001\x1d1Z12345678\x1dUPSN\x1e\x04',4:b'HELLO MAXICODE 123456789'}
for p in sorted(r.glob('mode*.pdf')):
 mode=int(p.stem[4]);dpi=int(p.stem.split('-')[1]);doc=pymupdf.open(p);page=doc[0]
 assert '中文箱标' in page.get_text()
 assert not page.get_images(), p
 pix=page.get_pixmap(dpi=dpi);pix.save(str(p.with_suffix('.preview.png')))
for p in sorted(r.glob('mode*.png')):
 mode=int(p.stem[4]);dpi=int(p.stem.split('-')[1].split('.')[0]);im=Image.open(p)
 # ZXing-C++ MaxiCode reader expects an isolated, axis-aligned symbol.
 crop=im.crop((30,90,30+dpi,90+round(dpi*193/200)))
 results=zxingcpp.read_barcodes(crop,formats=zxingcpp.BarcodeFormat.MaxiCode,is_pure=True)
 assert len(results)==1 and results[0].valid,(p,results)
 assert results[0].bytes == expected[mode],(p,results[0].bytes)
 report.append({'file':p.name,'mode':mode,'dpi':dpi,'decoded':True,'isolatedSymbol':True})
print(json.dumps(report,indent=2));(r/'decode-results.json').write_text(json.dumps(report,indent=2)+'\n')
