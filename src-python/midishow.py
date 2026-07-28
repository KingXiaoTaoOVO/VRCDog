"""
midishow 下载器 — 集成版
基于 midishow-downloader-selfhost 核心 + Node.js CLI 搜索
"""
import json
import os
import subprocess
import re
import requests
from bs4 import BeautifulSoup

from midishow_api import (
    download_midi_url,
    get_account_manager,
    AccountManager,
    MidiShowAPI,
)

# ================================================================
# 搜索（通过 Node.js CLI）
# ================================================================
CLI_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "midishow-downloader")
CLI_SCRIPT = os.path.join(CLI_DIR, "dist", "cli.js")


def _run_cli_search(keyword: str) -> str:
    """运行 Node.js CLI 搜索"""
    if not os.path.exists(CLI_SCRIPT):
        # 尝试编译
        tsconfig = os.path.join(CLI_DIR, "tsconfig.json")
        if os.path.exists(tsconfig):
            try:
                subprocess.run(
                    ["npx", "tsc"], cwd=CLI_DIR,
                    capture_output=True, timeout=30, shell=True,
                )
            except Exception:
                pass

    if not os.path.exists(CLI_SCRIPT):
        raise Exception("CLI 搜索脚本未找到，请确保已安装 Node.js")

    result = subprocess.run(
        ["node", CLI_SCRIPT, "search", keyword],
        cwd=CLI_DIR, capture_output=True, timeout=30, shell=True,
    )

    if result.returncode != 0:
        try:
            err_data = json.loads(result.stderr.decode("utf-8", errors="replace"))
            raise Exception(err_data.get("error", "搜索失败"))
        except (json.JSONDecodeError, UnicodeDecodeError):
            raise Exception(result.stderr.decode("utf-8", errors="replace").strip())

    return result.stdout.decode("utf-8", errors="replace")


def _clean_title(raw_title: str) -> str:
    title = raw_title.strip()
    title = " ".join(title.split())
    title = re.split(r"\s*[•|]\s*|上传于|下载|评分|\d+\.\d+\s*\(|\d+(?:\.\d+)?\s*(?:KB|MB)|\bGM\d*\b", title, maxsplit=1)[0].strip()
    parts = title.split()
    if len(parts) > 1 and re.match(r"^[A-Za-z0-9_.@-]{3,}$", parts[1]):
        title = parts[0]
    return title


def _clean_artist(raw_artist: str) -> str:
    artist = " ".join(str(raw_artist or "").split()).strip()
    artist = re.split(r"\s*[•|]\s*|上传于|下载|评分|KB|MB|\d{2}:\d{2}", artist, maxsplit=1)[0].strip()
    artist = re.sub(r"^(Artist|Author|艺术家|作者)[:：\s]*", "", artist, flags=re.I).strip()
    return artist[:48]


def _fallback_search(keyword: str, max_results: int) -> list:
    """纯 Python 搜索后备，复用 midishow-downloader 的解析策略。"""
    session = requests.Session()
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
    }
    resp = session.get(
        "https://www.midishow.com/search/result",
        params={"q": keyword, "page": 1, "per-page": max(30, max_results)},
        headers=headers,
        timeout=20,
    )
    resp.raise_for_status()
    html = resp.text
    soup = BeautifulSoup(html, "html.parser")
    results = []
    seen = set()

    for card in soup.select("[data-key], .midi-item, .col-midi, div[class*='midi']"):
        data_key = card.get("data-key")
        if not data_key or not data_key.isdigit():
            continue
        midi_id = int(data_key)
        if midi_id in seen:
            continue
        seen.add(midi_id)
        title_el = card.select_one('a[href*="/en/midi/"], a[href*="/midi/"]')
        artist_el = card.select_one(".artist, .author, [class*='artist'], [class*='author']")
        results.append({
            "id": midi_id,
            "title": _clean_title(title_el.get_text(" ", strip=True) if title_el else f"MIDI #{midi_id}"),
            "artist": artist_el.get_text(" ", strip=True) if artist_el else "",
            "page_url": f"https://www.midishow.com/en/midi/{midi_id}.html",
        })

    if not results:
        for midi_id in re.findall(r"/en/midi/(\d+)\.html", html):
            midi_id = int(midi_id)
            if midi_id in seen:
                continue
            seen.add(midi_id)
            results.append({
                "id": midi_id,
                "title": f"MIDI #{midi_id}",
                "artist": "",
                "page_url": f"https://www.midishow.com/en/midi/{midi_id}.html",
            })

    return results[:max_results]


# ================================================================
# 公共 API
# ================================================================

def search_midi(keyword: str, max_results: int = 30) -> list:
    """搜索 midishow 上的 MIDI 文件"""
    try:
        output = _run_cli_search(keyword)
        results = json.loads(output)
    except Exception:
        results = _fallback_search(keyword, max_results)
    for r in results:
        r["title"] = _clean_title(r.get("title", f"MIDI #{r['id']}"))
        r["artist"] = _clean_artist(r.get("artist", ""))
    return results[:max_results]


def download_midi(midi_id: int, username: str = None) -> bytes:
    """
    下载指定 ID 的 MIDI 文件。
    通过 midishow_api 下载（支持登录）。
    """
    url = f"https://www.midishow.com/en/midi/{midi_id}.html"
    data, title = download_midi_url(url, username)
    return data


def get_midi_info(midi_id: int) -> dict:
    """获取 MIDI 文件信息"""
    import requests
    from bs4 import BeautifulSoup

    url = f"https://www.midishow.com/en/midi/{midi_id}.html"
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        "Accept-Language": "zh-CN,zh;q=0.9",
    }
    resp = requests.get(url, headers=headers, timeout=15)
    soup = BeautifulSoup(resp.text, "html.parser")

    title_el = soup.find("h1") or soup.find("title")
    title = title_el.get_text(strip=True) if title_el else f"MIDI #{midi_id}"
    title = re.sub(r'\s*-\s*MidiShow.*$', '', title).strip()

    info = {
        "id": midi_id,
        "title": title,
        "artist": "",
        "uploader": "",
        "page_url": f"https://www.midishow.com/en/midi/{midi_id}.html",
    }

    for pattern in [".artist", ".author", "[class*='artist']", "[class*='author']"]:
        el = soup.select_one(pattern)
        if el:
            text = el.get_text(strip=True)
            text = re.sub(r'^Artist[:：\s]*', '', text, re.I)
            if text:
                info["artist"] = text
                break

    return info


def login_midi(identity: str, password: str) -> bool:
    """登录 midishow 账号"""
    mgr = get_account_manager()
    mgr.add_account(identity, password)
    try:
        mgr.get_api(identity)
        return True
    except Exception as e:
        raise Exception(f"登录失败: {e}")


def download_and_save(midi_id: int, save_dir: str, filename: str = None) -> str:
    """下载 MIDI 文件并保存到本地"""
    info = get_midi_info(midi_id)
    if not filename:
        safe_title = "".join(c for c in info["title"] if c not in r'<>:"/\\|?*')
        filename = f"{safe_title}.mid"
    save_path = os.path.join(save_dir, filename)
    data = download_midi(midi_id)
    with open(save_path, "wb") as f:
        f.write(data)
    return save_path
