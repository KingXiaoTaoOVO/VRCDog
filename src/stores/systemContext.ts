import { defineStore } from 'pinia';
import { ref } from 'vue';
import { SysApi, DbApi } from '../api';

export const useSystemContextStore = defineStore('systemContext', () => {
  const isVrcRunning = ref(false);
  const isSteamVrRunning = ref(false);
  const isAfk = ref(false); // Can be toggled if we detect user inactivity or VRC status
  
  let pollingInterval: any = null;

  const startPolling = () => {
    if (pollingInterval) return;
    pollingInterval = setInterval(async () => {
      try {
        const wasRunning = isVrcRunning.value;
        const nowRunning = await SysApi.isVrcRunning();
        isVrcRunning.value = nowRunning;
        
        if (!wasRunning && nowRunning) {
           const settings: any = await DbApi.getAllSettings();
           if (settings && settings.autoLaunchApps) {
               try {
                   const apps = typeof settings.autoLaunchApps === 'string' 
                        ? JSON.parse(settings.autoLaunchApps) 
                        : settings.autoLaunchApps;
                   if (Array.isArray(apps) && apps.length > 0) {
                       await SysApi.startAutoLaunchApps({ apps });
                       console.log("[AutoLaunch] Launched apps.");
                   }
               } catch(e) {}
           }
        } else if (wasRunning && !nowRunning) {
           const settings: any = await DbApi.getAllSettings();
           if (settings && (settings.killAppsOnExit === true || settings.killAppsOnExit === 'true')) {
               try { await SysApi.killAutoLaunchApps(); } catch(e) {}
               console.log("[AutoLaunch] Killed apps.");
           }
           
           if (settings && (settings.clearCacheOnExit === true || settings.clearCacheOnExit === 'true')) {
               try {
                   const cleared = await SysApi.clearVrcCache();
                   console.log(`[CacheCleaner] Auto-cleared ${cleared} bytes of VRChat cache on exit.`);
               } catch(e) {}
           }
           
           if (settings && (settings.autoStart === true || settings.autoStart === 'true')) {
               console.log("[AutoStart] VRChat stopped unexpectedly. Restarting in 5s...");
               setTimeout(async () => {
                   try { await SysApi.launchVrc({}); } catch (err) {}
               }, 5000);
           }
        }
      } catch (e) { /* ignore */ }
      
      try {
        isSteamVrRunning.value = await SysApi.checkSteamVR();
      } catch (e) { /* ignore */ }
    }, 5000); // Poll every 5 seconds
  };

  const stopPolling = () => {
    if (pollingInterval) {
      clearInterval(pollingInterval);
      pollingInterval = null;
    }
  };

  const setAfk = (afk: boolean) => {
    isAfk.value = afk;
  };

  return {
    isVrcRunning,
    isSteamVrRunning,
    isAfk,
    startPolling,
    stopPolling,
    setAfk
  };
});
