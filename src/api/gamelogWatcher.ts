import { GamelogApi, DbApi, SysApi } from './index';

let watcherTimer: number | null = null;
let isWatching = false;

export async function initGamelogWatcher() {
  if (isWatching) return;
  isWatching = true;
  console.log('[LogWatcher] Started monitoring VRChat output_log.txt');

  // Initial poll
  await pollGamelog();

  // Poll every 10 seconds
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
    // Skip polling if VRChat is not running
    const vrcRunning = await SysApi.isVrcRunning().catch(() => false);
    if (!vrcRunning) return;

    const logs = await GamelogApi.getLatestGamelogs({ maxLines: 500 });

    if (!logs || logs.length === 0) return;

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
        console.warn('[LogWatcher] Failed to update dynamic discord RPC', err);
      }
    }
  } catch (err) {
    console.warn('[LogWatcher] Error reading logs:', err);
  }
}
