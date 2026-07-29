# 使用 1Panel 部署 vrcdog-server

本文适用于单机 Docker 部署。最终结构为：

```text
VRCDog 客户端
    -> https://vrcdog.example.com
    -> 1Panel OpenResty (TLS / WebSocket 反向代理)
    -> 127.0.0.1:21451 (宿主机代理端口)
    -> vrcdog-server 容器的 11451 端口
    -> /opt/vrcdog-server/data/server-state.json
```

不要把 `11451` 或 `21451` 直接开放到公网。管理密码会随管理请求发送，公网部署必须使用 HTTPS。

## 1. 部署前准备

1. 在域名服务商处添加 `A` 记录，例如 `vrcdog.example.com` 指向服务器公网 IPv4。使用 IPv6 时同时添加 `AAAA`。
2. 云安全组和系统防火墙只需放行 TCP `80`、`443` 和管理员需要的 SSH/1Panel 端口，不要放行 `11451` 或 `21451`。
3. 在 1Panel 的“应用商店”安装 OpenResty；1Panel 的网站反向代理功能依赖 OpenResty。
4. 确认服务器至少有 2 GB 可用内存和 5 GB 可用磁盘。首次 Rust 镜像构建耗时较长，低配服务器建议增加临时交换空间。

1Panel 官方参考：[编排](https://1panel.cn/docs/v2/user_manual/containers/compose/)、[创建网站](https://1panel.cn/docs/v2/user_manual/websites/website_create/)、[申请证书](https://1panel.cn/docs/v2/user_manual/websites/certificate_create/)。

## 2. 上传项目目录

将仓库中的整个 `vrcdog-server` 目录上传到服务器，例如：

```text
/opt/vrcdog-server/
├── Cargo.toml
├── Cargo.lock
├── Dockerfile
├── docker-compose.yml
├── .env
├── data/
└── src/
```

在 1Panel“主机 -> 文件”中创建目录并上传，或通过 Git/SSH 放入该目录。然后在 1Panel 终端执行：

```bash
cd /opt/vrcdog-server
mkdir -p data
chown -R 10001:10001 data
cp .env.example .env
```

容器以 UID `10001` 运行；`data` 目录权限不正确会导致状态文件无法保存。

## 3. 生成管理员密码哈希

不要使用示例中的 `root` 默认密码。以下命令使用临时 Apache 镜像生成 BCrypt cost 12 哈希：

```bash
docker run --rm httpd:2.4-alpine htpasswd -bnBC 12 '' '换成至少16位的随机密码' | tr -d ':\n'
```

把完整输出写入 `/opt/vrcdog-server/.env`，并用单引号包围，防止 Compose 解析哈希中的 `$`：

```dotenv
VRCDOG_BIND_PORT=21451
VRCDOG_SERVER_PASSWORD_BCRYPT='$2y$12$此处替换为刚生成的完整哈希'
RUST_LOG=vrcdog_server=info,tower_http=info
```

限制环境文件读取权限：

```bash
chmod 600 /opt/vrcdog-server/.env
```

客户端后台登录时填写的是生成哈希前的原始密码，不是 BCrypt 字符串。

## 4. 在 1Panel 创建编排

1. 打开“容器 -> 编排 -> 创建编排”。
2. 名称填写 `vrcdog-server`。
3. 推荐选择“路径选择”，选择 `/opt/vrcdog-server/docker-compose.yml`。这样 `build: .` 能找到同目录的 Dockerfile、Cargo 文件和源码。
4. 确认环境变量文件为同目录的 `/opt/vrcdog-server/.env`。
5. 点击“确认/创建”。首次构建需要下载 Rust 和 Debian 基础镜像并编译依赖，请等待编排状态稳定。

当前 Compose 已包含：

- 容器内部监听 `11451`，只发布到宿主机 `127.0.0.1:21451`；
- `/app/data` 持久化挂载；
- 非 root 用户、只读根文件系统和 `no-new-privileges`；
- `/ping` 健康检查；
- 单日志文件 10 MB、最多 3 个文件的轮转。

在 1Panel 终端验证容器：

```bash
cd /opt/vrcdog-server
docker compose ps
curl --fail http://127.0.0.1:21451/ping
```

预期响应包含 `"status":"ok"`。如果容器反复重启，优先检查：

```bash
docker compose logs --tail=200 vrcdog-server
ls -ld /opt/vrcdog-server/data
```

### 端口冲突与旧编排残留

如果启动报错 `Bind for 0.0.0.0:11451 failed: port is already allocated`，不要直接 `kill` `docker-proxy`。`docker-proxy` 是 Docker 为某个容器创建的端口转发进程；只杀进程但不停止所属容器时，Docker 会重新创建它。

先按发布端口定位真实容器名称：

```bash
docker ps -a --filter publish=11451/tcp \
  --format 'table {{.ID}}\t{{.Names}}\t{{.Status}}\t{{.Ports}}\t{{.Labels}}'
docker compose ls --all
```

旧版本未设置 `container_name` 时，名称通常类似 `vrcdog-vrcdog-server-1`，所以执行 `docker rm -f vrcdog-server` 会提示不存在。确认列表中的容器确实是旧 VRCDog 服务后，优先在 1Panel 的旧编排页面执行“停止/删除”，或进入旧编排目录执行：

```bash
docker compose down --remove-orphans
```

不要删除其他业务容器。若 `publish` 过滤器没有结果，但 `ss` 仍显示 `docker-proxy`，查看代理进程参数以取得它转发到的容器 IP：

```bash
ss -ltnp | grep ':11451'
ps -fp <上一步显示的PID>
tr '\0' ' ' </proc/<PID>/cmdline
```

然后把该容器 IP 与容器列表匹配：

```bash
docker inspect -f '{{.Name}} {{range .NetworkSettings.Networks}}{{.IPAddress}} {{end}}' $(docker ps -aq)
```

清理旧编排后，检查本次实际生效的 Compose，而不是只看编辑器内容：

```bash
cd /opt/vrcdog-server
docker compose config
```

端口部分必须显示等价于：

```yaml
ports:
  - target: 11451
    published: "21451"
    host_ip: 127.0.0.1
    protocol: tcp
```

如果仍显示 `0.0.0.0:11451`，说明 1Panel 正在使用另一份 Compose 或其环境配置。删除这次失败创建的 `vrcdog-server` 容器后，从“容器 -> 编排”使用 `/opt/vrcdog-server/docker-compose.yml` 的“路径选择”重新创建。

## 5. 创建反向代理网站

1. 打开“网站 -> 网站 -> 创建网站”。
2. 类型选择“反向代理”。
3. 主域名填写 `vrcdog.example.com`，代理地址填写 `http://127.0.0.1:21451`。
4. 网站代号可填写 `vrcdog-server`，不要开启反向代理缓存。
5. 创建后进入该网站的“配置 -> 反向代理”，确认请求主机头和真实 IP 相关默认配置未被删除。

`vrcdog-server` 的 `/api/remote-assist/ws` 使用 WebSocket。检查 OpenResty 配置中代理 `location` 至少包含：

```nginx
proxy_http_version 1.1;
proxy_set_header Upgrade $http_upgrade;
proxy_set_header Connection "upgrade";
proxy_read_timeout 3600s;
proxy_send_timeout 3600s;
proxy_buffering off;
```

1Panel 创建的反向代理通常会生成 WebSocket 所需配置，但仍应实际检查。官方 WebSocket 说明见 [系统功能 FAQ](https://1panel.cn/docs/v2/faq/system_function/)。修改配置后先使用 1Panel 的配置检查，再重载 OpenResty。

如果代理返回 `502`：

1. 在宿主机确认 `curl http://127.0.0.1:21451/ping` 成功；
2. 确认 Compose 端口是宿主机 `127.0.0.1:21451` 到容器 `11451`；
3. 检查 1Panel OpenResty 的运行网络模式和错误日志；标准 1Panel 网站环境可访问宿主机回环代理端口。

## 6. 配置 HTTPS

1. 打开“网站 -> 证书 -> ACME 账户”，创建 ACME 账户。
2. 创建证书，可选择 DNS 账户自动验证或 HTTP 验证。自动验证方式可开启自动续签。
3. 回到 `vrcdog.example.com` 的“HTTPS”配置，选择该证书并启用 HTTPS。
4. 开启 HTTP 自动跳转 HTTPS。确认无兼容问题后可启用 HSTS。

已有 PEM 证书也可按官方[上传证书](https://1panel.cn/docs/v2/user_manual/websites/certificate_upload/)说明导入匹配的证书和私钥。

外部问卷图片和视频也应使用 HTTPS URL，否则可能被客户端环境或中间网络拦截。

## 7. 验证公网 API

在另一台机器执行：

```bash
curl --fail https://vrcdog.example.com/ping
curl -i -X POST https://vrcdog.example.com/api/admin/auth \
  -H 'Content-Type: application/json' \
  --data '{"password":"你的原始管理员密码"}'
```

正确密码应返回 HTTP `200` 和 `{"success":true}`，错误密码应返回 HTTP `401`。

然后打开 VRCDog：

1. 客户端模式填写 `https://vrcdog.example.com`，只填基础地址，不附加 `/ping` 或 `/api`。
2. 服务端模式选择“远程服务”，填写相同地址和原始管理员密码。
3. 后台应能查看用户、角色与权限、问卷管理。
4. 远程协助功能需要额外验证 WebSocket 建连；仅 `/ping` 成功不代表 WebSocket 配置正确。

## 8. 问卷功能配置

在“服务端控制台 -> 问卷管理”中：

1. 开启“客户端问卷”总开关。关闭后保留问卷和记录，但不提示也不阻断客户端。
2. 新建问卷，设置标题、说明以及“作为使用门禁”。
3. 添加单选、多选、填空、意见/长文本题。每题可独立设置“必答”和“答错阻断”。
4. “答错阻断”题必须配置正确答案；多选题必须与全部正确选项完全一致才通过，文本答案去除首尾空格且不区分大小写。
5. 图片或视频填写可公开访问的 HTTP/HTTPS URL。建议使用对象存储或独立静态站点，不建议使用会过期的临时签名 URL。
6. 保存草稿后发布。发布新问卷会提示用户；修改已发布问卷并保存会生成新版本；“重新发送”也会增加版本号并要求所有用户再次填写。
7. 删除问卷会同时删除其全部提交记录。

客户端登录并注册到服务端后才有可靠的 VRChat 用户 ID，因此问卷在“输入地址并完成 VRChat 登录后、进入产品界面前”出现。已登录客户端通过 15 秒心跳发现新版本。

## 9. 数据备份与恢复

所有用户、角色、封禁、问卷和提交记录都在：

```text
/opt/vrcdog-server/data/server-state.json
```

推荐在 1Panel“计划任务”创建每日 Shell 任务。为了得到严格一致的快照，可短暂停服：

```bash
cd /opt/vrcdog-server
docker compose stop vrcdog-server
tar -C /opt/vrcdog-server -czf /opt/backups/vrcdog-server-$(date +%F-%H%M%S).tar.gz data
docker compose start vrcdog-server
```

备份目录应配置保留周期并同步到异机或对象存储。恢复前先停止容器，备份当前 `data`，再解压目标备份并确认目录属主为 `10001:10001`。

## 10. 升级、回滚和改密

升级前先备份 `data`，然后替换源码并执行：

```bash
cd /opt/vrcdog-server
docker compose build --pull
docker compose up -d
docker compose ps
curl --fail http://127.0.0.1:21451/ping
```

构建失败不会删除现有数据。发布前应保留上一版源码或镜像标签，以便重新构建/启动旧版本。

轮换管理员密码时，生成新 BCrypt 哈希，更新 `.env` 后执行：

```bash
cd /opt/vrcdog-server
docker compose up -d --force-recreate vrcdog-server
```

## 11. 生产安全边界

- 必须使用 HTTPS，且不要公网开放 `11451` 或 `21451`。
- 不要把 `.env`、`server-state.json`、备份文件提交到 Git 或放在网站静态目录。
- 当前管理员认证是“每次请求验证 BCrypt 密码”，没有登录会话、MFA、速率限制或审计日志；建议在 1Panel/OpenResty 侧增加来源 IP 白名单和请求限速。
- 客户端身份目前由客户端提交的 VRChat `user_id` 声明，服务端未向 VRChat 验证身份签名，不应把它当成支付、实名或高价值授权系统。
- 当前状态存储是单机 JSON 文件，不支持多个服务实例同时写入；不要用负载均衡启动多个副本。
