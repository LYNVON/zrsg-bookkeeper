# -*- coding: utf-8 -*-
"""大厂安装向导视觉 — 参考联想电脑管家 / PPTV：全幅品牌区 + 底部控制条。"""
from PIL import Image, ImageDraw, ImageFilter, ImageFont, ImageEnhance
import os
import math

BRAND = r"C:\Users\93783\XiaomiMiMoProjects\zrsj-bookkeeper\assets\brand"
OUT = os.path.join(BRAND, "wizard")
os.makedirs(OUT, exist_ok=True)

TEAL = (0x22, 0x98, 0xA8)
MINT = (0x6C, 0xD2, 0xAA)
PURPLE = (0x68, 0x4A, 0xCC)
PINK = (0xD6, 0x90, 0xC0)

W, H = 720, 540
HERO_H = 360  # 上部品牌区
N_FRAMES = 5


def font(name, size):
    for path in (
        rf"C:\Windows\Fonts\{name}",
        r"C:\Windows\Fonts\msyhbd.ttc",
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\Deng.ttf",
    ):
        try:
            return ImageFont.truetype(path, size)
        except Exception:
            continue
    return ImageFont.load_default()


def saturate(c, f=0.72):
    g = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]
    return tuple(int(x + (g - x) * (1 - f)) for x in c)


def hero_bg(phase=0.0):
    """上部品牌区：品牌色弥散极光。"""
    img = Image.new("RGB", (W, HERO_H), (0xE8, 0xEE, 0xF8))
    glow = Image.new("RGB", (W, HERO_H), (0, 0, 0))
    g = ImageDraw.Draw(glow)

    c1 = saturate(TEAL, 0.8)
    c2 = saturate(MINT, 0.8)
    c3 = saturate(PURPLE, 0.75)
    c4 = saturate(PINK, 0.8)

    def orb(cx, cy, r, ax, ay, spd):
        dx = int(ax * math.cos(phase * spd))
        dy = int(ay * math.sin(phase * spd * 0.85))
        return [cx + dx - r, cy + dy - r, cx + dx + r, cy + dy + r]

    g.ellipse(orb(120, 80, 220, 40, 28, 1.0), fill=c3)
    g.ellipse(orb(600, 60, 240, 36, 24, 1.1), fill=c1)
    g.ellipse(orb(80, 300, 200, 32, 36, 0.9), fill=c2)
    g.ellipse(orb(640, 280, 220, 40, 30, 1.05), fill=c4)
    g.ellipse(orb(360, 160, 200, 50, 32, 0.7), fill=c3)

    glow = glow.filter(ImageFilter.GaussianBlur(100))
    img = Image.blend(img, glow, 0.48)
    return img


def load_coin(size):
    coin = Image.open(os.path.join(BRAND, "app-icon.png")).convert("RGBA")
    return coin.resize((size, size), Image.LANCZOS)


