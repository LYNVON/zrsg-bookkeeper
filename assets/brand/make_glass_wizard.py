# -*- coding: utf-8 -*-
"""轻奢毛玻璃安装向导视觉 — 品牌色弥散极光 + 磨砂面板 + 白描边。"""
from PIL import Image, ImageDraw, ImageFilter, ImageFont, ImageEnhance
import os

BRAND = r"C:\Users\93783\XiaomiMiMoProjects\zrsj-bookkeeper\assets\brand"
OUT = os.path.join(BRAND, "wizard")
os.makedirs(OUT, exist_ok=True)

# 品牌标准色
TEAL = (0x22, 0x98, 0xA8)    # #2298A8
MINT = (0x6C, 0xD2, 0xAA)    # #6CD2AA
PURPLE = (0x68, 0x4A, 0xCC)  # #684ACC
PINK = (0xD6, 0x90, 0xC0)    # #D690C0

W, H = 720, 480  # 1.5x 设计稿，细腻质感


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


def saturate(color, factor=0.72):
    """低饱和：向亮度中点靠拢。"""
    gray = 0.299 * color[0] + 0.587 * color[1] + 0.114 * color[2]
    return tuple(int(c + (gray - c) * (1 - factor)) for c in color)


def aurora_bg():
    """柔和弥散极光浅色流动渐变背景 — 低饱和、四色均衡。"""
    img = Image.new("RGB", (W, H), (0xF2, 0xF4, 0xFA))
    glow = Image.new("RGB", (W, H), (0, 0, 0))
    g = ImageDraw.Draw(glow)

    # 低饱和：向灰度靠拢更多
    c1 = saturate(TEAL, 0.7)
    c2 = saturate(MINT, 0.7)
    c3 = saturate(PURPLE, 0.65)
    c4 = saturate(PINK, 0.7)

    # 四角均衡光斑
    g.ellipse([-200, -160, 360, 300], fill=c3)   # 左上紫
    g.ellipse([360, -160, 920, 300], fill=c1)    # 右上青
    g.ellipse([-200, 280, 360, 740], fill=c2)    # 左下薄荷
    g.ellipse([360, 280, 920, 740], fill=c4)     # 右下粉

    glow = glow.filter(ImageFilter.GaussianBlur(140))
    img = Image.blend(img, glow, 0.42)
    # 极浅流动层
    glow2 = Image.new("RGB", (W, H), (0, 0, 0))
    g2 = ImageDraw.Draw(glow2)
    g2.ellipse([40, 40, 480, 360], fill=c1)
    g2.ellipse([280, 120, 720, 440], fill=c3)
    glow2 = glow2.filter(ImageFilter.GaussianBlur(120))
    img = Image.blend(img, glow2, 0.14)
    return img


def frosted_panel(bg, x, y, w, h, radius=32):
    """半透明磨砂面板 + 细微白色描边 + 柔和光影。"""
    # 柔和投影（更淡、更远）
    shadow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    sd = ImageDraw.Draw(shadow)
    sd.rounded_rectangle([x + 2, y + 18, x + w + 6, y + h + 28], radius=radius, fill=(60, 70, 120, 36))
    shadow = shadow.filter(ImageFilter.GaussianBlur(28))
    bg = Image.alpha_composite(bg.convert("RGBA"), shadow)

    # 磨砂：裁一块背景做高斯模糊，再叠更透的白
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

    # 细微白色描边
    stroke = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    ImageDraw.Draw(stroke).rounded_rectangle(
        [1, 1, w - 2, h - 2], radius=radius - 1, outline=(255, 255, 255, 175), width=2
    )
    panel = Image.alpha_composite(panel, stroke)

    out = bg.copy()
    out.paste(panel, (x, y), panel)
    return out


