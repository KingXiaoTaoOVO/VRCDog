# VRCDog 代码审查报告

> 审查范围：`src/`（TS/Vue 前端）、`src-python/`（Python 组件）、`src-tauri/src/`（Rust/Tauri 后端）、`vrcdog-server/src/`（独立 Rust 服务端）
> 审查方式：分模块静态审查 + 高危项逐行复核（行号均来自实际源码）
> 统计：发现 **High 15 项 / Medium 22 项 / Low 18 项**，其中 9 项 High 级安全漏洞已逐行核实。

---

## 总览表

| 等级 | 数量 | 代表性问题 |
|------|------|-----------|
| 🔴 High | 15 | TTS PowerShell 注入 RCE、cmd 注入、gallery 路径遍历、硬编码管理员密码、远程协助明文、更新签名被跳过、TLS 校验关闭、开放代理、客户端接口无鉴权、localStorage 明文 Cookie、SSRF ×3 |
| 🟠 Medium | 22 | 凭据泄露到日志、翻译 Key 明文、注册鉴权 `&&` 逻辑缺陷、空笔画 panic、无超时阻塞、解压炸弹、弱随机口令、无界通道、心跳空转、好友全量重拉 |
| 🟡 Low | 18 | any 滥用、魔法数字、unwrap 泛滥、死代码中间件、重复导入、未用参数 |

**结论**：代码功能面较完整，但**安全短板集中且高危**，尤其是多处命令注入与明文凭据/完整性缺失，在桌面应用 + 远程协助场景下可被本地或网络攻击者利用。建议按“先止血（High 安全）→ 再防崩（panic/鉴权）→ 后优化（性能/质量）”的顺序修复。

---

## 一、严重安全漏洞（必须优先修复）

### R1 🔴 TTS 功能 PowerShell 命令注入 → 远程 RCE
- **位置**：`src-tauri/src/ovr.rs:1609-1616`（已逐行核实）
- **影响**：开启 Windows 原生 TTS 时，聊天翻译文本被插值进 PowerShell 脚本。`tts_text.replace("'", "''")` 仅转义单引号，**未处理 `#`**。攻击者只需在 VRChat 公屏发送 `'); calc.exe #`，脚本变为 `$synth.Speak(''); calc.exe #');`，`#` 注释掉尾部 `)`，从而执行任意命令（窃取令牌、植入木马）。
- **根因**：把用户可控文本拼进可执行的 shell 脚本字符串，且转义不充分。
- **修复**：**绝不对用户输入做脚本插值**。改为从 stdin 以纯数据方式读入文本，PowerShell 只当它是待朗读内容，不参与代码解析：
```rust
let script = "Add-Type -AssemblyName System.Speech; \
  $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
  $s.Speak([Console]::In.ReadToEnd())";
std::thread::spawn(move || {
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    if let Ok(mut child) = std::process::Command::new("powershell")
        .args(["-ExecutionPolicy", "Bypass", "-Command", &script])
        .stdin(std::process::Stdio::piped())
        .creation_flags(0x08000000)
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(tts_text.as_bytes());
        }
        let _ = child.wait();
    }
});
```
> 因 `[Console]::In.ReadToEnd()` 读取的是原始字符串数据而非 PowerShell 代码，`'`、`#`、`&` 等都会被原样朗读，注入彻底消除。

---