def draw_hero(d, title="自然拾光记账", slogan="账清财旺", show_btn=True, btn_label="立即安装"):
    """品牌区：logo + 标题 + slogan + 大按钮。"""
    cx = W // 2

    # 金币
    coin = load_coin(88)
    sh = Image.new("RGBA", (112, 112), (0, 0, 0, 0))
    ImageDraw.Draw(sh).ellipse([12, 18, 100, 106], fill=(20, 30, 80, 70))
    sh = sh.filter(ImageFilter.GaussianBlur(10))
    img_tmp = d._image
    img_tmp.paste(sh, (cx - 56, 36), sh)
    img_tmp.paste(coin, (cx - 44, 42), coin)

    # 重建 draw（paste 后需要）
    d = ImageDraw.Draw(img_tmp, "RGBA")

    f1 = font("msyhbd.ttc", 32)
    tw = d.textlength(title, font=f1)
    d.text((cx - tw / 2, 142), title, font=f1, fill=(0x1A, 0x22, 0x38))

    f2 = font("msyh.ttc", 16)
    tw = d.textlength(slogan, font=f2)
    d.text((cx - tw / 2, 192), slogan, font=f2, fill=(0x4A, 0x55, 0x68))

    if show_btn:
        # 大圆角主按钮（胶囊）
        bw, bh = 200, 52
        bx, by = cx - bw // 2, 250
        # 按钮发光
        gl = Image.new("RGBA", (bw + 40, bh + 40), (0, 0, 0, 0))
        ImageDraw.Draw(gl).rounded_rectangle(
            [20, 22, 20 + bw, 22 + bh], radius=bh // 2, fill=(80, 100, 230, 110)
        )
        gl = gl.filter(ImageFilter.GaussianBlur(14))
        img_tmp.paste(gl, (bx - 20, by - 12), gl)
        d = ImageDraw.Draw(img_tmp, "RGBA")
        # 渐变按钮
        grad = Image.new("RGBA", (bw, bh))
        for x in range(bw):
            t = x / (bw - 1)
            r = int(59 + (110 - 59) * t)
            g = int(109 + (95 - 109) * t)
            b = int(242 + (240 - 242) * t)
            ImageDraw.Draw(grad).line([(x, 0), (x, bh)], fill=(r, g, b, 255))
        mask = Image.new("L", (bw, bh), 0)
        ImageDraw.Draw(mask).rounded_rectangle([0, 0, bw - 1, bh - 1], radius=bh // 2, fill=255)
        img_tmp.paste(grad, (bx, by), mask)
        d = ImageDraw.Draw(img_tmp, "RGBA")
        d.rounded_rectangle([bx + 2, by + 2, bx + bw - 2, by + bh // 2 - 2], radius=bh // 3, outline=(255, 255, 255, 50), width=1)
        bf = font("msyhbd.ttc", 19)
        tw = d.textlength(btn_label, font=bf)
        d.text((bx + (bw - tw) / 2, by + 15), btn_label, font=bf, fill="#FFFFFF")

    return d


def draw_bottom_bar(img, d, path_text, show_check=True, show_modify=True):
    """底部控制条：协议勾选 + 安装路径 + 修改 + 空间信息。"""
    bar_y = HERO_H
    bar_h = H - HERO_H
    # 浅色条
    d.rectangle([0, bar_y, W, H], fill=(0xF7, 0xF8, 0xFC))
    d.line([(0, bar_y), (W, bar_y)], fill=(0xE2, 0xE7, 0xF0), width=1)

    # 协议勾选行
    f = font("msyh.ttc", 13)
    if show_check:
        # 勾选框
        d.rounded_rectangle([28, bar_y + 18, 46, bar_y + 36], radius=4, fill=(0x68, 0x4A, 0xCC), outline=(0x68, 0x4A, 0xCC))
        d.line([(32, bar_y + 27), (37, bar_y + 32), (43, bar_y + 22)], fill="#FFFFFF", width=2)
        d.text((56, bar_y + 20), "我已阅读并同意", font=f, fill=(0x4A, 0x55, 0x68))
        x = 56 + d.textlength("我已阅读并同意", font=f)
        d.text((x, bar_y + 20), "《用户协议》", font=f, fill=(0x3B, 0x6D, 0xF2))
        x += d.textlength("《用户协议》", font=f)
        d.text((x, bar_y + 20), "和", font=f, fill=(0x4A, 0x55, 0x68))
        x += d.textlength("和", font=f)
        d.text((x, bar_y + 20), "《隐私政策》", font=f, fill=(0x3B, 0x6D, 0xF2))

    # 自定义安装（右侧小字）
    f2 = font("msyh.ttc", 12)
    t = "自定义安装  ▲"
    tw = d.textlength(t, font=f2)
    d.text((W - 28 - tw, bar_y + 22), t, font=f2, fill=(0x5B, 0x65, 0x77))

    # 安装位置行
    y2 = bar_y + 62
    d.text((28, y2 + 6), "安装位置:", font=f, fill=(0x5B, 0x65, 0x77))

    # 路径输入框
    box_x, box_w = 110, 420
    d.rounded_rectangle([box_x, y2, box_x + box_w, y2 + 36], radius=6, fill=(0xFF, 0xFF, 0xFF), outline=(0xD5, 0xDC, 0xE8), width=1)
    fp = font("msyh.ttc", 12)
    # 路径过长省略
    p = path_text
    max_w = box_w - 24
    while d.textlength(p, font=fp) > max_w and len(p) > 8:
        p = p[:-2]
    if p != path_text:
        p = p[:-1] + "…"
    d.text((box_x + 12, y2 + 10), p, font=fp, fill=(0x2A, 0x33, 0x48))

    if show_modify:
        # 修改按钮
        mx, mw = box_x + box_w + 12, 72
        d.rounded_rectangle([mx, y2, mx + mw, y2 + 36], radius=6, fill=(0xFF, 0xFF, 0xFF), outline=(0xC5, 0xCE, 0xDE), width=1)
        fm = font("msyh.ttc", 13)
        tw = d.textlength("修改", font=fm)
        d.text((mx + (mw - tw) / 2, y2 + 10), "修改", font=fm, fill=(0x2A, 0x33, 0x48))

    # 空间信息
    fs = font("msyh.ttc", 11)
    d.text((28, y2 + 48), "所需空间: 约 30M    可用空间: 充足", font=fs, fill=(0x8A, 0x93, 0xA6))

    return d


def make_welcome(phase=0.0):
    img = Image.new("RGB", (W, H), (0xF7, 0xF8, 0xFC))
    hero = hero_bg(phase)
    img.paste(hero, (0, 0))
    d = ImageDraw.Draw(img, "RGBA")
    d = draw_hero(d, show_btn=True, btn_label="立即安装")
    path = r"C:\Users\93783\AppData\Local\自然拾光记账"
    d = draw_bottom_bar(img, d, path)
    return img


def make_install(phase=0.0):
    img = Image.new("RGB", (W, H), (0xF7, 0xF8, 0xFC))
    hero = hero_bg(phase)
    img.paste(hero, (0, 0))
    d = ImageDraw.Draw(img, "RGBA")
    d = draw_hero(d, show_btn=False)
    # 进度条在品牌区下方
    bx, by, bw, bh = 120, 270, 480, 12
    d.rounded_rectangle([bx, by, bx + bw, by + bh], radius=bh // 2, fill=(255, 255, 255, 120))
    d.rounded_rectangle([bx, by, bx + int(bw * 0.35), by + bh], radius=bh // 2, fill=(0x68, 0x4A, 0xCC, 220))
    ft = font("msyh.ttc", 13)
    t = "正在安装，请稍候…"
    tw = d.textlength(t, font=ft)
    d.text(((W - tw) / 2, 296), t, font=ft, fill=(0x3D, 0x4A, 0x5C))
    # 底部简化
    bar_y = HERO_H
    d.rectangle([0, bar_y, W, H], fill=(0xF7, 0xF8, 0xFC))
    d.line([(0, bar_y), (W, bar_y)], fill=(0xE2, 0xE7, 0xF0), width=1)
    f = font("msyh.ttc", 13)
    d.text((28, bar_y + 40), "正在写入文件，请勿关闭窗口…", font=f, fill=(0x5B, 0x65, 0x77))
    return img


def make_finish(phase=0.0):
    img = Image.new("RGB", (W, H), (0xF7, 0xF8, 0xFC))
    hero = hero_bg(phase)
    img.paste(hero, (0, 0))
    d = ImageDraw.Draw(img, "RGBA")
    d = draw_hero(d, title="安装完成", slogan="自然拾光 账清财旺", show_btn=True, btn_label="完成")
    bar_y = HERO_H
    d.rectangle([0, bar_y, W, H], fill=(0xF7, 0xF8, 0xFC))
    d.line([(0, bar_y), (W, bar_y)], fill=(0xE2, 0xE7, 0xF0), width=1)
    f = font("msyh.ttc", 13)
    t = "点击「完成」退出向导并启动应用"
    tw = d.textlength(t, font=f)
    d.text(((W - tw) / 2, bar_y + 40), t, font=f, fill=(0x5B, 0x65, 0x77))
    return img


if __name__ == "__main__":
    for i in range(N_FRAMES):
        ph = 2 * math.pi * i / N_FRAMES
        make_welcome(ph).convert("RGB").save(os.path.join(OUT, f"hero-welcome-{i:02d}.bmp"), format="BMP")
        make_install(ph).convert("RGB").save(os.path.join(OUT, f"hero-install-{i:02d}.bmp"), format="BMP")
        make_finish(ph).convert("RGB").save(os.path.join(OUT, f"hero-finish-{i:02d}.bmp"), format="BMP")
    # 静帧预览
    make_welcome(0).save(os.path.join(OUT, "hero-welcome.png"))
    make_install(0).save(os.path.join(OUT, "hero-install.png"))
    make_finish(0).save(os.path.join(OUT, "hero-finish.png"))
    print("hero frames ok", N_FRAMES, "->", OUT)
