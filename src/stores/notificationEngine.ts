import { useSystemContextStore } from './systemContext';
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import { DbApi } from '../api';

export type NotificationCondition = 'never' | 'desktop' | 'vr' | 'not_vr' | 'vrc_running' | 'vrc_not_running' | 'always';

export interface NotificationRules {
  desktopCondition: NotificationCondition;
  showWhenAfk: boolean;
  ttsCondition: NotificationCondition;
  ttsVoice: string;
  ttsVolume: number;
}

export const useNotificationEngine = () => {
  const systemStore = useSystemContextStore();

  const evaluateCondition = (condition: NotificationCondition, isAfkEnabled: boolean): boolean => {
    if (condition === 'never') return false;
    if (condition === 'always') return true;

    // Check AFK override
    if (systemStore.isAfk && !isAfkEnabled) return false;

    if (condition === 'desktop') return !systemStore.isSteamVrRunning;
    if (condition === 'vr') return systemStore.isSteamVrRunning;
    if (condition === 'not_vr') return !systemStore.isSteamVrRunning;
    if (condition === 'vrc_running') return systemStore.isVrcRunning;
    if (condition === 'vrc_not_running') return !systemStore.isVrcRunning;

    return false;
  };

  const getRules = async (): Promise<NotificationRules> => {
    // Fetch rules from DB. Fallback to defaults.
    const all = await DbApi.getAllSettings() as Record<string, any>;
    return {
      desktopCondition: all.notifyDesktopCondition || 'always',
      showWhenAfk: all.notifyShowWhenAfk !== false,
      ttsCondition: all.notifyTtsCondition || 'never',
      ttsVoice: all.notifyTtsVoice || '',
      ttsVolume: Number(all.notifyTtsVolume) || 50
    };
  };

  const notify = async (title: string, body: string, type: 'friend_online' | 'invite' | 'test' = 'test') => {
    const rules = await getRules();
    
    // Evaluate Desktop Notification
    if (evaluateCondition(rules.desktopCondition, rules.showWhenAfk)) {
      let permissionGranted = await isPermissionGranted();
      if (!permissionGranted) {
        const permission = await requestPermission();
        permissionGranted = permission === 'granted';
      }
      if (permissionGranted) {
        sendNotification({ title, body });
      }
    }

    // Evaluate TTS
    if (evaluateCondition(rules.ttsCondition, rules.showWhenAfk)) {
      playTts(body, rules.ttsVoice, rules.ttsVolume);
    }
  };

  const playTts = (text: string, voiceURI: string, volume: number) => {
    if (!('speechSynthesis' in window)) return;
    const utterance = new SpeechSynthesisUtterance(text);
    utterance.volume = volume / 100;
    
    if (voiceURI) {
      const voices = window.speechSynthesis.getVoices();
      const selectedVoice = voices.find(v => v.voiceURI === voiceURI);
      if (selectedVoice) {
        utterance.voice = selectedVoice;
      }
    }
    
    window.speechSynthesis.speak(utterance);
  };

  return { notify, playTts };
};
