/**
 * VRCDog 自托管服务端（vrcdog-server）的 client token 持有者。
 *
 * 背景：服务端 `a77f8aa` 起给 `/api/client/*` 挂了 `require_client_token` 中间件，
 * 只认 `x-vrcdog-client-token` 头。但当时只有心跳和 disconnect 两处补上了这个头，
 * `/api/client/features/{id}`、`/api/client/surveys/*`、`/api/client/survey-history/*`、
 * `/api/client/check-status/{id}`、`/api/client/uploads/{id}` 全都没有带 —— 而这些调用点
 * 又都把错误吞掉了（空 catch），于是服务端问卷系统和服务端菜单/主题权限
 * **长期稳定地 401 并静默失败**：问卷中心永远显示「暂无问卷」，管理员配置的权限过滤形同虚设。
 *
 * 这里把 token 收敛到一处，由 `request.ts` 在发出请求时自动附加，避免再出现
 * 「新增一个客户端接口 → 忘记带头」的情况。
 *
 * 安全约束：**只发给注册时那台服务器的 origin**。token 是设备身份凭证，
 * 绝不能被顺手带到任意 `allowExternalHost` 的目标（比如仪表盘里手填的别的地址）。
 */

let clientToken = '';
let serverOrigin = '';

function originOf(url: string): string | null {
  try {
    return new URL(url).origin;
  } catch {
    return null;
  }
}

/**
 * 由 authStore 在注册成功 / 切换服务器 / 登出时调用。
 * `serverUrl` 传空或 token 传空都会把状态清干净（登出后不得再携带旧身份）。
 */
export function setVrcdogClientToken(token: string, serverUrl: string): void {
  clientToken = typeof token === 'string' ? token : '';
  serverOrigin = serverUrl ? (originOf(serverUrl) ?? '') : '';
  if (!clientToken) serverOrigin = '';
}

/** 命中已钉死的服务器 origin 时返回鉴权头，否则返回空对象。 */
export function vrcdogClientTokenHeaders(url: string): Record<string, string> {
  if (!clientToken || !serverOrigin) return {};
  const target = originOf(url);
  if (!target || target !== serverOrigin) return {};
  return { 'x-vrcdog-client-token': clientToken };
}

/** 仅供测试与调试：查看当前是否已绑定 token 与服务器。 */
export function vrcdogClientTokenState(): { hasToken: boolean; origin: string } {
  return { hasToken: Boolean(clientToken), origin: serverOrigin };
}