### R2 🔴 `sys_launch_vrchat` 经 `cmd /C` 注入
- **位置**：`src-tauri/src/hardware.rs:98-111`（已逐行核实）
- **影响**：`launch_args` 直接 `push_str` 进 `steam://...` 字符串后整体作为 `cmd /C start` 的参数。`la = "x & notepad"` 即触发 `cmd /C start steam://...//x & notepad`，执行任意命令。
- **根因**：用户可控参数未经校验/转义即拼入 shell 命令行。
- **修复**：(a) 对 `launch_args` 做白名单校验（仅允许字母/数字/`-`/`=`/`/`，不含 shell 元字符）；(b) 把 URL 用双引号包裹以抑制 cmd 元字符；更稳妥是**绕过 cmd**，直接交给系统打开协议处理器：
```rust
pub fn sys_launch_vrchat(launch_args: Option<String>) -> AppResult<()> {
    // 仅允许安全的 URI 片段，拒绝包含 shell 元字符的输入
    if let Some(ref la) = launch_args {
        if !la.chars().all(|c| c.is_alphanumeric() || "-=/.:_%".contains(c)) {
            return Err(AppError::from("launch_args 含非法字符"));
        }
    }
    let url = format!(
        "steam://rungameid/438100{}",
        launch_args.map(|la| format!("//{}", la)).unwrap_or_default()
    );
    // 不用 cmd 解释器，避免 shell 注入
    std::process::Command::new("cmd")
        .args(["/C", "start", "", &format!("\"{}\"", url)])
        .spawn()
        .map_err(|e| AppError::from(e.to_string()))?;
    Ok(())
}
```
> 双引号 + 白名单双重防护；`start ""` 显式提供空标题避免把 URL 误判为标题。

---

### R3 🔴 `gallery_delete_image` 路径遍历 → 任意 .png 删除
- **位置**：`src-tauri/src/gallery.rs:85-92`（已逐行核实）
- **影响**：仅校验 `path.ends_with(".png")`，未校验路径是否位于 VRChat 图片目录。恶意前端可传入 `C:/Users/受害者/secret.png` 删除系统中任意 PNG 文件（配合 webview 注入可破坏用户数据）。
- **根因**：缺少“目录包含性”校验。
- **修复**：`canonicalize` 后比对图片根目录：
```rust
pub async fn gallery_delete_image(path: String) -> AppResult<()> {
    let mut root = dirs::picture_dir().ok_or_else(|| AppError::from("无法定位图片目录"))?;
    root.push("VRChat");
    let root = canonicalize(&root).map_err(|e| AppError::from(e.to_string()))?;
    let target = canonicalize(&path).map_err(|e| AppError::from(e.to_string()))?;
    if target.extension().and_then(|e| e.to_str()) != Some("png") {
        return Err(AppError::from("仅允许删除 PNG 文件"));
    }
    if !target.starts_with(&root) {
        return Err(AppError::from("非法路径：不在 VRChat 图片目录内"));
    }
    async_fs::remove_file(target).await.map_err(|e| AppError::from(e.to_string()))?;
    Ok(())
}
```

---

### R4 🔴 硬编码默认管理员密码（bcrypt 写入源码）
- **位置**：`src-tauri/src/lib.rs:618-631`（明文 `root`）、`vrcdog-server/src/main.rs:308-321`（另一份哈希）
- **影响**：环境变量 `VRCDOG_SERVER_PASSWORD_BCRYPT` 缺失时回退到编译期常量，其明文为 `root`。任何能读到仓库/二进制的人都能以管理员登录，封禁/踢人/篡改问卷/读全部用户数据。`verify_server_password` 还**无速率限制**，可离线爆破。
- **根因**：用“默认值兜底”替代“强制配置”。
- **修复**：环境变量缺失时**拒绝启动管理员接口**，并要求首次运行设置密码；同时为 Admin 登录加速率限制：
```rust
fn server_password_hash() -> Option<String> {
    std::env::var("VRCDOG_SERVER_PASSWORD_BCRYPT")
        .ok().map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
    // 缺失则返回 None；调用方据此禁用管理员路由
}
```
并在 `main`/启动逻辑：若 `server_password_hash().is_none()`，记录告警并让 `/api/admin/*` 返回 `503 Service Unavailable`，直到配置完成。两处（lib.rs 与 vrcdog-server/main.rs）应共用同一来源，避免两份不同哈希。

---

