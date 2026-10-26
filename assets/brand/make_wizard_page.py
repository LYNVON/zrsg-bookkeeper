# -*- coding: utf-8 -*-
"""大厂风安装向导整页视觉 — 微信/钉钉范式：留白、层级、单一主按钮。"""
from PIL import Image, ImageDraw, ImageFilter, ImageFont
import os

BRAND = r"C:\Users\93783\XiaomiMiMoProjects\zrsj-bookkeeper\assets\brand"
OUT = os.path.join(BRAND, "wizard")
os.makedirs(OUT, exist_ok=True)


def font(name, size):
    for path in (
        rf"C:\Windows\Fonts\{name}",
        r"C:\Windows\Fonts\msyhbd.ttc",
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\Deng.ttf",
        r"C:\Windows\Fonts\segoeuib.ttf",
    ):
        try:
            return ImageFont.truetype(path, size)
        except Exception:
            continue
    return ImageFont.load_default()


# 窗口客户区约 560x380（含品牌左栏）
W, H = 560, 380
PANEL = 168  # 左侧品牌栏宽

# —— 色板（与应用一致）——
INK = "#141B2D"
SUB = "#5B6577"
MUTED = "#8A93A6"
LINE = "#E6EAF2"
BTN1 = "#3B6DF2"
BTN2 = "#6B5CE7"
CARD = "#F5F7FB"


def load_coin(size):
    coin = Image.open(os.path.join(BRAND, "app-icon.png")).convert("RGBA")
    return coin.resize((size, size), Image.LANCZOS)


