import { useSystemContextStore } from './systemContext';
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import { DbApi } from '../api';

export type NotificationCondition = 'never' | 'desktop' | 'vr' | 'not_vr' | 'vrc_running' | 'vrc_not_running' | 'always';
export type NotificationKind = 'friend_online' | 'friend_offline' | 'friend_location' | 'invite' | 'friend_request' | 'group' | 'test' | 'other';

export interface NotificationRules {
  desktopCondition: NotificationCondition;
  showWhenAfk: boolean;
  soundEnabled: boolean;
  ttsCondition: NotificationCondition;
  ttsVoice: string;
  ttsVolume: number;
  notifyFriendsOnline: boolean;
  notifyInvite: boolean;
  notifyStatusChange: boolean;
}

export function isNotificationKindEnabled(settings: Record<string, unknown>, kind: NotificationKind): boolean {
  if (kind === 'test') return true;
  const asBool = (value: unknown, fallback: boolean) => {
    if (typeof value === 'boolean') return value;
    if (typeof value === 'string') return value.trim().toLowerCase() === 'true';
    return fallback;
  };
  if (kind === 'friend_online' || kind === 'friend_offline') return asBool(settings.notifyFriendsOnline, true);
  if (kind === 'friend_location') return asBool(settings.notifyStatusChange, false);
  if (kind === 'invite' || kind === 'friend_request' || kind === 'group') return asBool(settings.notifyInvite, true);
  return true;
}

export const useNotificationEngine = () => {
  const systemStore = useSystemContextStore();

  const asBool = (value: unknown, fallback = false) => {
    if (typeof value === 'boolean') return value;
    if (typeof value === 'string') {
      const normalized = value.trim().toLowerCase();
      if (normalized === 'true') return true;
      if (normalized === 'false') return false;
    }
    return fallback;
  };

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
    const all = await DbApi.getAllSettings() as Record<string, unknown>;
    const systemEnabled = asBool(all.notifySystem, true);
    return {
      desktopCondition: systemEnabled ? ((all.notifyDesktopCondition as NotificationCondition) || 'always') : 'never',
      showWhenAfk: asBool(all.notifyShowWhenAfk, false),
      soundEnabled: asBool(all.notifySound, true),
      ttsCondition: asBool(all.notifyTts, false)
        ? ((all.notifyTtsCondition as NotificationCondition) || 'always')
        : 'never',
      ttsVoice: (all.notifyTtsVoice as string) || '',
      ttsVolume: Number(all.notifyTtsVolume) || 50,
      notifyFriendsOnline: asBool(all.notifyFriendsOnline, true),
      notifyInvite: asBool(all.notifyInvite, true),
      notifyStatusChange: asBool(all.notifyStatusChange, false),
    };
  };

  const notify = async (title: string, body: string, type: NotificationKind = 'test') => {
    const rules = await getRules();
    const categoryEnabled = isNotificationKindEnabled(rules as unknown as Record<string, unknown>, type);
    if (!categoryEnabled) return { desktop: 'disabled' as const, sound: false, tts: false };

    const desktopEnabled = evaluateCondition(rules.desktopCondition, rules.showWhenAfk);
    let desktop: 'sent' | 'disabled' | 'denied' | 'unavailable' = 'disabled';

    // Evaluate Desktop Notification
    if (desktopEnabled) {
      try {
        let permissionGranted = await isPermissionGranted();
        if (!permissionGranted) {
          const permission = await requestPermission();
          permissionGranted = permission === 'granted';
        }
        if (permissionGranted) {
          await sendNotification({ title, body });
          desktop = 'sent';
        } else {
          desktop = 'denied';
        }
      } catch (err) {
        // 通知插件未注册或不可用（旧的构建/未授权）时静默忽略，
        // 否则会在 console 刷屏并影响调试。
        console.debug('[notify] desktop notification unavailable:', err);
        desktop = 'unavailable';
      }
    }

    const sound = desktopEnabled && rules.soundEnabled;
    if (sound) {
      playNotificationSound();
    }

    // Evaluate TTS
    const tts = evaluateCondition(rules.ttsCondition, rules.showWhenAfk);
    if (tts) {
      playTts(body, rules.ttsVoice, rules.ttsVolume);
    }
    return { desktop, sound, tts };
  };

  const playNotificationSound = () => {
    try {
      const AudioCtor = window.AudioContext || (window as any).webkitAudioContext;
      if (!AudioCtor) return;
      const context = new AudioCtor();
      const oscillator = context.createOscillator();
      const gain = context.createGain();
      const start = context.currentTime;
      oscillator.type = 'sine';
      oscillator.frequency.setValueAtTime(880, start);
      oscillator.frequency.exponentialRampToValueAtTime(1320, start + 0.08);
      gain.gain.setValueAtTime(0.0001, start);
      gain.gain.exponentialRampToValueAtTime(0.12, start + 0.01);
      gain.gain.exponentialRampToValueAtTime(0.0001, start + 0.22);
      oscillator.connect(gain).connect(context.destination);
      oscillator.start(start);
      oscillator.stop(start + 0.24);
      window.setTimeout(() => void context.close(), 450);
    } catch {
      // Ignore browsers that block audio before user interaction.
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

  return { notify, playTts, playNotificationSound };
};
