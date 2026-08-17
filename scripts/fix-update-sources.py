# -*- coding: utf-8 -*-
"""Fix VRCDog online update sources to point at vrcdog-releases.
Run: python3 fix-update.py
"""
import os
import sys
from pathlib import Path

ROOT = Path(r"C:\Users\Administrator\Documents\Project\VRCDog")

def patch(path: Path, replacements):
    """replacements: list of (old, new, required_label). Returns True if patched."""
    text = path.read_text(encoding="utf-8")
    original = text
    hits = []
    for old, new, label in replacements:
        if old in text:
            text = text.replace(old, new)
            hits.append(label)
    if text == original:
        print(f"[SKIP] {path}")
        return False
    # Use byte-level write to preserve line endings (CRLF on Windows sources)
    raw = original.encode("utf-8")
    new_raw = text.encode("utf-8")
    if b"\r\n" in raw and b"\r\n" not in new_raw:
        new_raw = new_raw.replace(b"\n", b"\r\n")
    path.write_bytes(new_raw)
    print(f"[OK]   {path}  ({', '.join(hits)})")
    return True


# ----------------------------------------------------------------------
# 1) tauri.conf.json: updater endpoint
# ----------------------------------------------------------------------
tauri_conf = ROOT / "src-tauri" / "tauri.conf.json"
patch(
    tauri_conf,
    [
        (
            '"https://github.com/KingXiaoTaoOVO/VRCDog/releases/latest/download/updater.json"',
            '"https://github.com/KingXiaoTaoOVO/vrcdog-releases/releases/latest/download/updater.json"',
            "updater.endpoint",
        ),
    ],
)

# ----------------------------------------------------------------------
# 2) LoginView.vue: fetch URL + download fallback URL
# ----------------------------------------------------------------------
login_view = ROOT / "src" / "components" / "LoginView.vue"
patch(
    login_view,
    [
        (
            "'https://api.github.com/repos/KingXiaoTaoOVO/VRCDog/releases?per_page=30'",
            "'https://api.github.com/repos/KingXiaoTaoOVO/vrcdog-releases/releases?per_page=30'",
            "fetchReleases URL",
        ),
        (
            "`https://github.com/KingXiaoTaoOVO/VRCDog/releases/tag/${rel.tag}`",
            "`https://github.com/KingXiaoTaoOVO/vrcdog-releases/releases/tag/${rel.tag}`",
            "downloadUpdate fallback URL",
        ),
    ],
)

# ----------------------------------------------------------------------
# 3) release.yml: cross-repo publish to vrcdog-releases
# ----------------------------------------------------------------------
release_yml = ROOT / ".github" / "workflows" / "release.yml"
patch(
    release_yml,
    [
        # Use the PAT secret for cross-repo publishing.
        (
            "GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}",
            "GITHUB_TOKEN: ${{ secrets.VRCDOG_RELEASES_TOKEN }}",
            "GITHUB_TOKEN source",
        ),
        # Comment hint inside the same block.
        (
            "          # 用于自动更新包的签名。必须在 GitHub 仓库设置中添加这两个 Secret!",
            "          # 用于自动更新包的签名。\n"
            "          # VRCDOG_RELEASES_TOKEN 是有 public_repo 权限的 PAT，用于把 release 推到跨仓库 vrcdog-releases。\n"
            "          # TAURI_PRIVATE_KEY / TAURI_KEY_PASSWORD 用于签名 updater.json。",
            "env comment",
        ),
        # Add cross-repo publish params (owner/repo/releaseCommitish).
        # Place after `prerelease: false` so they sit with the other `with:` options.
        (
            "          prerelease: false\n          # 自动合并更新文件以便生成 updater.json\n          updaterJsonKeepUniversal: false\n",
            "          prerelease: false\n"
            "          # 把 release / updater.json / 安装包推到专门的发版仓 vrcdog-releases（VRCDog 本身是私有的）\n"
            "          owner: KingXiaoTaoOVO\n"
            "          repo: vrcdog-releases\n"
            "          releaseCommitish: main\n"
            "          # 自动合并更新文件以便生成 updater.json\n"
            "          updaterJsonKeepUniversal: false\n",
            "owner/repo/releaseCommitish",
        ),
    ],
)

print("done.")
