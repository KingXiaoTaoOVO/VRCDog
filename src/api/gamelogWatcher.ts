import { GamelogApi, DbApi, SysApi } from './index';

let watcherTimer: number | null = null;
let isWatching = false;
const lastLogCount = 0;

export async function initGamelogWatcher() {
  if (isWatching) return;
  isWatching = true;
  console.log('[LogWatcher] Started monitoring VRChat output_log.txt');
  
  // 首次运行
  await pollGamelog();
  
  // 每 10 秒轮询一次日志文件
  watcherTimer = setInterval(pollGamelog, 10000) as unknown as number;
}

export function stopGamelogWatcher() {
  if (watcherTimer) {
    clearInterval(watcherTimer);
    watcherTimer = null;
  }
  isWatching = false;
  console.log('[LogWatcher] Stopped');
}

async function pollGamelog() {
  try {
    // 读取最新的日志（倒序返回，最新的在前面）
    const logs = await GamelogApi.getLatestGamelogs({ maxLines: 500 });
    
    if (!logs || logs.length === 0) return;
    
    // 我们只需要把它们批量塞进数据库
    // 数据库使用 `INSERT ... WHERE NOT EXISTS` 来保证不重复插入
    const savedCount = await DbApi.saveGameLogs({ logsJson: JSON.stringify(logs) });
    
    if (savedCount > 0) {
      console.log(`[LogWatcher] Found ${savedCount} new game log events.`);
      window.dispatchEvent(new CustomEvent('vrc-gamelog-updated'));
      
      // Dynamic Discord RPC update
      try {
        const settings = await DbApi.getAllSettings();
        if (settings && settings.discordRpcEnabled === 'true' && settings.discordRpcEnableWorldIntegration === 'true') {
          const joinedEvent = logs.find((l: any) => l.event_type === 'Instance Joined');
          if (joinedEvent) {
             const worldName = joinedEvent.content;
             await SysApi.setDiscordRpc({
               details: settings.discordRpcShowWorldName === 'true' ? `Playing in ${worldName}` : 'In VRChat',
               state: 'Active'
             });
          }
        }
      } catch (err) {
        console.warn('Failed to update dynamic discord RPC', err);
      }
    }
  } catch (err) {
    console.warn('[LogWatcher] Error reading logs:', err);
  }
}
