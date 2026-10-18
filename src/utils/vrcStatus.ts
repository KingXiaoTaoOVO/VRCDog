/**
 * VRChat 用户状态 → 侧边栏文案的 i18n key。
 *
 * 原来两个布局（PcLayout / VrLayout）直接写：
 *   $t('status.' + (currentUser?.status?.replace(' ', '_') || 'offline'))
 * 有两个问题：
 * 1. `String.replace` 传入**字符串**模式时只替换第一个匹配，
 *    所以 "do not disturb" 会变成 "do_not disturb"，拼出一个必然不存在的 key；
 * 2. 语言包里也没有 `status.do_not disturb`，而 vue-i18n 对缺失 key 返回 key 本身 ——
 *    于是侧边栏把 "status.do_not disturb" 这样的原始字符串直接显示给用户。
 *
 * 这里集中做三件事：小写归一、把**所有**空白转成下划线、未知状态回退到一定存在的 key。
 */

/** VRChat 实际会下发的状态取值（与 PcLayout.getStatusColor 用的集合保持一致）。 */
export const VRC_STATUSES = [
  'active',
  'online',
  'offline',
  'join me',
  'ask me',
  'busy',
  'do not disturb',
] as const;

/** 语言包里确实存在对应条目的 status.* 后缀。 */
const TRANSLATED_STATUS_KEYS = new Set([
  'active',
  'online',
  'offline',
  'join_me',
  'ask_me',
  'busy',
  'do_not_disturb',
]);

export function vrcStatusLabelKey(status?: string | null): string {
  const normalized = (status || 'offline').trim().toLowerCase().replace(/\s+/g, '_');
  // 未知/将来新增的状态一律落到 offline：宁可文案不够精确，也绝不能把裸 key 渲染到界面上。
  return `status.${TRANSLATED_STATUS_KEYS.has(normalized) ? normalized : 'offline'}`;
}
