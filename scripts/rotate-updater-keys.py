"""替换 plugins.updater.pubkey：用 ~/.tauri/vrcdog.key.pub 的内容做替换。"""
from pathlib import Path
import re, base64

conf = Path(r"C:/Users/Administrator/Documents/Project/VRCDog/src-tauri/tauri.conf.json")
pub = Path(r"C:/Users/Administrator/.tauri/vrcdog.key.pub")

raw = pub.read_text(encoding="utf-8")
new_pub_b64 = raw.replace("\r", "").replace("\n", "").strip()
try:
    decoded_head = base64.b64decode(new_pub_b64, validate=True).decode("utf-8", errors="replace")[:80]
except Exception as e:
    raise SystemExit(f"base64 decode failed: {e}")
assert decoded_head.startswith("untrusted comment: minisign public key"), f"unexpected decode: {decoded_head!r}"
print("decoded pubkey head:", decoded_head)

text = conf.read_text(encoding="utf-8")
m = re.search(r'"pubkey":\s*"([^"]+)"', text)
assert m, "pubkey field not found"
old = m.group(1)
text = text.replace(old, new_pub_b64)

raw_old = conf.read_bytes()
new_raw = text.encode("utf-8")
if b"\r\n" in raw_old and b"\r\n" not in new_raw:
    new_raw = new_raw.replace(b"\n", b"\r\n")
conf.write_bytes(new_raw)
print(f"rotated: old={old[:40]}...")
print(f"       new={new_pub_b64[:40]}...")
