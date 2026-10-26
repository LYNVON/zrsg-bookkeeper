from PIL import Image, ImageDraw, ImageFilter, ImageFont
import os

brand = r'C:\Users\93783\XiaomiMiMoProjects\zrsj-bookkeeper\assets\brand'
os.makedirs(brand, exist_ok=True)

# ---------- 侧边栏 164x314（欢迎/完成页左侧） ----------
W, H = 164, 314
sb = Image.new('RGB', (W, H), '#1b2240')
d = ImageDraw.Draw(sb)
# 垂直渐变：深靛 → 紫罗兰
for y in range(H):
    t = y / H
    r = int(27 + t * (110 - 27))
    g = int(34 + t * (80 - 34))
    b = int(64 + t * (200 - 64))
    d.line([(0, y), (W, y)], fill=(r, g, b))
# 弥散光斑（极光）
glow = Image.new('RGB', (W, H), (0, 0, 0))
gd = ImageDraw.Draw(glow)
gd.ellipse([-40, 20, 110, 150], fill=(70, 220, 190))
gd.ellipse([60, 150, 210, 290], fill=(230, 130, 200))
gd.ellipse([10, 230, 164, 360], fill=(120, 130, 255))
glow = glow.filter(ImageFilter.GaussianBlur(38))
sb = Image.blend(sb, Image.blend(sb, glow, 0.45), 0.55)
d = ImageDraw.Draw(sb)
# 细装饰线（呼应 logo 放射线）
for i in range(8):
    y0 = 30 + i * 12
    d.line([(0, y0), (W, y0 - 26)], fill=(255, 255, 255, 12), width=1)
# 金币 logo 居中偏上
coin = Image.open(os.path.join(brand, 'app-icon.png')).convert('RGBA')
side = 96
coin = coin.resize((side, side), Image.LANCZOS)
sb.paste(coin, ((W - side) // 2, 62), coin)
# 标题 + slogan（每个字体独立加载，互不拖累）
def load_font(name, size):
    try:
        return ImageFont.truetype(rf'C:\Windows\Fonts\{name}', size)
    except Exception:
        try:
            return ImageFont.truetype(r'C:\Windows\Fonts\msyh.ttc', size)
        except Exception:
            return ImageFont.load_default()

font1 = load_font('msyhbd.ttc', 17)
font2 = load_font('msyh.ttc', 11)
font3 = load_font('STXINGKA.TTF', 15)  # 行楷缺失时自动回退 msyh
d = ImageDraw.Draw(sb)
for text, font, y in [('自然拾光记账', font1, 178), ('本地数据 · 账不上云', font2, 206)]:
    w = d.textlength(text, font=font)
    d.text(((W - w) / 2, y), text, font=font, fill=(245, 246, 252))
try:
    text = '账清财旺'
    w = d.textlength(text, font=font3)
    d.text(((W - w) / 2, 246), text, font=font3, fill=(240, 220, 180))
except Exception:
    pass
# 底部细线 + 英文
d.line([(28, 282), (W - 28, 282)], fill=(255, 255, 255, 90), width=1)
en = 'FORTUNE & CLARITY'
w = d.textlength(en, font=font2)
d.text(((W - w) / 2, 290), en, font=font2, fill=(210, 215, 235))
sb.save(os.path.join(brand, 'installer-sidebar.bmp'), format='BMP')
print('sidebar ok')

# ---------- 页眉图 150x57（页面右上角） ----------
hw, hh = 150, 57
hd = Image.new('RGB', (hw, hh), '#20284a')
d = ImageDraw.Draw(hd)
for y in range(hh):
    t = y / hh
    d.line([(0, y), (hw, y)], fill=(int(32 + t * 20), int(40 + t * 26), int(74 + t * 60)))
glow = Image.new('RGB', (hw, hh), (0, 0, 0))
gd = ImageDraw.Draw(glow)
gd.ellipse([-20, -10, 70, 50], fill=(90, 140, 255))
gd.ellipse([80, 10, 170, 70], fill=(190, 120, 240))
glow = glow.filter(ImageFilter.GaussianBlur(18))
hd = Image.blend(hd, Image.blend(hd, glow, 0.4), 0.5)
d = ImageDraw.Draw(hd)
coin_sm = coin.resize((40, 40), Image.LANCZOS)
hd.paste(coin_sm, (10, 8), coin_sm)
t = '自然拾光记账'
try:
    fw = ImageFont.truetype(r'C:\Windows\Fonts\msyhbd.ttc', 15)
except Exception:
    fw = font1
d.text((58, 12), t, font=fw, fill=(245, 246, 252))
d.text((58, 34), '安装向导', font=font2, fill=(185, 195, 225))
hd.save(os.path.join(brand, 'installer-header.bmp'), format='BMP')
print('header ok')
