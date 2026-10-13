import { GamelogApi, DbApi, SysApi } from './index';
import { refreshCurrentInstance, currentInstanceState, parseVrcLocation } from '../stores/currentInstance';
import { isTauri } from '@tauri-apps/api/core';

let watcherTimer: number | null = null;
let isWatching = false;
let pollInFlight = false;

export async function initGamelogWatcher() {
  if (!isTauri()) return;
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

    // Clear stale room state immediately even if the final log read fails.
    if (vrcRunning === false) {
      await refreshCurrentInstance({ vrcRunning: false });
    }

    const logs = await GamelogApi.getLatestGamelogs({
      maxLines: 100000,
      finalizeSession: vrcRunning === false,
    });

    const newLogs = Array.isArray(logs) ? logs : [];
    const savedCount = newLogs.length > 0
      ? await DbApi.saveGameLogs({ logsJson: JSON.stringify(newLogs) })
      : 0;

    await refreshCurrentInstance({
      vrcRunning,
      force: savedCount > 0,
    });

    if (savedCount > 0) {
      console.log(`[LogWatcher] Found ${savedCount} new game log events.`);
      window.dispatchEvent(new CustomEvent('vrc-gamelog-updated'));

      // Dynamic Discord RPC update
      try {
        const settings = await DbApi.getAllSettings();
        if (settings && (settings.discordRpcEnabled === 'true' || settings.discordRpcEnabled === true)) {
          if (settings.discordRpcEnableWorldIntegration === 'true' || settings.discordRpcEnableWorldIntegration === true) {
            const joinedEvent = newLogs.find((l: any) => l.event_type === 'Instance Joined');
            const roomName = currentInstanceState.roomName || joinedEvent?.content;
            if (roomName) {
              const showWorldName = settings.discordRpcShowWorldName === 'true' || settings.discordRpcShowWorldName === true;
              const details = showWorldName ? `Playing in ${roomName}` : (settings.discordRpcDetails || 'In VRChat');

              const showRoomTypeAndCount = settings.discordRpcShowRoomTypeAndCount === 'true' || settings.discordRpcShowRoomTypeAndCount === true;
              let state = settings.discordRpcState || 'Active';
              let partySize: [number, number] | undefined = undefined;

              if (showRoomTypeAndCount && currentInstanceState.playerCount) {
                const count = currentInstanceState.playerCount;
                state = `${count} ${count === 1 ? 'player' : 'players'}`;
                partySize = [currentInstanceState.players?.length || count, Math.max(count, currentInstanceState.players?.length || 1)];
              }

              const showPlatform = settings.discordRpcShowPlatform === 'true' || settings.discordRpcShowPlatform === true;
              const showThumbnail = settings.discordRpcShowWorldThumbnail === 'true' || settings.discordRpcShowWorldThumbnail === true;
              const showJoinButton = settings.discordRpcShowJoinButton === 'true' || settings.discordRpcShowJoinButton === true;

              let joinUrl: string | undefined = undefined;
              if (showJoinButton && currentInstanceState.location) {
                const parsed = parseVrcLocation(currentInstanceState.location);
                if (parsed) {
                  joinUrl = `https://vrchat.com/home/launch?worldId=${parsed.worldId}&instanceId=${parsed.instanceId}`;
                }
              }

              await SysApi.setDiscordRpc({
                details,
                state,
                showWorldThumbnail: showThumbnail,
                showJoinButton,
                smallImage: showPlatform ? 'pc' : undefined,
                smallText: showPlatform ? 'VRChat (PC)' : undefined,
                partySize,
                joinUrl,
              });
            }
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
