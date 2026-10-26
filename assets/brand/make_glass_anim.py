# -*- coding: utf-8 -*-
"""流动极光帧 + 铜钱旋转帧，供安装向导动画使用。"""
from PIL import Image, ImageDraw, ImageFilter, ImageFont, ImageEnhance
import os

BRAND = r"C:\Users\93783\XiaomiMiMoProjects\zrsj-bookkeeper\assets\brand"
OUT = os.path.join(BRAND, "wizard")
os.makedirs(OUT, exist_ok=True)

TEAL = (0x22, 0x98, 0xA8)
MINT = (0x6C, 0xD2, 0xAA)
PURPLE = (0x68, 0x4A, 0xCC)
PINK = (0xD6, 0x90, 0xC0)

W, H = 720, 480
N_FRAMES = 5
COIN_FRAMES = 8


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


def saturate(color, factor=0.7):
    gray = 0.299 * color[0] + 0.587 * color[1] + 0.114 * color[2]
    return tuple(int(c + (gray - c) * (1 - factor)) for c in color)


def aurora_frame(phase):
    """相位偏移的弥散极光，形成流动感。"""
    import math
    img = Image.new("RGB", (W, H), (0xF2, 0xF4, 0xFA))
    glow = Image.new("RGB", (W, H), (0, 0, 0))
    g = ImageDraw.Draw(glow)

    c1 = saturate(TEAL, 0.7)
    c2 = saturate(MINT, 0.7)
    c3 = saturate(PURPLE, 0.65)
    c4 = saturate(PINK, 0.7)

    # 光斑随相位缓慢漂移
    def orbit(cx, cy, r, ax, ay, spd):
        dx = int(ax * math.cos(phase * spd))
        dy = int(ay * math.sin(phase * spd * 0.8))
        return [cx + dx - r, cy + dy - r, cx + dx + r, cy + dy + r]

    g.ellipse(orbit(160, 140, 220, 50, 36, 1.0), fill=c3)
    g.ellipse(orbit(560, 120, 240, 42, 30, 1.1), fill=c1)
    g.ellipse(orbit(160, 360, 230, 38, 44, 0.9), fill=c2)
    g.ellipse(orbit(560, 360, 240, 46, 32, 1.05), fill=c4)
    g.ellipse(orbit(360, 240, 200, 60, 40, 0.7), fill=c3)
    g.ellipse(orbit(400, 200, 180, 40, 50, 1.2), fill=c1)

    glow = glow.filter(ImageFilter.GaussianBlur(130))
    img = Image.blend(img, glow, 0.42)
    return img


def frost_overlay(bg, x, y, w, h, radius=32):
    shadow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    sd = ImageDraw.Draw(shadow)
    sd.rounded_rectangle([x + 2, y + 18, x + w + 6, y + h + 28], radius=radius, fill=(60, 70, 120, 36))
    shadow = shadow.filter(ImageFilter.GaussianBlur(28))
    bg = Image.alpha_composite(bg.convert("RGBA"), shadow)

    crop = bg.crop((x, y, x + w, y + h)).convert("RGB")
    crop = crop.filter(ImageFilter.GaussianBlur(36))
    crop = ImageEnhance.Color(crop).enhance(0.7)
    crop = ImageEnhance.Brightness(crop).enhance(1.12)
    veil = Image.new("RGBA", (w, h), (255, 255, 255, 148))
    frost = Image.alpha_composite(crop.convert("RGBA"), veil)

    mask = Image.new("L", (w, h), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, w - 1, h - 1], radius=radius, fill=255)
    panel = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    panel.paste(frost, (0, 0), mask)
    stroke = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    ImageDraw.Draw(stroke).rounded_rectangle(
        [1, 1, w - 2, h - 2], radius=radius - 1, outline=(255, 255, 255, 175), width=2
    )
    panel = Image.alpha_composite(panel, stroke)
    out = bg.copy()
    out.paste(panel, (x, y), panel)
    return out


def glass_button(d, x, y, w, h, label, primary=False):
    if primary:
        fill = (0x68, 0x4A, 0xCC, 235)
        outline = (255, 255, 255, 90)
        tcol = (255, 255, 255)
    else:
        fill = (255, 255, 255, 120)
        outline = (255, 255, 255, 160)
        tcol = (0x2A, 0x33, 0x48)
    d.rounded_rectangle([x, y, x + w, y + h], radius=14, fill=fill, outline=outline, width=1)
    f = font("msyhbd.ttc" if primary else "msyh.ttc", 15)
    tw = d.textlength(label, font=f)
    d.text((x + (w - tw) / 2, y + (h - 20) / 2), label, font=f, fill=tcol)


