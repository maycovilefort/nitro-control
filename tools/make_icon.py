#!/usr/bin/env python3
"""Gera icons/source.png (1024x1024) com um ícone original do Nitro Control."""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFilter

S = 1024
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)
d.rounded_rectangle([64, 64, S - 64, S - 64], radius=180, fill=(18, 8, 10, 255))
glow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
g = ImageDraw.Draw(glow)
for dx, col in ((0, (255, 42, 26, 255)), (190, (176, 38, 255, 255))):
    g.polygon([(260 + dx, 300), (420 + dx, 300), (600 + dx, 512), (420 + dx, 724), (260 + dx, 724), (440 + dx, 512)], fill=col)
img.alpha_composite(glow.filter(ImageFilter.GaussianBlur(28)))
img.alpha_composite(glow)
out = Path(__file__).resolve().parent.parent / "src-tauri" / "icons"
out.mkdir(parents=True, exist_ok=True)
img.save(out / "source.png")
print(out / "source.png")
