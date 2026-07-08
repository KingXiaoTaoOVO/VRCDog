/**
 * 全局调试日志开关
 * 默认关闭——用户需在设置中手动开启 Debug Console 后才启用
 * 避免每次 API 调用都 dispatch 事件、序列化参数，浪费性能
 */
let _debugLogEnabled = false;

export function setDebugLogEnabled(v: boolean) {
  _debugLogEnabled = v;
}

export function isDebugLogEnabled(): boolean {
  return _debugLogEnabled;
}
