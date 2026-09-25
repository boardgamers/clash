"""Regenerate browser font subsets. Requires fonttools[woff] and the Noto fonts."""
import argparse
import json
from pathlib import Path
from fontTools import subset
from fontTools.ttLib import TTFont

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--cjk-font', default='/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc')
parser.add_argument('--devanagari-font', default='/usr/share/fonts/truetype/noto/NotoSansDevanagari-Regular.ttf')
args = parser.parse_args()
root = Path(__file__).resolve().parent.parent
output = root / 'client/assets/localization'
output.mkdir(parents=True, exist_ok=True)
for locale, name, index in [('ko', 'Korean', 1), ('zh-TW', 'Chinese', 3), ('hi', 'Devanagari', None)]:
    catalog = json.loads((root / f'client/js/src/localization/{locale}.json').read_text())
    text = ''.join(catalog.values()) + ''.join(chr(i) for i in range(32, 256))
    font = TTFont(args.cjk_font, fontNumber=index) if index is not None else TTFont(args.devanagari_font)
    processor = subset.Subsetter(options=subset.Options())
    processor.populate(text=text)
    processor.subset(font)
    font.flavor = 'woff2'
    font.save(output / f'{name}.woff2')
    print(f'{name}: {(output / f"{name}.woff2").stat().st_size} bytes')