def make_panel():
    """左侧品牌栏：深靛→紫罗兰 + 弥散光。"""
    panel = Image.new("RGB", (PANEL, H), "#1B2240")
    d = ImageDraw.Draw(panel)
    for y in range(H):
        t = y / H
        r = int(24 + t * (92 - 24))
        g = int(32 + t * (62 - 32))
        b = int(72 + t * (188 - 72))
        d.line([(0, y), (PANEL, y)], fill=(r, g, b))
    glow = Image.new("RGB", (PANEL, H), (0, 0, 0))
    gd = ImageDraw.Draw(glow)
    gd.ellipse([-60, 20, 130, 170], fill=(70, 220, 190))
    gd.ellipse([40, 160, 220, 320], fill=(230, 120, 200))
    gd.ellipse([-20, 260, 160, 420], fill=(120, 120, 255))
    glow = glow.filter(ImageFilter.GaussianBlur(42))
    panel = Image.blend(panel, glow, 0.42)
    d = ImageDraw.Draw(panel)

    coin = load_coin(96)
    sh = Image.new("RGBA", (120, 120), (0, 0, 0, 0))
    ImageDraw.Draw(sh).ellipse([12, 18, 108, 114], fill=(10, 15, 50, 90))
    sh = sh.filter(ImageFilter.GaussianBlur(10))
    panel.paste(sh, ((PANEL - 96) // 2 - 12, 88), sh)
    panel.paste(coin, ((PANEL - 96) // 2, 92), coin)
    d = ImageDraw.Draw(panel)

    def ctext(y, text, fnt, fill):
        tw = d.textlength(text, font=fnt)
        d.text(((PANEL - tw) / 2, y), text, font=fnt, fill=fill)

    ctext(210, "自然拾光记账", font("msyhbd.ttc", 18), "#F7F8FC")
    ctext(242, "本地数据 · 账不上云", font("msyh.ttc", 11), "#C8D2F0")
    ctext(278, "账清财旺", font("msyhbd.ttc", 15), "#F0DCB4")
    d.line([(32, 330), (PANEL - 32, 330)], fill=(255, 255, 255, 70), width=1)
    en = "FORTUNE & CLARITY"
    tw = d.textlength(en, font=font("msyh.ttc", 9))
    d.text(((PANEL - tw) / 2, 342), en, font=font("msyh.ttc", 9), fill="#AEB8D8")
    return panel


def make_welcome(path_text):
    img = Image.new("RGB", (W, H), "#FFFFFF")
    # 顶部极光细带
    top = Image.new("RGB", (W, 6), "#E8ECF8")
    td = ImageDraw.Draw(top)
    for x in range(W):
        t = x / (W - 1)
        r = int(120 + (255 - 120) * t)
        g = int(100 + (150 - 100) * t)
        b = int(240 + (190 - 240) * t)
        td.line([(x, 0), (x, 6)], fill=(r, g, b))
    img.paste(top, (0, 0))

    panel = make_panel()
    img.paste(panel, (0, 0))
    d = ImageDraw.Draw(img)

    x0 = PANEL + 36  # 右侧内容起始
    content_w = W - x0 - 36

    # 标题
    d.text((x0, 48), "自然拾光记账", font=font("msyhbd.ttc", 26), fill=INK)
    d.text((x0, 88), "本地账套 · 账不上云", font=font("msyh.ttc", 13), fill=SUB)

    # 分隔
    d.line([(x0, 122), (x0 + content_w, 122)], fill=LINE, width=1)

    # 安装位置
    d.text((x0, 142), "安装位置", font=font("msyh.ttc", 12), fill=MUTED)

    # 路径卡片（路径文字由 NSIS 标签覆盖，便于「更改」后更新）
    card_y, card_h = 168, 40
    d.rounded_rectangle([x0, card_y, x0 + content_w - 72, card_y + card_h], radius=8, fill=CARD, outline=LINE, width=1)

    # 更改按钮（浅底）
    chg_x = x0 + content_w - 64
    d.rounded_rectangle([chg_x, card_y + 4, chg_x + 56, card_y + card_h - 4], radius=8, fill="#FFFFFF", outline="#C9D2E8", width=1)
    cf = font("msyh.ttc", 12)
    tw = d.textlength("更改…", font=cf)
    d.text((chg_x + (56 - tw) / 2, card_y + 12), "更改…", font=cf, fill="#3B6DF2")

    # 勾选区：只留占位，真实 Checkbox 叠在上面
    cb_y = 232
    d.text((x0 + 28, cb_y + 1), "创建桌面快捷方式", font=font("msyh.ttc", 13), fill="#3D4A5C")

    # 主按钮
    btn_y, btn_h = 278, 52
    grad = Image.new("RGB", (content_w, btn_h))
    for x in range(content_w):
        t = x / (content_w - 1)
        r = int(59 + (107 - 59) * t)
        g = int(109 + (92 - 109) * t)
        b = int(242 + (231 - 242) * t)
        ImageDraw.Draw(grad).line([(x, 0), (x, btn_h)], fill=(r, g, b))
    mask = Image.new("L", (content_w, btn_h), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, content_w - 1, btn_h - 1], radius=12, fill=255)
    # 投影
    sh = Image.new("RGBA", (content_w + 24, btn_h + 24), (0, 0, 0, 0))
    ImageDraw.Draw(sh).rounded_rectangle([12, 16, 12 + content_w, 16 + btn_h], radius=12, fill=(59, 109, 242, 90))
    sh = sh.filter(ImageFilter.GaussianBlur(10))
    img.paste(sh, (x0 - 12, btn_y - 8), sh)
    img.paste(grad, (x0, btn_y), mask)
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([x0 + 1, btn_y + 1, x0 + content_w - 2, btn_y + btn_h // 2], radius=10, outline=(255, 255, 255), width=1)
    bf = font("msyhbd.ttc", 18)
    tw = d.textlength("立即安装", font=bf)
    d.text((x0 + (content_w - tw) / 2, btn_y + 15), "立即安装", font=bf, fill="#FFFFFF")

    # 底部
    d.text((x0, 348), "安装即表示同意《用户协议》《隐私政策》", font=font("msyh.ttc", 11), fill=MUTED)
    ver = "v0.1.0"
    d.text((W - 36 - d.textlength(ver, font=font("msyh.ttc", 11)), 348), ver, font=font("msyh.ttc", 11), fill="#A8B0C0")

    return img


def make_finish():
    img = Image.new("RGB", (W, H), "#FFFFFF")
    top = Image.new("RGB", (W, 6), "#E8ECF8")
    td = ImageDraw.Draw(top)
    for x in range(W):
        t = x / (W - 1)
        td.line([(x, 0), (x, 6)], fill=(int(120 + 135 * t), int(100 + 50 * t), int(240 - 50 * t)))
    img.paste(top, (0, 0))
    img.paste(make_panel(), (0, 0))
    d = ImageDraw.Draw(img)
    x0 = PANEL + 36
    content_w = W - x0 - 36

    coin = load_coin(72)
    img.paste(coin, (x0 + (content_w - 72) // 2, 72), coin)
    d = ImageDraw.Draw(img)
    f = font("msyhbd.ttc", 28)
    t = "安装完成"
    d.text((x0 + (content_w - d.textlength(t, font=f)) / 2, 160), t, font=f, fill=INK)
    f2 = font("msyh.ttc", 13)
    t2 = "自然拾光 账清财旺"
    d.text((x0 + (content_w - d.textlength(t2, font=f2)) / 2, 204), t2, font=f2, fill=SUB)

    btn_y, btn_h = 260, 52
    grad = Image.new("RGB", (content_w, btn_h))
    for x in range(content_w):
        tt = x / (content_w - 1)
        ImageDraw.Draw(grad).line(
            [(x, 0), (x, btn_h)], fill=(int(59 + 48 * tt), int(109 - 17 * tt), int(242 - 11 * tt))
        )
    mask = Image.new("L", (content_w, btn_h), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, content_w - 1, btn_h - 1], radius=12, fill=255)
    img.paste(grad, (x0, btn_y), mask)
    d = ImageDraw.Draw(img)
    bf = font("msyhbd.ttc", 18)
    tw = d.textlength("开始使用", font=bf)
    d.text((x0 + (content_w - tw) / 2, btn_y + 15), "开始使用", font=bf, fill="#FFFFFF")

    d.text((x0, 348), "安装即表示同意《用户协议》《隐私政策》", font=font("msyh.ttc", 11), fill=MUTED)
    ver = "v0.1.0"
    d.text((W - 36 - d.textlength(ver, font=font("msyh.ttc", 11)), 348), ver, font=font("msyh.ttc", 11), fill="#A8B0C0")
    return img


if __name__ == "__main__":
    default_path = r"C:\Users\93783\AppData\Local\自然拾光记账"
    w = make_welcome(default_path)
    w.save(os.path.join(OUT, "page-welcome.bmp"), format="BMP")
    w.save(os.path.join(OUT, "page-welcome.png"))
    f = make_finish()
    f.save(os.path.join(OUT, "page-finish.bmp"), format="BMP")
    f.save(os.path.join(OUT, "page-finish.png"))
    print("welcome", w.size, "finish", f.size)
    # 热区坐标（供 NSIS 叠控件）：
    # 更改按钮: x0=204, y=172, w=56, h=32  (PANEL+36=204)
    # 立即安装: x0=204, y=278, w=content_w=320, h=52
    # 勾选: x0=204, y=232
    print("hotspots: browse=204,172,56,32  install=204,278,320,52  check=204,232,28,28")
