# -*- coding: utf-8 -*-
"""编译「自然拾光记账」大厂风 NSIS 安装包。"""
import os
import shutil
import subprocess
import sys

ROOT = r"F:\MiMoWork\projects\zrsj-bookkeeper"
APP = os.path.join(ROOT, "app", "src-tauri")
NSIS_SRC = os.path.join(APP, "nsis")
STAGE = os.path.join(APP, "target", "nsis-custom")
OUT_DIR = os.path.join(APP, "target", "release", "bundle", "nsis")
MAKENSIS = r"C:\Users\93783\AppData\Local\tauri\NSIS\makensis.exe"
PLUGIN_DIR = r"C:\Users\93783\AppData\Local\tauri\NSIS\Plugins\x86-unicode\additional"


def main():
    version = "0.1.0"
    if len(sys.argv) > 1:
        version = sys.argv[1]

    if os.path.isdir(STAGE):
        shutil.rmtree(STAGE)
    os.makedirs(STAGE, exist_ok=True)

    # 素材与脚本
    shutil.copy2(os.path.join(NSIS_SRC, "zrsj-installer.nsi"), os.path.join(STAGE, "installer.nsi"))
    assets_dst = os.path.join(STAGE, "assets")
    docs_dst = os.path.join(STAGE, "docs")
    os.makedirs(assets_dst, exist_ok=True)
    os.makedirs(docs_dst, exist_ok=True)
    for fn in os.listdir(os.path.join(NSIS_SRC, "assets")):
        src = os.path.join(NSIS_SRC, "assets", fn)
        dst = os.path.join(assets_dst, fn)
        if os.path.isdir(src):
            shutil.copytree(src, dst, dirs_exist_ok=True)
        else:
            shutil.copy2(src, dst)
    for fn in os.listdir(os.path.join(NSIS_SRC, "docs")):
        shutil.copy2(os.path.join(NSIS_SRC, "docs", fn), os.path.join(docs_dst, fn))

    app_exe = os.path.join(APP, "target", "release", "app.exe")
    if not os.path.isfile(app_exe):
        raise SystemExit(f"缺少 app.exe，请先 npm run tauri build：{app_exe}")
    shutil.copy2(app_exe, os.path.join(STAGE, "app.exe"))

    icon = os.path.join(APP, "icons", "icon.ico")
    if os.path.isfile(icon):
        shutil.copy2(icon, os.path.join(STAGE, "app.ico"))
    else:
        raise SystemExit(f"缺少图标：{icon}")

    # makensis 以 UTF-8 脚本 + 插件路径编译
    cmd = [
        MAKENSIS,
        f"/DAPP_EXE=app.exe",
        f"/DASSET_DIR=assets",
        f"/DDOC_DIR=docs",
        f"/DINSTALLERICON=app.ico",
        f"/DVERSION={version}",
        f"/X!addplugindir \"{PLUGIN_DIR}\"",
        "installer.nsi",
    ]
    print("RUN:", " ".join(cmd))
    r = subprocess.run(cmd, cwd=STAGE, capture_output=True)
    sys.stdout.buffer.write(r.stdout)
    sys.stderr.buffer.write(r.stderr)
    if r.returncode != 0:
        raise SystemExit(f"makensis 失败 code={r.returncode}")

    produced = os.path.join(STAGE, f"自然拾光记账-Setup-{version}.exe")
    if not os.path.isfile(produced):
        # 退化匹配
        cands = [f for f in os.listdir(STAGE) if f.endswith(".exe") and "Setup" in f]
        if not cands:
            raise SystemExit("未找到生成的 Setup.exe")
        produced = os.path.join(STAGE, cands[0])

    os.makedirs(OUT_DIR, exist_ok=True)
    final = os.path.join(OUT_DIR, f"自然拾光记账_{version}_x64-setup-branded.exe")
    shutil.copy2(produced, final)

    # 可选签名：配置证书环境变量后自动签名，未配置则跳过
    sign_pfx = os.environ.get("ZRSJ_SIGN_PFX")
    sign_thumb = os.environ.get("ZRSJ_SIGN_THUMBPRINT")
    if sign_pfx or sign_thumb:
        sign_ps = os.path.join(ROOT, "tools", "sign.ps1")
        cmd2 = [
            "powershell", "-NoProfile", "-ExecutionPolicy", "Bypass",
            "-File", sign_ps, "-File", final,
        ]
        print("SIGN:", " ".join(cmd2))
        r2 = subprocess.run(cmd2, capture_output=True)
        sys.stdout.buffer.write(r2.stdout)
        sys.stderr.buffer.write(r2.stderr)
        if r2.returncode != 0:
            raise SystemExit("签名失败，请检查证书配置")
    else:
        print("SKIP SIGN: 未设置 ZRSJ_SIGN_PFX / ZRSJ_SIGN_THUMBPRINT")

    print("OK ->", final)


if __name__ == "__main__":
    main()