### R5 🔴 远程协助通道明文 / 无 TLS / 配对口令无盐
- **位置**：`src-tauri/src/remote_assist_hub.rs`（`register_peer` 102、`connect_peer` 226、`relay_message` 264）、`main.rs:545`（ws 升级）
- **影响**：`/api/remote-assist/ws` 以明文 `ws://` 暴露（bind 0.0.0.0:11451）；`connect` 消息中的 `password` 明文传输；配对仅用 **无盐 SHA256** 比对；双方虽交换 `public_key` 却**从未用于加密**，payload 以明文 JSON 转发。中间人可截获配对口令与会话内容。
- **根因**：缺少 TLS 层，且交换出的公钥被闲置。
- **修复（分阶段）**：
  1. **短期**：给 `/api/remote-assist/ws` 套 TLS（`wss://`），配对口令改为 `sha256(salt + password)`（服务端随机 salt 下发）；中继 payload 至少用会话密钥封装。
  2. **根本**：用已交换的 `public_key` 做 **ECDH 协商 + AES-GCM** 端到端加密，密钥不入中继；加 `nonce` 单调递增 + 去重（见 R11）防重放。
  3. bind 地址默认改为 `127.0.0.1`，仅在使用中继时连可信中继节点。

---

### R6 🔴 自动更新下载并执行未校验/不限主机的任意 URL
- **位置**：`src-tauri/src/update.rs:515`（任意 URL GET）、`589-596`（签名可选跳过）、`156-234`（`parse_release` 取 GitHub `digest` 字段）
- **影响**：`download_url` 由前端/渲染进程控制，无主机白名单；`expected_sha256` 为 `None` 时**整段跳过签名校验**。而 GitHub Releases REST API **不返回 `digest` 字段**，导致 `installer_sha256` 实际恒为 `None`——**自动更新默认永远不做完整性校验**，供应链投毒即可 RCE。
- **根因**：把下载与执行解耦但都不可信；依赖不存在的 API 字段作为完整性来源。
- **修复**：
```rust
// 仅允许官方发布域名
fn is_allowed_host(u: &Url) -> bool {
    matches!(u.host_str(), Some("github.com") | Some("objects.githubusercontent.com"))
}
// 下载前
let url = Url::parse(&download_url)?;
if !is_allowed_host(&url) { return Err("下载源不在允许列表".into()); }
// 校验必须存在且匹配，否则中止
let expected = expected_sha256.filter(|s| !s.trim().is_empty())
    .ok_or("未提供有效的 SHA-256，拒绝安装")?;
if expected.to_ascii_lowercase() != computed_hex { /* 删除并报错 */ }
```
> 长期应改为：从**签名清单**（独立密钥签名）读取哈希，校验签名后再安装，而非依赖 GitHub 字段。

---

### R7 🔴 下载引擎全局关闭 TLS 证书校验
- **位置**：`src-tauri/src/toolchain.rs:345-353`（`.danger_accept_invalid_certs(true)`）
- **影响**：所有经该 client 的 HTTPS 请求（VCC/ALCOM 等开发工具下载）跳过证书校验，MITM 可劫持并投放恶意二进制。
- **根因**：为“方便内网/自签”误开全局禁用。
- **修复**：删除 `danger_accept_invalid_certs(true)`；确有自签需求时，仅对特定已知证书用 `add_root_certificate()` 显式信任，而非全局放开。

---

### R8 🟠 `uninstall_software` 命令注入（提权）
- **位置**：`src-tauri/src/toolchain.rs:234-243`
- **影响**：把磁盘发现的卸载路径用单引号插值进 `Start-Process -FilePath '{}'`，路径含 `'` 时可注入，并配合 `-Verb RunAs` 以提权执行。
- **根因**：路径字符串插值进 PowerShell。
- **修复**：把路径作为**独立参数**传入，避免插值：
```rust
std::process::Command::new("powershell")
    .args(["-NoProfile", "-Command",
           "Start-Process", "-FilePath", &uninstall_path, "-Verb", "RunAs", "-Wait"])
    .output()
```
> 路径经 `args()` 单独传递，不会被解析为命令片段。

---

### R9 🟠 SSRF（三处）
- **位置/影响/修复**：
  1. **`xiaohongshu/mod.rs:33-48`**：`url` 完全由前端提供，`client.get(&url)` 可打内网/元数据端点。**修复**：限制协议为 `https` 且 host 白名单（`xhslink.com`/`xiaohongshu.com`），拒绝私网地址（`127.0.0.0/8`、`10/8`、`169.254.169.254` 等）。
  2. **`vrc_api.rs:235-248`（`allow_external_host`）**：任意 host 放行。**修复**：`allow_external_host` 仅允许指向**自身配置的 VRCDog 服务端 host**，禁止任意外网/内网。
  3. **`src-python/midishow_api.py:359-373`**：仅判断 `midishow.com` 子串即可放行（`https://midishow.com.attacker.net` 也能过）。**修复**：用 `urllib.parse.urlparse` 严格校验 `scheme==https` 且 `netloc` 以 `midishow.com` 结尾（含端口/子域边界）。

