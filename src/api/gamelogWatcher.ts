import { GamelogApi, DbApi, SysApi } from './index';

let watcherTimer: number | null = null;
let isWatching = false;
let pollInFlight = false;

export async function initGamelogWatcher() {
  if (isWatching) return;
  isWatching = true;
  console.log('[LogWatcher] Started monitoring VRChat output_log.txt');

  // Initial poll
  await pollGamelogOnce();

  // A shorter interval prevents a fast VRChat restart from rotating the log
  // before the previous file's final leave events have been drained.
  watcherTimer = setInterval(pollGamelogOnce, 3000) as unknown as number;
}

export function stopGamelogWatcher() {
  if (watcherTimer) {
    clearInterval(watcherTimer);
    watcherTimer = null;
  }
  isWatching = false;
  console.log('[LogWatcher] Stopped');
}

export async function pollGamelogOnce() {
  if (pollInFlight) return;
  pollInFlight = true;
  try {
    let vrcRunning: boolean | null = null;
    try {
      vrcRunning = await SysApi.isVrcRunning();
    } catch {
      // A process-status error must not be treated as a confirmed game exit.
    }

    const logs = await GamelogApi.getLatestGamelogs({
      maxLines: 100000,
      finalizeSession: vrcRunning === false,
    });

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
  } finally {
    pollInFlight = false;
  }
}
