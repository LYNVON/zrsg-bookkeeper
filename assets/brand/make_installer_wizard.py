# -*- coding: utf-8 -*-
"""生成互联网大厂风安装向导视觉素材（极简科技感，条款不进页）。"""
from PIL import Image, ImageDraw, ImageFilter, ImageFont
import os

BRAND = os.path.dirname(os.path.abspath(__file__))
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


def aurora_bg(w, h, base="#F7F8FC"):
    img = Image.new("RGB", (w, h), base)
    glow = Image.new("RGB", (w, h), (0, 0, 0))
    g = ImageDraw.Draw(glow)
    # 弥散极光光斑 —— 与应用内核一致
    g.ellipse([int(w * -0.15), int(h * -0.25), int(w * 0.55), int(h * 0.35)], fill=(140, 120, 255))
    g.ellipse([int(w * 0.55), int(h * -0.2), int(w * 1.25), int(h * 0.4)], fill=(110, 170, 255))
    g.ellipse([int(w * -0.1), int(h * 0.55), int(w * 0.5), int(h * 1.2)], fill=(120, 210, 200))
    g.ellipse([int(w * 0.55), int(h * 0.5), int(w * 1.2), int(h * 1.15)], fill=(255, 160, 210))
    glow = glow.filter(ImageFilter.GaussianBlur(max(w, h) // 7))
    return Image.blend(img, glow, 0.14)


def paste_coin(img, cx, cy, size):
    coin = Image.open(os.path.join(BRAND, "app-icon.png")).convert("RGBA")
    coin = coin.resize((size, size), Image.LANCZOS)
    # 柔和阴影
    sh = Image.new("RGBA", (size + 24, size + 24), (0, 0, 0, 0))
    sd = ImageDraw.Draw(sh)
    sd.ellipse([12, 18, size + 12, size + 18], fill=(20, 30, 80, 55))
    sh = sh.filter(ImageFilter.GaussianBlur(10))
    img.paste(sh, (cx - size // 2 - 12, cy - size // 2 - 6), sh)
    img.paste(coin, (cx - size // 2, cy - size // 2), coin)


def text_center(d, y, text, fnt, fill):
    w = d.textlength(text, font=fnt)
    d.text(((d.im.size[0] if hasattr(d, "im") else 0), y), "", font=fnt, fill=fill)  # noop keep API
    return w


def draw_center(d, w, y, text, fnt, fill):
    tw = d.textlength(text, font=fnt)
    d.text(((w - tw) / 2, y), text, font=fnt, fill=fill)


def pill_button(w, h, label, c1, c2, text_color="#FFFFFF"):
    """渐变胶囊按钮位图。"""
    btn = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    # 外发光
    glow = Image.new("RGBA", (w + 32, h + 32), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow)
    gd.rounded_rectangle([16, 20, w + 16, h + 20], radius=h // 2, fill=(*c1, 90))
    glow = glow.filter(ImageFilter.GaussianBlur(12))
    canvas = Image.new("RGBA", (w + 32, h + 32), (0, 0, 0, 0))
    canvas.paste(glow, (0, 0), glow)
    # 渐变填充
    grad = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    for x in range(w):
        t = x / max(w - 1, 1)
        r = int(c1[0] + (c2[0] - c1[0]) * t)
        g = int(c1[1] + (c2[1] - c1[1]) * t)
        b = int(c1[2] + (c2[2] - c1[2]) * t)
        ImageDraw.Draw(grad).line([(x, 0), (x, h)], fill=(r, g, b, 255))
    mask = Image.new("L", (w, h), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, w - 1, h - 1], radius=h // 2, fill=255)
    # 顶部高光
    highlight = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    ImageDraw.Draw(highlight).rounded_rectangle([2, 2, w - 3, h // 2], radius=h // 3, fill=(255, 255, 255, 36))
    grad = Image.alpha_composite(grad, highlight)
    canvas.paste(grad, (16, 12), mask)
    fnt = font("msyhbd.ttc", int(h * 0.38))
    td = ImageDraw.Draw(canvas)
    tw = td.textlength(label, font=fnt)
    td.text((16 + (w - tw) / 2, 12 + (h - fnt.size) / 2 - 2), label, font=fnt, fill=text_color)
    return canvas


def make_welcome():
    W, H = 502, 330
    img = aurora_bg(W, H)
    d = ImageDraw.Draw(img)

    # 顶部极光描边
    for i, c in enumerate([(122, 92, 240), (72, 150, 255), (255, 150, 200)]):
        d.line([(0, i), (W, i)], fill=c)

    paste_coin(img, W // 2, 78, 72)
    d = ImageDraw.Draw(img)

    draw_center(d, W, 128, "自然拾光记账", font("msyhbd.ttc", 26), "#1A2233")
    draw_center(d, W, 162, "本地数据 · 账不上云", font("msyh.ttc", 13), "#5B6577")

    # 手写体 slogan（无行楷则雅黑）
    try:
        slogan = "账清财旺"
        f3 = font("STXINGKA.TTF", 18)
        draw_center(d, W, 188, slogan, f3, "#A07A3C")
    except Exception:
        pass

    # 主按钮
    btn = pill_button(220, 48, "立即安装", (59, 109, 242), (123, 92, 240))
    img.paste(btn, ((W - 220) // 2, 218), btn)
    d = ImageDraw.Draw(img)

    # 次要：自定义安装
    draw_center(d, W, 272, "自定义安装路径 ›", font("msyh.ttc", 11), "#3B6DF2")

    # 底部：条款只留链接，不显示正文
    d.line([(24, H - 36), (W - 24, H - 36)], fill=(210, 216, 230), width=1)
    footer_font = font("msyh.ttc", 10)
    left = "安装即表示同意"
    mid1 = "《用户协议》"
    mid2 = "《隐私政策》"
    x = 24
    d.text((x, H - 26), left, font=footer_font, fill="#8A93A6")
    x += d.textlength(left, font=footer_font)
    d.text((x, H - 26), mid1, font=footer_font, fill="#3B6DF2")
    x += d.textlength(mid1, font=footer_font)
    d.text((x, H - 26), "和", font=footer_font, fill="#8A93A6")
    x += d.textlength("和", font=footer_font)
    d.text((x, H - 26), mid2, font=footer_font, fill="#3B6DF2")

    ver = "v0.1.0"
    vw = d.textlength(ver, font=footer_font)
    d.text((W - 24 - vw, H - 26), ver, font=footer_font, fill="#A0A8B8")

    img.save(os.path.join(OUT, "welcome-bg.bmp"), format="BMP")
    print("welcome-bg", img.size)


def make_progress():
    W, H = 502, 330
    img = aurora_bg(W, H)
    d = ImageDraw.Draw(img)
    for i, c in enumerate([(122, 92, 240), (72, 150, 255), (255, 150, 200)]):
        d.line([(0, i), (W, i)], fill=c)
    paste_coin(img, W // 2, 100, 64)
    d = ImageDraw.Draw(img)
    draw_center(d, W, 150, "正在安装 自然拾光记账", font("msyhbd.ttc", 20), "#1A2233")
    draw_center(d, W, 182, "本地安装 · 数据不出本机", font("msyh.ttc", 12), "#5B6577")
    # 进度条槽
    bw, bh = 320, 8
    bx, by = (W - bw) // 2, 230
    d.rounded_rectangle([bx, by, bx + bw, by + bh], radius=bh // 2, fill="#E4E8F2")
    d.rounded_rectangle([bx, by, bx + int(bw * 0.35), by + bh], radius=bh // 2, fill="#3B6DF2")
    draw_center(d, W, 250, "请稍候…", font("msyh.ttc", 11), "#8A93A6")
    img.save(os.path.join(OUT, "progress-bg.bmp"), format="BMP")
    print("progress-bg", img.size)


def make_finish():
    W, H = 502, 330
    img = aurora_bg(W, H)
    d = ImageDraw.Draw(img)
    for i, c in enumerate([(122, 92, 240), (72, 150, 255), (255, 150, 200)]):
        d.line([(0, i), (W, i)], fill=c)
    paste_coin(img, W // 2, 92, 68)
    d = ImageDraw.Draw(img)
    draw_center(d, W, 140, "安装完成", font("msyhbd.ttc", 26), "#1A2233")
    draw_center(d, W, 178, "自然拾光 账清财旺", font("msyh.ttc", 13), "#5B6577")
    btn = pill_button(220, 48, "开始使用", (59, 109, 242), (123, 92, 240))
    img.paste(btn, ((W - 220) // 2, 212), btn)
    d = ImageDraw.Draw(img)
    d.line([(24, H - 36), (W - 24, H - 36)], fill=(210, 216, 230), width=1)
    footer_font = font("msyh.ttc", 10)
    left = "安装即表示同意"
    mid1 = "《用户协议》"
    mid2 = "《隐私政策》"
    x = 24
    d.text((x, H - 26), left, font=footer_font, fill="#8A93A6")
    x += d.textlength(left, font=footer_font)
    d.text((x, H - 26), mid1, font=footer_font, fill="#3B6DF2")
    x += d.textlength(mid1, font=footer_font)
    d.text((x, H - 26), "和", font=footer_font, fill="#8A93A6")
    x += d.textlength("和", font=footer_font)
    d.text((x, H - 26), mid2, font=footer_font, fill="#3B6DF2")
    ver = "v0.1.0"
    vw = d.textlength(ver, font=footer_font)
    d.text((W - 24 - vw, H - 26), ver, font=footer_font, fill="#A0A8B8")
    img.save(os.path.join(OUT, "finish-bg.bmp"), format="BMP")
    print("finish-bg", img.size)


def make_buttons():
    # 主按钮（贴在欢迎/完成页上，供点击热区覆盖）
    for name, label in [("btn-install", "立即安装"), ("btn-launch", "开始使用")]:
        btn = pill_button(220, 48, label, (59, 109, 242), (123, 92, 240))
        # 去掉外边距，只要按钮本体+发光
        btn.save(os.path.join(OUT, f"{name}.png"))
        btn.convert("RGB").save(os.path.join(OUT, f"{name}.bmp"), format="BMP")
        print(name, btn.size)

    # 取消 / 更改路径 小按钮底（浅色）
    small = pill_button(108, 32, "更改…", (232, 236, 245), (232, 236, 245), text_color="#3B6DF2")
    small.save(os.path.join(OUT, "btn-secondary.bmp"), format="BMP")
    print("btn-secondary", small.size)


def make_checkbox_glyphs():
    for name, fill, border in [
        ("check-off", None, "#8A93A6"),
        ("check-on", "#3B6DF2", "#3B6DF2"),
    ]:
        s = 20
        im = Image.new("RGBA", (s, s), (0, 0, 0, 0))
        d = ImageDraw.Draw(im)
        d.rounded_rectangle([1, 1, s - 2, s - 2], radius=5, fill=fill, outline=border, width=2)
        if fill:
            d.line([(5, 10), (9, 14), (15, 6)], fill="#FFFFFF", width=2)
        im.save(os.path.join(OUT, f"{name}.png"))
        im.convert("RGB").save(os.path.join(OUT, f"{name}.bmp"), format="BMP")
        print(name, im.size)


if __name__ == "__main__":
    make_welcome()
    make_progress()
    make_finish()
    make_buttons()
    make_checkbox_glyphs()
    print("wizard assets ->", OUT)