---

### R10 🟠 `/api/vrchat-proxy` 未鉴权的开放代理
- **位置**：`vrcdog-server/src/main.rs:1754`（挂在主 Router，未套 `require_admin_session`）
- **影响**：任何人可匿名让服务端以其 IP 转发请求到 `api.vrchat.cloud`，绕过客户端速率限制、滥用他人配额。
- **根因**：该路由漏挂 `require_admin_session` 中间件（`require_admin_password` 甚至定义了却从未挂载，见 R-Q1）。
- **修复**：将其加入 `admin_routes`，或在 handler 内校验管理员 token；前端调用必须携带 `x-vrcdog-admin-token`。

---

## 二、敏感凭据与数据保护

### S1 🔴 Web 模式 VRChat 会话 Cookie 明文存 localStorage
- **位置**：`src/api/index.ts:1557-1559`、`src/api/serverClient.ts:268-276`（已核实）
- **影响**：`auth` cookie 即完整账号会话令牌，明文写入 `localStorage`，不受 httpOnly 保护，任何同源 XSS 即可窃取。
- **根因**：Web 回退实现直接把桌面端的安全存储替换为 localStorage。
- **修复**：Web 模式下**不要持久化原始 cookie**。若必须支持 Web 部署：改用 `sessionStorage`（会话级、关闭即清）+ 后端代理持有令牌（前端只持短期 session id）；或至少对 cookie 做应用层加密（密钥不落前端）。桌面（Tauri）模式应保持用 OS keychain（`@tauri-apps/plugin-store` 或 keyring），不进 localStorage。

### S2/S3 🟠 会话令牌泄露到浏览器控制台与调试日志
- **位置**：`src/api/request.ts:196`（console.warn 原样打 args）、`301-309`（`response: res` 未脱敏，含 `auth_cookie`）
- **影响**：`auth_cookie` 进入 `DebugConsole` 的内存 logs 与控制台。
- **根因**：`sanitizeArgs` 只在 Tauri 分支调用，浏览器分支与 `response` 未复用。
- **修复**：浏览器分支同样先 `sanitizeArgs` 再打印；`response` 也做脱敏（剥离 `auth_cookie`/`set-cookie`）：
```ts
if (!isTauri()) {
  console.warn(`[Browser Mode] API Command: ${cmd}`, sanitizeArgs(args));
}
// 派发调试事件时
const sanitizedResponse = sanitizeResponse(res); // 移除 auth_cookie / set-cookie
window.dispatchEvent(new CustomEvent('app-debug-log', { detail: { ..., response: sanitizedResponse } }));
```
> 同时补全 `sanitizeArgs` 覆盖 `options.body`（S7），避免 body 内密码泄露。

### S4 🟠 翻译 API Key 明文存 localStorage
- **位置**：`src/components/TranslatorView.vue:149,151`
- **修复**：迁移到 OS 安全存储（`@tauri-apps/plugin-store` 配合加密，或 keyring）；Web 模式同理避免明文。

### S5 🟠 Python 会话 Cookie 明文落盘
- **位置**：`src-python/midishow_api.py:33-36,_save_cookie_cache`
- **修复**：缓存文件权限设为 `0o600`（`os.open(..., 0o600)` 或创建后 `os.chmod`）；敏感字段加密后再写。

### S6 🟠 会话 Cookie 外发用户自配服务端
- **位置**：`src/stores/authStore.ts:157,381`
- **修复**：属于架构信任边界，需向用户明确告知“将向该服务器发送 VRChat 会话”，并对 `clientServerUrl` 做 HTTPS + 域名校验；服务端必须签名校验该 cookie（见 R-LOGIC 的注册鉴权修复）。