def make_coin_frames():
    """铜钱 3D 滚动帧（水平翻转模拟滚动）。"""
    coin = Image.open(os.path.join(BRAND, "app-icon.png")).convert("RGBA")
    side = 72
    coin = coin.resize((side, side), Image.LANCZOS)
    for i in range(COIN_FRAMES):
        t = i / COIN_FRAMES
        # 用水平缩放模拟翻滚
        sx = abs(0.15 + 0.85 * abs(2 * ((t * 2) % 1.0) - 1))
        nw = max(4, int(side * sx))
        frame = coin.resize((nw, side), Image.LANCZOS)
        canvas = Image.new("RGBA", (side, side), (0, 0, 0, 0))
        canvas.paste(frame, ((side - nw) // 2, 0), frame)
        canvas.save(os.path.join(OUT, f"coin-{i:02d}.png"))
    print("coin frames", COIN_FRAMES)


def make_welcome_frames():
    """欢迎页：流动极光 + 面板 + 标题/说明/路径/安装按钮（无进度条）。"""
    px, py, pw, ph = 84, 56, W - 168, H - 112
    for i in range(N_FRAMES):
        phase = 2 * 3.14159 * i / N_FRAMES
        bg = aurora_frame(phase)
        img = frost_overlay(bg, px, py, pw, ph, radius=32)
        d = ImageDraw.Draw(img, "RGBA")
        cx = px + pw / 2

        f1 = font("msyhbd.ttc", 30)
        t1 = "自然拾光记账"
        d.text((cx - d.textlength(t1, font=f1) / 2, py + 48), t1, font=f1, fill=(0x1A, 0x22, 0x38))
        f2 = font("msyh.ttc", 15)
        t2 = "本地账套 · 账不上云"
        d.text((cx - d.textlength(t2, font=f2) / 2, py + 98), t2, font=f2, fill=(0x5B, 0x65, 0x77))

        f3 = font("msyh.ttc", 13)
        d.text((cx - d.textlength("安装位置", font=f3) / 2, py + 150), "安装位置", font=f3, fill=(0x8A, 0x93, 0xA6))
        # 路径卡片
        card_w = pw - 100
        card_x = cx - card_w / 2
        d.rounded_rectangle([card_x, py + 172, card_x + card_w, py + 210], radius=10, fill=(255, 255, 255, 150), outline=(255, 255, 255, 180), width=1)

        f4 = font("msyh.ttc", 13)
        hint = "点击「安装」开始安装到本地"
        d.text((cx - d.textlength(hint, font=f4) / 2, py + 236), hint, font=f4, fill=(0x5B, 0x65, 0x77))

        # 底部：安装 / 取消（无上一步、无进度条）
        bw, bh, gap = 108, 40, 16
        total = bw * 2 + gap
        bx = cx - total / 2
        by = py + ph - 88
        glass_button(d, bx, by, bw, bh, "安装", primary=True)
        glass_button(d, bx + bw + gap, by, bw, bh, "取消")

        img.convert("RGB").save(os.path.join(OUT, f"anim-welcome-{i:02d}.bmp"), format="BMP")
    print("welcome frames", N_FRAMES)


def make_install_frames():
    """安装页：流动极光 + 面板 + 进度槽（铜钱在 NSIS 侧叠放）。"""
    px, py, pw, ph = 84, 56, W - 168, H - 112
    for i in range(N_FRAMES):
        phase = 2 * 3.14159 * i / N_FRAMES
        bg = aurora_frame(phase)
        img = frost_overlay(bg, px, py, pw, ph, radius=32)
        d = ImageDraw.Draw(img, "RGBA")
        cx = px + pw / 2

        f1 = font("msyhbd.ttc", 26)
        t1 = "正在安装"
        d.text((cx - d.textlength(t1, font=f1) / 2, py + 56), t1, font=f1, fill=(0x1A, 0x22, 0x38))
        f2 = font("msyh.ttc", 14)
        t2 = "请稍候，安装完成后即可开始使用"
        d.text((cx - d.textlength(t2, font=f2) / 2, py + 104), t2, font=f2, fill=(0x5B, 0x65, 0x77))

        # 进度条轨道（不含进度，由 NSIS 控件/铜钱叠放）
        bar_w = pw - 140
        bar_x, bar_y, bar_h = px + 70, py + 180, 12
        d.rounded_rectangle([bar_x, bar_y, bar_x + bar_w, bar_y + bar_h], radius=bar_h // 2, fill=(220, 226, 238, 180))

        bw, bh = 108, 40
        glass_button(d, cx - bw / 2, py + ph - 88, bw, bh, "取消")
        img.convert("RGB").save(os.path.join(OUT, f"anim-install-{i:02d}.bmp"), format="BMP")
    print("install frames", N_FRAMES)


def make_finish_frames():
    """完成页：流动极光 + 成功态。"""
    px, py, pw, ph = 84, 56, W - 168, H - 112
    for i in range(N_FRAMES):
        phase = 2 * 3.14159 * i / N_FRAMES
        bg = aurora_frame(phase)
        img = frost_overlay(bg, px, py, pw, ph, radius=32)
        d = ImageDraw.Draw(img, "RGBA")
        cx = px + pw / 2
        f1 = font("msyhbd.ttc", 28)
        t1 = "安装完成"
        d.text((cx - d.textlength(t1, font=f1) / 2, py + 64), t1, font=f1, fill=(0x1A, 0x22, 0x38))
        f2 = font("msyh.ttc", 14)
        t2 = "自然拾光记账 已就绪，点击「完成」退出向导"
        d.text((cx - d.textlength(t2, font=f2) / 2, py + 114), t2, font=f2, fill=(0x5B, 0x65, 0x77))
        # 满进度条
        bar_w = pw - 140
        bar_x, bar_y, bar_h = px + 70, py + 180, 12
        d.rounded_rectangle([bar_x, bar_y, bar_x + bar_w, bar_y + bar_h], radius=bar_h // 2, fill=(220, 226, 238, 180))
        d.rounded_rectangle([bar_x, bar_y, bar_x + bar_w, bar_y + bar_h], radius=bar_h // 2, fill=(0x68, 0x4A, 0xCC, 230))

        bw, bh, gap = 108, 40, 16
        total = bw * 2 + gap
        bx = cx - total / 2
        glass_button(d, bx, py + ph - 88, bw, bh, "完成", primary=True)
        glass_button(d, bx + bw + gap, py + ph - 88, bw, bh, "关闭")
        img.convert("RGB").save(os.path.join(OUT, f"anim-finish-{i:02d}.bmp"), format="BMP")
    print("finish frames", N_FRAMES)


if __name__ == "__main__":
    make_coin_frames()
    make_welcome_frames()
    make_install_frames()
    make_finish_frames()
    print("anim assets ->", OUT)