def draw_progress(d, x, y, w, h, ratio, label):
    """柔和进度条。"""
    # 轨道
    d.rounded_rectangle([x, y, x + w, y + h], radius=h // 2, fill=(220, 226, 238, 200))
    # 进度
    fw = int(w * ratio)
    if fw > h:
        d.rounded_rectangle([x, y, x + fw, y + h], radius=h // 2, fill=(*PURPLE, 230))
        # 顶部微高光
        d.rounded_rectangle([x + 2, y + 2, x + fw - 2, y + h // 2], radius=h // 3, fill=(255, 255, 255, 40))
    # 文案
    f = font("msyh.ttc", 13)
    d.text((x, y + h + 12), label, font=f, fill=(0x4A, 0x55, 0x68))


def glass_button(d, x, y, w, h, label, primary=False):
    """底部玻璃按钮。"""
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


def make_welcome():
    bg = aurora_bg()
    # 大圆角面板
    px, py, pw, ph = 84, 56, W - 168, H - 112
    img = frosted_panel(bg, px, py, pw, ph, radius=32)
    d = ImageDraw.Draw(img, "RGBA")

    cx = px + pw / 2

    # 顶部标题
    f1 = font("msyhbd.ttc", 30)
    t1 = "自然拾光记账"
    d.text((cx - d.textlength(t1, font=f1) / 2, py + 48), t1, font=f1, fill=(0x1A, 0x22, 0x38))

    f2 = font("msyh.ttc", 15)
    t2 = "本地账套 · 账不上云"
    d.text((cx - d.textlength(t2, font=f2) / 2, py + 98), t2, font=f2, fill=(0x5B, 0x65, 0x77))

    # 中间文字说明
    f3 = font("msyh.ttc", 14)
    lines = [
        "安装过程将在本地完成，账套数据默认不上传。",
        "安装即表示同意《用户协议》与《隐私政策》。",
    ]
    for i, line in enumerate(lines):
        d.text(
            (cx - d.textlength(line, font=f3) / 2, py + 150 + i * 28),
            line,
            font=f3,
            fill=(0x4A, 0x55, 0x68),
        )

    # 进度条（欢迎页展示为空轨道，示意流程）
    bar_w = pw - 120
    draw_progress(d, px + 60, py + 230, bar_w, 10, 0.0, "准备安装")

    # 底部按钮：首页无「上一步」，仅 下一步 / 取消
    bw, bh, gap = 108, 40, 16
    total = bw * 2 + gap
    bx = cx - total / 2
    by = py + ph - 88
    glass_button(d, bx, by, bw, bh, "下一步", primary=True)
    glass_button(d, bx + bw + gap, by, bw, bh, "取消")

    img.save(os.path.join(OUT, "glass-welcome.bmp"), format="BMP")
    img.save(os.path.join(OUT, "glass-welcome.png"))
    print("glass-welcome", img.size)
    return img


def make_progress():
    bg = aurora_bg()
    px, py, pw, ph = 84, 56, W - 168, H - 112
    img = frosted_panel(bg, px, py, pw, ph, radius=32)
    d = ImageDraw.Draw(img, "RGBA")
    cx = px + pw / 2

    f1 = font("msyhbd.ttc", 26)
    t1 = "正在安装"
    d.text((cx - d.textlength(t1, font=f1) / 2, py + 56), t1, font=f1, fill=(0x1A, 0x22, 0x38))

    f2 = font("msyh.ttc", 14)
    t2 = "请稍候，安装完成后即可开始使用"
    d.text((cx - d.textlength(t2, font=f2) / 2, py + 104), t2, font=f2, fill=(0x5B, 0x65, 0x77))

    bar_w = pw - 120
    draw_progress(d, px + 60, py + 180, bar_w, 12, 0.62, "正在复制文件…")

    # 底部：仅取消
    bw, bh = 108, 40
    glass_button(d, cx - bw / 2, py + ph - 88, bw, bh, "取消")

    img.save(os.path.join(OUT, "glass-progress.bmp"), format="BMP")
    img.save(os.path.join(OUT, "glass-progress.png"))
    print("glass-progress", img.size)
    return img


def make_finish():
    bg = aurora_bg()
    px, py, pw, ph = 84, 56, W - 168, H - 112
    img = frosted_panel(bg, px, py, pw, ph, radius=32)
    d = ImageDraw.Draw(img, "RGBA")
    cx = px + pw / 2

    f1 = font("msyhbd.ttc", 28)
    t1 = "安装完成"
    d.text((cx - d.textlength(t1, font=f1) / 2, py + 64), t1, font=f1, fill=(0x1A, 0x22, 0x38))

    f2 = font("msyh.ttc", 14)
    t2 = "自然拾光记账 已就绪，点击「完成」退出向导"
    d.text((cx - d.textlength(t2, font=f2) / 2, py + 114), t2, font=f2, fill=(0x5B, 0x65, 0x77))

    bar_w = pw - 120
    draw_progress(d, px + 60, py + 180, bar_w, 12, 1.0, "安装完成")

    bw, bh, gap = 108, 40, 16
    total = bw * 2 + gap
    bx = cx - total / 2
    by = py + ph - 88
    glass_button(d, bx, by, bw, bh, "完成", primary=True)
    glass_button(d, bx + bw + gap, by, bw, bh, "关闭")

    img.save(os.path.join(OUT, "glass-finish.bmp"), format="BMP")
    img.save(os.path.join(OUT, "glass-finish.png"))
    print("glass-finish", img.size)
    return img


if __name__ == "__main__":
    make_welcome()
    make_progress()
    make_finish()
    print("glass assets ->", OUT)