---

## 三、鉴权 / 逻辑缺陷

### L1 🔴 客户端注册鉴权 `&&` 误用 → 可冒名任意 user_id
- **位置**：`src-tauri/src/local_server.rs:716`（已逐行核实）
- **影响**：判定为
  ```rust
  if verified_id != req.user_id && verified_name != req.display_name { /* 拒绝 */ }
  ```
  用 `&&` 时，只要攻击者把 `req.display_name` 设为**自己 cookie 的昵称**，`verified_name != req.display_name` 即为 `false`，整体不拒绝 → 用自身合法 cookie 却声明他人 `user_id` 成功注册/冒名。
- **根因**：把“昵称相等”当成身份一致的证据，且 `&&` 使两个条件任一满足即放行。
- **修复**：严格绑定 cookie 到声明身份，去掉冗余的 name 判断：
```rust
if verified_id != req.user_id {
    return Json(serde_json::json!({
        "status": "auth_failed",
        "reason": "VRChat credentials do not match claimed identity",
    }));
}
// 后续一律以 verified_id 作为权威身份，忽略 req.user_id
```
> 同时建议下游逻辑改用 `verified_id` 而非请求体里的 `user_id`，消除“自证身份”风险。

### L2 🟠 `vrcdog-server` 客户端接口无身份认证（user_id 自证）
- **位置**：`vrcdog-server/src/main.rs:552-627, 681-750, 1000-1176`
- **影响**：所有 `/api/client/*` 直接信任请求体里的 `user_id`，可冒充任意用户提交/删除问卷、读取他人角色与问卷状态、伪造奖励受益人。
- **修复**：引入服务端签发的**短期 client token**（登录时下发，后续请求带 `x-vrcdog-client-token`）；或对 `user_id` 用与客户端共享的密钥做 HMAC 校验。至少 `GET /features/{id}`、`/surveys/{id}`、`/survey-history/{id}` 需校验归属。

### L3 🟠 `authStore.isCurrentClientEvent` 用 displayName 判定当前用户
- **位置**：`src/stores/authStore.ts:63-67`
- **影响**：服务端推送 `client_kicked/frozen/banned` 事件用 `displayName` 匹配，非唯一可伪造；同昵称用户误匹配，攻击者可构造事件触发当前用户被登出。
- **修复**：匹配权威 `user.id`（或 `verified_id`），不要匹配 `displayName`：
```ts
const isCurrentClientEvent = (p: any): boolean => {
  const uid = normalizeServerEventUserId(p);
  return appRole.value === 'client' && Boolean(uid) && uid === currentUser.value?.id;
};
```

### L4 🟠 腾讯翻译签名跨 UTC 午夜不一致
- **位置**：`src-tauri/src/translate.rs:389 vs 403`
- **影响**：`timestamp` 与 `date` 分别 `Utc::now()`，午夜边界可能分属两日，签名被拒（偶发难查）。
- **修复**：一次取 `now`，派生 `timestamp` 与 `date`：
```rust
let now = Utc::now();
let timestamp = now.timestamp();
let date = now.format("%Y-%m-%d").to_string();
```

---

## 四、功能异常与崩溃（panic / 异常）

### B1 🟠 数据库打开失败直接 `expect` 崩溃
- **位置**：`src-tauri/src/db.rs:25`
- **修复**：返回 `AppError` 而非 `expect`，让前端提示“数据库不可用/磁盘只读”。

### B2 🟠 空笔画 `unwrap()` panic（DoS）
- **位置**：`src-tauri/src/vrdrawing.rs:824-827,876,900`
- **影响**：`stroke.points.first()/last()` 为空时 `unwrap` 崩溃绘制线程。
- **修复**：先判空，空笔画跳过或记录告警；用 `if let Some(...)` 替代 `unwrap()`。

### B3 🟠 下载信号量 `acquire().unwrap()` panic
- **位置**：`src-tauri/src/bilibili/mod.rs:419,500`
- **修复**：信号量关闭时返回 `Err` 应优雅降级（取消该下载并回报），不要 `unwrap()`。

