"""
midishow API 客户端 — 基于 midishow-downloader-selfhost 核心逻辑
支持登录、Cookie 缓存、下载 MIDI 文件
"""
import os
import re
import json
import base64
import time
import requests
import threading
from urllib.parse import urlparse
from bs4 import BeautifulSoup

# ================================================================
# Cookie 缓存（文件版，无需 Redis）
# ================================================================
COOKIE_CACHE_FILE = os.environ.get("VRCDOG_MIDISHOW_COOKIE_CACHE") or os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    ".midishow_cookies.json"
)
_cache_lock = threading.Lock()


def _load_cookie_cache() -> dict:
    with _cache_lock:
        if os.path.exists(COOKIE_CACHE_FILE):
            try:
                with open(COOKIE_CACHE_FILE, "r", encoding="utf-8") as f:
                    return json.load(f)
            except Exception:
                return {}
        return {}


def _save_cookie_cache(cache: dict):
    # S2: 会话 Cookie 属敏感凭据，缓存文件以 0600 权限写入，避免其他本地用户/程序读取
    with _cache_lock:
        fd = os.open(COOKIE_CACHE_FILE, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
        try:
            with os.fdopen(fd, "w", encoding="utf-8") as f:
                json.dump(cache, f, ensure_ascii=False, indent=2)
        except Exception:
            os.close(fd)
            raise


def _clear_expired_cookies():
    """清理过期的 cookie 缓存"""
    cache = _load_cookie_cache()
    changed = False
    for key in list(cache.keys()):
        if isinstance(cache[key], dict) and cache[key].get("expires", 0) < time.time():
            del cache[key]
            changed = True
    if changed:
        _save_cookie_cache(cache)


# ================================================================
# 请求头
# ================================================================
STATIC_HEADERS = {
    "accept": "*/*",
    "accept-encoding": "gzip, deflate, br",
    "accept-language": "zh-CN,zh;q=0.9,en;q=0.8",
    "cache-control": "no-cache",
    "pragma": "no-cache",
    "referer": "https://www.midishow.com/",
    "sec-ch-ua": '"Not A(Brand";v="8", "Chromium";v="132", "Google Chrome";v="132"',
    "sec-ch-ua-mobile": "?0",
    "sec-ch-ua-platform": '"Windows"',
    "sec-fetch-dest": "script",
    "sec-fetch-mode": "no-cors",
    "sec-fetch-site": "cross-site",
    "user-agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
                  "(KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36"
}


def _gen_req_headers(args=None):
    if args is None:
        args = {}
    headers = {
        "accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,"
                  "image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7",
        "accept-encoding": "gzip, deflate, br",
        "accept-language": "zh-CN,zh;q=0.9",
        "connection": "keep-alive",
        "sec-ch-ua": '"Chromium";v="140", "Not=A?Brand";v="24", "Google Chrome";v="140"',
        "sec-ch-ua-mobile": "?0",
        "sec-ch-ua-platform": '"Linux"',
        "sec-fetch-dest": "document",
        "sec-fetch-mode": "navigate",
        "sec-fetch-site": "none",
        "sec-fetch-user": "?1",
        "upgrade-insecure-requests": "1",
        "user-agent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
                      "(KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36"
    }
    headers.update(args)
    return headers


# ================================================================
# 解码函数
# ================================================================
def _decode_base64(encoded_str: str, chr_set: str) -> bytes:
    """自定义 Base64 解码（midishow 专用）"""
    standard_charset = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/="
    trans_table = str.maketrans(chr_set, standard_charset)
    standard_encoded = encoded_str.translate(trans_table)
    return base64.b64decode(standard_encoded)


def _hex2str(hex_str: str) -> str:
    """十六进制转字符串"""
    result = ""
    for i in range(0, len(hex_str), 2):
        if hex_str[i:i + 2] == "00":
            break
        result += chr(int(hex_str[i:i + 2], 16))
    return result


# ================================================================
# MidiShow API
# ================================================================
class MidiShowAPI:
    """midishow 下载 API（基于 midishow-downloader-selfhost 核心逻辑）"""

    def __init__(self):
        self.session = requests.Session()

    # ---------- 登录 ----------
    def login_by_cookies(self, cookies: list):
        for cookie in cookies:
            self.session.cookies.set(*cookie)

    def login_by_password(self, username: str, password: str) -> bool:
        headers = _gen_req_headers({
            "Cache-Control": "max-age=0",
            "Content-Type": "application/x-www-form-urlencoded",
            "Origin": "https://www.midishow.com",
            "Referer": "https://www.midishow.com/user/account/login"
        })
        try:
            csrf_token = self._get_csrf_token("https://www.midishow.com/user/account/login")
        except Exception as e:
            raise Exception(f"无法获取登录页面（网络错误）: {e}")
        data = {
            "_csrf": csrf_token,
            "LoginForm[identity]": username,
            "LoginForm[password]": password,
            "login-button": ""
        }
        resp = self.session.post(
            "https://www.midishow.com/user/account/login",
            headers=headers, data=data, allow_redirects=False, timeout=20
        )
        if "Location" in resp.headers:
            return True
        # F4: 部分部署登录成功返回 200（无重定向）。以"是否获得登录态 cookie"作为兜底成功判据，
        # 避免把成功的登录误判为密码错误
        if resp.status_code in (200, 302):
            cookie_names = {c.name.lower() for c in self.session.cookies}
            if any(("identity" in n or "auth" in n) for n in cookie_names):
                return True
        # Check for specific error messages in response
        if resp.status_code == 403:
            raise Exception("账号被风控（HTTP 403），请稍后重试")
        if resp.status_code == 422 or resp.status_code == 400:
            # Try to extract error from HTML
            try:
                from bs4 import BeautifulSoup
                soup = BeautifulSoup(resp.text, "html.parser")
                error_div = soup.find("div", class_="help-block") or soup.find("div", class_="error")
                if error_div:
                    raise Exception(f"登录失败: {error_div.get_text(strip=True)}")
            except Exception:
                pass
        raise Exception("用户名或密码错误")

    def export_cookies(self) -> list:
        return list(self.session.cookies.items())

    # ---------- CSRF ----------
    def _get_csrf_token(self, page_url: str) -> str:
        headers = _gen_req_headers()
        resp = self.session.get(page_url, headers=headers, timeout=20)
        soup = BeautifulSoup(resp.text, "html.parser")
        csrf_tag = soup.find_all("meta", {"name": "csrf-token"})[0]
        return csrf_tag.attrs["content"]

    # ---------- 下载 ----------
    def download_midi(self, page_url: str):
        """
        下载 MIDI 文件。
        参数: page_url - midishow 查看页面 URL
        返回: (midi_bytes, midi_title) 或 (None, None) 失败
        """
        headers = _gen_req_headers({"Referer": "https://www.midishow.com/"})
        resp = self.session.get(page_url, headers=headers, timeout=20)
        resp.raise_for_status()
        soup = BeautifulSoup(resp.text, "html.parser")

        # 获取标题
        container = soup.find("div", attrs={"class": "ms-player-container"})
        if container:
            h1 = container.find("h1")
            midi_title = h1.get_text().strip() if h1 else "Unknown"
        else:
            # 备用标题提取
            h1 = soup.find("h1")
            title_tag = soup.find("title")
            midi_title = h1.get_text().strip() if h1 else (
                title_tag.get_text().strip() if title_tag else "Unknown"
            )
            # 清理标题
            midi_title = re.sub(r'\s*-\s*MidiShow.*$', '', midi_title).strip()

        # 获取 data-mid 和 data-id
        div0 = soup.find_all(lambda tag: tag.has_attr("data-mid"))
        if not div0:
            return None, None
        div0 = div0[0]
        fake_midi_url = div0.attrs["data-mid"]
        midi_id = div0.attrs["data-id"]

        # 获取 CSRF token
        csrf_tags = soup.find_all("meta", {"name": "csrf-token"})
        if not csrf_tags:
            return None, None
        csrf_token = csrf_tags[0].attrs["content"]

        # 请求新文件
        rsp1 = self.session.post(
            f"https://www.midishow.com/midi/new-file?id={midi_id}",
            headers=_gen_req_headers({
                "Content-Type": "application/x-www-form-urlencoded; charset=UTF-8",
                "Origin": "https://www.midishow.com",
                "Referer": page_url,
                "X-Csrf-Token": csrf_token,
                "X-Requested-With": "XMLHttpRequest"
            }),
            # B6：必须带超时，否则对端不响应时会永久阻塞整个下载流程
            data={"id": midi_id},
            timeout=20,
        )
        rsp1.encoding = "utf-8"
        if rsp1.status_code == 403:
            return None, None
        # F2: 仅对成功响应做解码；非 200（500/429 等错误页）直接放弃，避免对 HTML 做 base64 解码崩溃
        if rsp1.status_code != 200:
            return None, None

        # 获取编码后的 MIDI 文件
        real_url = (
            fake_midi_url
            .replace("tokeno#:@!", "token", 1)
            .replace("https://www.midishow.com", "https://s.midishow.net")
            .replace(".mid?", ".js?")
        )
        rsp2 = self.session.get(real_url, headers=STATIC_HEADERS, timeout=20)
        if rsp2.status_code != 200:
            return None, None

        # 解码
        chr_set = _hex2str(rsp1.headers.get("Etag", "")) + rsp1.text[56:]
        midi_file = (
            _decode_base64(rsp1.text[28:56], chr_set) +
            _decode_base64(rsp2.text[3:-3], chr_set) +
            _decode_base64(rsp1.text[:28], chr_set)
        )
        return midi_file, midi_title


# ================================================================
# 账户管理器
# ================================================================
class AccountManager:
    """管理 midishow 账号，自动缓存 Cookie"""

    def __init__(self):
        self.accounts = []  # [(username, password), ...]
        self._api_cache = {}  # username -> MidiShowAPI instance

    def add_account(self, username: str, password: str):
        """添加账号"""
        for i, (u, p) in enumerate(self.accounts):
            if u == username:
                self.accounts[i] = (username, password)
                return
        self.accounts.append((username, password))

    def remove_account(self, username: str):
        """移除账号"""
        self.accounts = [(u, p) for u, p in self.accounts if u != username]
        self._api_cache.pop(username, None)

    def get_accounts(self) -> list:
        return list(self.accounts)

    def get_api(self, username: str = None) -> MidiShowAPI:
        """
        获取已登录的 API 实例。
        如果指定 username，使用该账号；否则随机选择一个。
        """
        if not self.accounts:
            raise Exception("未设置 midishow 账号，请在设置中添加账号")

        if username:
            accounts = [(u, p) for u, p in self.accounts if u == username]
        else:
            accounts = self.accounts

        if not accounts:
            raise Exception(f"账号 {username} 不存在")

        # 尝试从缓存中获取
        for username, password in accounts:
            if username in self._api_cache:
                api = self._api_cache[username]
                return api

            # 尝试从文件缓存加载 cookie
            cookie_cache = _load_cookie_cache()
            cached = cookie_cache.get(username)
            if cached and isinstance(cached, dict) and cached.get("cookies"):
                # Check if cookies are expired
                expires = cached.get("expires", 0)
                if expires < __import__("time").time():
                    # Cookies expired, need fresh login
                    pass
                else:
                    api = MidiShowAPI()
                    api.login_by_cookies(cached["cookies"])
                    self._api_cache[username] = api
                    return api

            # 重新登录
            api = MidiShowAPI()
            if api.login_by_password(username, password):
                # 缓存 cookie
                cookies = api.export_cookies()
                cache = _load_cookie_cache()
                cache[username] = {
                    "cookies": cookies,
                    "expires": time.time() + 24 * 3600
                }
                _save_cookie_cache(cache)
                self._api_cache[username] = api
                return api
            # B5: 该账号登录失败不直接中断，继续尝试列表中其余账号（真正的多账号轮换）
            continue

        raise Exception("所有账号登录失败")

    def clear_cache(self):
        """清除所有缓存"""
        self._api_cache = {}
        if os.path.exists(COOKIE_CACHE_FILE):
            try:
                os.remove(COOKIE_CACHE_FILE)
            except:
                pass


# ================================================================
# 全局单例
# ================================================================
_account_manager = AccountManager()


def get_account_manager() -> AccountManager:
    # Clean expired cookies on first access
    _clear_expired_cookies()
    return _account_manager


def _is_midishow_host(url: str) -> bool:
    """S3: 仅允许 midishow 官方域名（含子域），拒绝内网/元数据/任意外部地址，防 SSRF"""
    try:
        p = urlparse(url)
    except Exception:
        return False
    if p.scheme not in ("http", "https"):
        return False
    host = (p.hostname or "").lower()
    return (
        host == "midishow.com"
        or host == "www.midishow.com"
        or host == "s.midishow.net"
        or host.endswith(".midishow.com")
        or host.endswith(".midishow.net")
    )


def download_midi_url(url: str, username: str = None) -> tuple:
    """
    下载 midishow URL 对应的 MIDI 文件。
    返回: (midi_bytes, title)
    """
    if "midishow.com" not in url:
        match = re.search(r'(\d+)', url)
        if match:
            url = f"https://www.midishow.com/en/midi/{match.group(1)}.html"
        else:
            raise Exception("无效的 midishow URL")

    if not url.startswith("http"):
        url = "https://" + url

    # S3: 严格校验目标域名，避免被诱导请求内网/云元数据等地址
    if not _is_midishow_host(url):
        raise Exception("无效的 midishow URL（仅允许 midishow.com 官方域名）")

    api = _account_manager.get_api(username)
    data, title = api.download_midi(url)
    if data is None:
        # May be cookie expired - clear only this account's cache, not all accounts
        if username:
            _account_manager._api_cache.pop(username, None)
        else:
            _account_manager.clear_cache()
        api = _account_manager.get_api(username)
        data, title = api.download_midi(url)
        if data is None:
            raise Exception("下载失败，可能是账号被风控或网络问题，请稍后重试或更换账号")
    return data, title
