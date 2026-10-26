# -*- coding: utf-8 -*-
"""安装向导：浅色毛玻璃 + 品牌四色柔光 + 无白边。"""
from PIL import Image, ImageDraw, ImageFilter, ImageFont
import math
import os

BRAND = r"F:\MiMoWork\projects\zrsj-bookkeeper\assets\brand"
OUT = os.path.join(BRAND, "wizard")
os.makedirs(OUT, exist_ok=True)

# 品牌标准色
TEAL = (0x22, 0x98, 0xA8)    # #2298A8
MINT = (0x6C, 0xD2, 0xAA)    # #6CD2AA
PURPLE = (0x68, 0x4A, 0xCC)  # #684ACC
PINK = (0xD6, 0x90, 0xC0)    # #D690C0

W, H = 720, 540
TITLE_H = 36
HERO_H = 340
N_FRAMES = 6


def font(name, size):
    for p in (rf"C:\Windows\Fonts\{name}", r"C:\Windows\Fonts\msyhbd.ttc",
              r"C:\Windows\Fonts\msyh.ttc", r"C:\Windows\Fonts\Deng.ttf"):
        try:
            return ImageFont.truetype(p, size)
        except Exception:
            continue
    return ImageFont.load_default()


def aa_rounded_rect(img, xy, radius, fill=None, outline=None, width=1):
    """4x 超采样圆角矩形，避免锯齿。"""
    x0, y0, x1, y1 = xy
    w, h = int(x1 - x0) + 1, int(y1 - y0) + 1
    if w <= 0 or h <= 0:
        return
    s = 4
    pad = max(radius, 4) * s + width * s + 8
    tmp = Image.new("RGBA", (w * s + pad * 2, h * s + pad * 2), (0, 0, 0, 0))
    td = ImageDraw.Draw(tmp)
    td.rounded_rectangle(
        [pad, pad, pad + w * s - 1, pad + h * s - 1],
        radius=radius * s,
        fill=fill,
        outline=outline,
        width=max(1, int(width * s)),
    )
    small = tmp.resize((tmp.size[0] // s, tmp.size[1] // s), Image.LANCZOS)
    base = img.convert("RGBA")
    base.paste(small, (x0 - pad // s, y0 - pad // s), small)
    img.paste(base.convert("RGB"))


def aa_line(img, p1, p2, fill, width=2):
    """4x 超采样直线。"""
    s = 4
    x0, y0 = min(p1[0], p2[0]), min(p1[1], p2[1])
    x1, y1 = max(p1[0], p2[0]), max(p1[1], p2[1])
    pad = width * s + 8
    w, h = int(x1 - x0) + 1 + pad * 2 // s, int(y1 - y0) + 1 + pad * 2 // s
    tmp = Image.new("RGBA", (max(w * s, 8), max(h * s, 8)), (0, 0, 0, 0))
    td = ImageDraw.Draw(tmp)
    a = (int((p1[0] - x0) * s + pad), int((p1[1] - y0) * s + pad))
    b = (int((p2[0] - x0) * s + pad), int((p2[1] - y0) * s + pad))
    td.line([a, b], fill=fill, width=int(width * s))
    small = tmp.resize((tmp.size[0] // s, tmp.size[1] // s), Image.LANCZOS)
    base = img.convert("RGBA")
    base.paste(small, (x0 - pad // s, y0 - pad // s), small)
    img.paste(base.convert("RGB"))


def flow_layer(g, phase, w, h):
    """品牌四色柔光，浅底上低饱和弥散。"""
    def orb(cx, cy, rx, ry, sx, sy, spd, sk=1.0):
        dx = sx * math.sin(phase * spd + sk)
        dy = sy * math.cos(phase * spd * 0.83 + sk)
        return [cx + dx - rx, cy + dy - ry, cx + dx + rx, cy + dy + ry]

    # 浅色底上用减淡后的品牌色，避免脏色
    c1 = (0x7EC8D8, 0x9BE0C8, 0xA89AE8, 0xE8B8D4)
    g.ellipse(orb(w * 0.18, h * 0.25, w * 0.32, h * 0.38, 28, 22, 0.55, 0.0), fill=c1[2])
    g.ellipse(orb(w * 0.82, h * 0.22, w * 0.34, h * 0.36, 24, 26, 0.48, 1.2), fill=c1[0])
    g.ellipse(orb(w * 0.15, h * 0.78, w * 0.30, h * 0.34, 22, 30, 0.62, 2.1), fill=c1[1])
    g.ellipse(orb(w * 0.85, h * 0.80, w * 0.32, h * 0.36, 30, 20, 0.51, 3.0), fill=c1[3])
    g.ellipse(orb(w * 0.50, h * 0.48, w * 0.28, h * 0.30, 36, 28, 0.40, 0.7), fill=c1[2])
    g.ellipse(orb(w * 0.42, h * 0.30, w * 0.22, h * 0.26, 20, 34, 0.58, 1.7), fill=c1[0])


def make_hero_frame(phase, title="自然拾光记账", subtitle="自然拾光 · 账清财旺", cta="立即安装"):
    # 浅色毛玻璃底
    img = Image.new("RGB", (W, H), (0xEE, 0xF3, 0xFA))
    glow = Image.new("RGB", (W, H), (255, 255, 255))
    g = ImageDraw.Draw(glow)
    flow_layer(g, phase, W, HERO_H + 40)
    glow = glow.filter(ImageFilter.GaussianBlur(110))
    img = Image.blend(img, glow, 0.72)

    # 毛玻璃：轻模糊 + 提亮
    img = img.filter(ImageFilter.GaussianBlur(2))
    img = img.point(lambda v: min(255, int(v * 1.02 + 6)))

    # 顶部高光条，强化玻璃感
    d = ImageDraw.Draw(img, "RGBA")
    d.rectangle([0, 0, W, 2], fill=(255, 255, 255, 90))
    d.rectangle([0, 0, 2, HERO_H], fill=(255, 255, 255, 50))

    # 标题栏
    ft = font("msyh.ttc", 12)
    d.text((16, 10), "自然拾光记账  安装向导", font=ft, fill=(0x5B, 0x65, 0x77))
    # 图标 12px，热区 36×28：最小化 648–684，关闭 684–720
    gc = (0x5B, 0x65, 0x77)
    d.line([(W - 60, 18), (W - 48, 18)], fill=gc, width=2)
    d.line([(W - 24, 12), (W - 12, 24)], fill=gc, width=2)
    d.line([(W - 12, 12), (W - 24, 24)], fill=gc, width=2)

    # 品牌区
    coin = Image.open(os.path.join(BRAND, "app-icon.png")).convert("RGBA")
    side = 84
    coin = coin.resize((side, side), Image.LANCZOS)
    cx = W // 2
    img.paste(coin, (cx - side // 2, 58), coin)
    d = ImageDraw.Draw(img, "RGBA")

    f1 = font("msyhbd.ttc", 30)
    d.text((cx - d.textlength(title, font=f1) / 2, 152), title, font=f1, fill=(0x1A, 0x22, 0x33))

    f2 = font("msyh.ttc", 16)
    d.text((cx - d.textlength(subtitle, font=f2) / 2, 200), subtitle, font=f2, fill=(0x5B, 0x65, 0x77))

    # 主按钮：品牌紫渐变胶囊（cta 为空则不画）
    if cta:
        bw, bh = 210, 52
        bx, by = cx - bw // 2, 248
        grad = Image.new("RGBA", (bw, bh))
        for x in range(bw):
            t = x / (bw - 1)
            r = int(0x68 + (0x22 - 0x68) * t * 0.35)
            gcol = int(0x4A + (0x98 - 0x4A) * t * 0.25)
            b = int(0xCC + (0xA8 - 0xCC) * t * 0.15)
            ImageDraw.Draw(grad).line([(x, 0), (x, bh)], fill=(r, gcol, b, 255))
        s = 4
        mask = Image.new("L", (bw * s, bh * s), 0)
        ImageDraw.Draw(mask).rounded_rectangle([0, 0, bw * s - 1, bh * s - 1], radius=(bh // 2) * s, fill=255)
        mask = mask.resize((bw, bh), Image.LANCZOS)
        img.paste(grad, (bx, by), mask)
        aa_rounded_rect(img, (bx + 2, by + 2, bx + bw - 2, by + bh // 2 - 2), bh // 3,
                        fill=(255, 255, 255, 40))
        d = ImageDraw.Draw(img, "RGBA")
        bf = font("msyhbd.ttc", 19)
        tw = d.textlength(cta, font=bf)
        d.text((bx + (bw - tw) / 2, by + 15), cta, font=bf, fill="#FFFFFF")

    return img


def draw_bottom(img, path_text, checked=True):
    """底部浅色毛玻璃控制条。"""
    d = ImageDraw.Draw(img, "RGBA")
    bar_y = HERO_H
    d.rectangle([0, bar_y, W, H], fill=(0xF7, 0xF9, 0xFC, 235))
    d.line([(0, bar_y), (W, bar_y)], fill=(0x68, 0x4A, 0xCC, 40), width=1)
    d.rectangle([0, bar_y, W, bar_y + 1], fill=(255, 255, 255, 120))

    f = font("msyh.ttc", 13)
    # 勾选框底图兜底；上层还有可点的 ChkImg 小图
    aa_rounded_rect(img, (28, bar_y + 20, 48, bar_y + 40), 4, fill=PURPLE)
    aa_line(img, (33, bar_y + 30), (38, bar_y + 35), (255, 255, 255, 255), width=2)
    aa_line(img, (38, bar_y + 35), (44, bar_y + 25), (255, 255, 255, 255), width=2)
    d = ImageDraw.Draw(img, "RGBA")
    x = 56
    d.text((x, bar_y + 22), "我已阅读并同意", font=f, fill=(0x3C, 0x45, 0x58))
    x += d.textlength("我已阅读并同意", font=f)
    d.text((x, bar_y + 22), "《用户协议》", font=f, fill=TEAL)
    x += d.textlength("《用户协议》", font=f)
    d.text((x, bar_y + 22), "和", font=f, fill=(0x3C, 0x45, 0x58))
    x += d.textlength("和", font=f)
    d.text((x, bar_y + 22), "《隐私政策》", font=f, fill=TEAL)

    # 自定义安装
    f2 = font("msyh.ttc", 12)
    t = "自定义安装  ▲"
    d.text((W - 28 - d.textlength(t, font=f2), bar_y + 24), t, font=f2, fill=(0x8A, 0x93, 0xA6))

    # 路径
    y2 = bar_y + 68
    d.text((28, y2 + 8), "安装位置:", font=f, fill=(0x5B, 0x65, 0x77))
    box_x, box_w = 112, 400
    aa_rounded_rect(img, (box_x, y2, box_x + box_w, y2 + 36), 6,
                    fill=(255, 255, 255, 200), outline=(0xC8, 0xD2, 0xE4), width=1)
    # 路径文字由 NSI 可刷新控件显示，底图只画输入框
    d = ImageDraw.Draw(img, "RGBA")

    # 修改
    mx, mw = box_x + box_w + 12, 72
    aa_rounded_rect(img, (mx, y2, mx + mw, y2 + 36), 6,
                    fill=(0xEE, 0xF2, 0xFA), outline=(0xC8, 0xD2, 0xE4), width=1)
    d = ImageDraw.Draw(img, "RGBA")
    fm = font("msyh.ttc", 13)
    tw = d.textlength("修改", font=fm)
    d.text((mx + (mw - tw) / 2, y2 + 10), "修改", font=fm, fill=(0x3C, 0x45, 0x58))

    fs = font("msyh.ttc", 11)
    d.text((28, y2 + 52), "所需空间: 约 30M    可用空间: 充足", font=fs, fill=(0x8A, 0x93, 0xA6))


def make_welcome(phase=0.0, checked=True):
    img = make_hero_frame(phase)
    draw_bottom(img, r"C:\Users\93783\AppData\Local\自然拾光记账", checked=checked)
    return img


def make_install(phase=0.0):
    img = make_hero_frame(phase, cta=None)
    d = ImageDraw.Draw(img, "RGBA")
    bx, by, bw, bh = 130, 268, 460, 12
    d.rounded_rectangle([bx, by, bx + bw, by + bh], radius=bh // 2, fill=(0xC8, 0xD2, 0xE4, 160))
    d.rounded_rectangle([bx, by, bx + int(bw * 0.3), by + bh], radius=bh // 2, fill=PURPLE)
    ft = font("msyh.ttc", 13)
    t = "正在安装，请稍候…"
    d.text(((W - d.textlength(t, font=ft)) / 2, 292), t, font=ft, fill=(0x3C, 0x45, 0x58))
    bar_y = HERO_H
    d.rectangle([0, bar_y, W, H], fill=(0xF7, 0xF9, 0xFC, 235))
    d.line([(0, bar_y), (W, bar_y)], fill=(0x68, 0x4A, 0xCC, 40), width=1)
    f = font("msyh.ttc", 13)
    d.text((28, bar_y + 40), "正在写入文件，请勿关闭窗口…", font=f, fill=(0x5B, 0x65, 0x77))
    return img


def make_finish(phase=0.0):
    img = make_hero_frame(phase, title="安装完成", cta="完成")
    d = ImageDraw.Draw(img, "RGBA")
    bar_y = HERO_H
    d.rectangle([0, bar_y, W, H], fill=(0xF7, 0xF9, 0xFC, 235))
    d.line([(0, bar_y), (W, bar_y)], fill=(0x68, 0x4A, 0xCC, 40), width=1)
    f = font("msyh.ttc", 13)
    d.text((28, bar_y + 40), "点击「完成」退出向导", font=f, fill=(0x5B, 0x65, 0x77))
    return img


def make_checkbox_icons():
    """勾选/未勾选按钮脸 28×28，底色与底栏一致，4x 超采样。"""
    w, h = 28, 28
    bg = (0xF6, 0xF9, 0xF9)
    s = 4
    for name, on in (("chk-on.bmp", True), ("chk-off.bmp", False)):
        im = Image.new("RGB", (w * s, h * s), bg)
        d = ImageDraw.Draw(im)
        bx0 = (w * s - 18 * s) // 2
        by0 = (h * s - 18 * s) // 2
        bx1 = bx0 + 18 * s - 1
        by1 = by0 + 18 * s - 1
        if on:
            d.rounded_rectangle([bx0, by0, bx1, by1], radius=4 * s, fill=PURPLE)
            d.line([(bx0 + 4 * s, by0 + 9 * s), (bx0 + 7 * s, by0 + 12 * s)],
                   fill=(255, 255, 255), width=2 * s)
            d.line([(bx0 + 7 * s, by0 + 12 * s), (bx0 + 13 * s, by0 + 6 * s)],
                   fill=(255, 255, 255), width=2 * s)
        else:
            d.rounded_rectangle([bx0, by0, bx1, by1], radius=4 * s,
                                fill=(255, 255, 255), outline=(0xC8, 0xD2, 0xE4), width=s)
        im = im.resize((w, h), Image.LANCZOS)
        im.save(os.path.join(OUT, name), format="BMP")


if __name__ == "__main__":
    for i in range(N_FRAMES):
        ph = 2 * math.pi * i / N_FRAMES + 0.4 * i
        make_welcome(ph).save(os.path.join(OUT, f"pptv-welcome-{i:02d}.bmp"), format="BMP")
        make_install(ph).save(os.path.join(OUT, f"pptv-install-{i:02d}.bmp"), format="BMP")
        make_finish(ph).save(os.path.join(OUT, f"pptv-finish-{i:02d}.bmp"), format="BMP")
    make_welcome(0).save(os.path.join(OUT, "pptv-welcome.png"))
    make_welcome(0, checked=False).save(os.path.join(OUT, "pptv-welcome-uncheck.bmp"), format="BMP")
    make_welcome(0).save(os.path.join(OUT, "pptv-welcome-check.bmp"), format="BMP")
    make_install(0).save(os.path.join(OUT, "pptv-install.png"))
    make_finish(0).save(os.path.join(OUT, "pptv-finish.png"))
    make_checkbox_icons()
    print("pptv frames ok", N_FRAMES, "->", OUT)