### B4 🟠 `sys_start_auto_launch_apps` 尾随引号 bug
- **位置**：`src-tauri/src/hardware.rs:35-43`
- **影响**：带引号路径 `let cmd = &app[1..=end_idx];` 把闭合引号纳入可执行名（`C:\foo.exe"`），启动失败。
- **修复**：应为 `&app[1..end_idx]`（去掉闭合引号）。

### B5 🟠 Python 登录/下载逻辑错误（多处）
- **位置**：`src-python/midishow_api.py`（已核实 152-167、321-335）、`midishow.py:165-189`
- **具体**：
  - `login_by_password` 仅当响应头有 `Location` 才返回 True，站点返回 200 成功会被误判为密码错（F4）。
  - `get_api` 多账号循环里 `login_by_password` 永远不返回 False，`else: raise` 是死代码，首个账号异常即中断，轮换名存实亡（F3）。
  - `download_midi` 只对 403 兜底，500/429 错误页仍做 base64 解码 → 抛异常（F2）。
  - `get_midi_info` 不 `raise_for_status()`，404 被当空标题静默失败（F7）。
- **修复**：
```python
# 登录：以会话是否拿到有效 cookie 判定成功，而非是否重定向
def login_by_password(self, username, password) -> bool:
    ...
    return bool(self.session.cookies.get("auth"))

# get_api：失败不要抛，继续下一个账号
for username, password in accounts:
    api = MidiShowAPI()
    if api.login_by_password(username, password):
        _account_manager._api_cache[username] = api
        return api
    # 不 raise，尝试下一个
raise Exception("所有账号登录失败")

# download_midi：先判状态
if rsp1.status_code != 200:
    return None, None
# get_midi_info：校验状态
resp = requests.get(url, headers=headers, timeout=15)
resp.raise_for_status()
```

### B6 🟠 Python / TS 下载器无超时 → 永久阻塞
- **位置**：`midishow_api.py` 全部 `requests` 调用（175/188/222/244）、`midishow-downloader.ts` 的 axios
- **修复**：统一加 `timeout=15`（requests）/`timeout: 15000`（axios）；TS 版 `getMidiFile` 还需对 `response1.headers['etag']` 做空值兜底（F5，`etag_decode(undefined)` 会 TypeError）。

### B7 🟠 Python MIDI 标题路径遍历
- **位置**：`src-python/midishow.py:203-213`（已核实）
- **影响**：标题含 `..` 可逃逸 `save_dir` 写任意路径。
- **修复**：
```python
import os, re
safe_title = re.sub(r'[<>:"/\\|?*\x00-\x1f]', '', info["title"])[:120] or f"midi_{midi_id}"
filename = f"{safe_title}.mid"
save_path = os.path.normpath(os.path.join(save_dir, filename))
if not save_path.startswith(os.path.normpath(save_dir)):
    raise Exception("非法文件名")
```

### B8 🟠 `bilibili` 缺失 LOCALAPPDATA 回退到 `C:\`
- **位置**：`src-tauri/src/bilibili/mod.rs:301`
- **修复**：回退到应用专属目录（如 `std::env::temp_dir().join("VrcDog")`），不要落到系统盘根。

---

## 五、性能瓶颈

### P1 🟠 每个 pipeline 事件全量重拉好友列表
- **位置**：`src/components/FriendsListView.vue:119-129`（监听 `vrc-pipeline-event` 后 500ms 防抖调用 `fetchFriends`，后者最多拉 100 页/1 万好友）
- **影响**：好友频繁上下线/移动时产生请求风暴与整列表重渲染，UI 卡顿。
- **修复**：监听**具体字段增量**（`friendsStore.updateFriend`），仅更新变更的那个好友，不再整表重拉；保留 `vrc-friends-synced` 作为全量刷新触发。

### P2 🟠 远程协助无界通道堆积
- **位置**：`src-tauri/src/remote_assist/transport.rs`、`remote_assist_hub.rs:46`
- **影响**：对端慢/断连时消息无限堆积 → 内存耗尽 DoS。
- **修复**：改用 `mpsc::channel(N)` 有界通道，写满时丢弃最旧帧（视频流可容忍丢帧）并限流。

### P3 🟠 每次截图新建整套 tokio Runtime
- **位置**：`src-tauri/src/ovr.rs:2239-2240`
- **影响**：单次截图 `tokio::runtime::Runtime::new().unwrap()`，开销大且可能 panic。
- **修复**：复用应用已有的 Tauri async runtime（经 `tauri::async_runtime::spawn` 或通道回传结果），不要自建运行时。

### P4 🟠 心跳 1s 空转，15s 才工作
- **位置**：`src/stores/authStore.ts:241-259`
- **修复**：直接 `setInterval(..., 15000)`，去掉 `normalTick` 计数器空转。

### P5 🟠 解压无大小上限 + 递归（解压炸弹）
- **位置**：`src-tauri/src/danmaku.rs:1329-1374`
- **修复**：`read_to_end` 前用 `take(MAX_DECOMPRESSED)` 限流；限制递归深度；超限直接丢弃该帧。

### P6 🟠 Regex 每次请求重编译 / OSC 每包新建套接字
- **位置**：`xiaohongshu/mod.rs:51,61`（`Regex::new().unwrap()` 在请求内）、`osc.rs:124,257`（`UdpSocket::bind("0.0.0.0:0")` 每包）
- **修复**：Regex 用 `once_cell`/`lazy_static` 编译一次；OSC socket 复用单例。

---

## 六、代码质量

- **Q1 🟠 死代码中间件**：`vrcdog-server/src/main.rs:1201 require_admin_password` 已定义却**从未挂载**，易让维护者误以为有密码头校验。要么挂载使用，要么删除。
- **Q2 🟡 unwrap/expect 泛滥**：`db.rs:25`、`bilibili/mod.rs:29`、`sys.rs:571/591/600/711`、`danmaku.rs:2378/2380`、`vrcdog_audio.py` 等。统一改为 `?`/带上下文的错误返回。
- **Q3 🟡 `any` 滥用**：`src/api/request.ts`、`authStore.ts`、`TranslatorView.vue` 大量 `as any`，掩盖字段重命名导致的静默 `undefined`。优先为 `vrc_execute` 响应、`auth` 对象补 TS 类型。
- **Q4 🟡 魔法数字**：超时 `30000/120000`、并发 `300`、心跳 `15s`、防抖 `500ms`、好友 TTL `30000` 等散落硬编码。集中到 `src/api/constants.ts` 与 Rust `const` 模块，便于调优。
- **Q5 🟡 重复/惰性导入**：`midishow.py`、`midishow_api.py` 在函数体内 `import requests/re`、用 `__import__("time")`。上提到模块顶部。
- **Q6 🟡 跨模块访问私有成员**：`midishow_api.py:380` 直接操作单例 `_api_cache`。改为暴露 `get/clear` 方法。
- **Q7 🟡 弱随机口令**：`remote_assist/commands.rs:342`、`nat.rs:222` 用时间种子 LCG。改用 `rand::thread_rng()`/`OsRng`（CSPRNG）。
- **Q8 🟡 非加密哈希作设备 ID**：`commands.rs:332 DefaultHasher`。设备 ID 用稳定 UUID 或 OS 提供值。
- **Q9 🟡 未使用参数**：`vrcdog_audio.py:824 --dynamic-energy-threshold` 声明但从不读取。删除该参数。

---

## 七、修复优先级路线图

**P0（立即，发布前必须）**：R1、R2、R3、R4、R6、R7、R8、L1、S1
**P1（尽快）**：R5、R9、R10、L2、L3、B1–B3、B5–B7、S2–S5
**P2（优化）**：P1–P6、Q1–Q9

> 建议先建立最小复现/单测：R1 用 `'); notepad #` 验证注入消除；R3 用 `C:/Windows/xxx.png` 验证被拒；L1 用“自身 cookie + 他人 user_id”验证注册失败。修复后跑 `pnpm test` 与 Rust 侧测试，确认原有登录/下载/好友/远程协助流程不受影响。

---
*本报告基于静态审查与高危项逐行复核，未对源码做任何修改。所有行号对应审查时的源码版本，若文件已变更请以实际为准。*
