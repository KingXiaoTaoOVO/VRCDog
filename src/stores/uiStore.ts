import { defineStore } from 'pinia';
import { ref, computed, markRaw } from 'vue';
import { VrcApi, DbApi } from '../api';
import { currentTheme, setTheme, themes, type ThemeId } from '../theme';
import { Settings, RefreshCcw, Bone, X, Heart, Users, Wrench, Flame, StickyNote, Sparkles, Download, LogOut, Loader2, Search, Globe, Bell, UserCircle, ScrollText, UsersRound, LayoutDashboard, Rss, Image, ShieldAlert, Activity, Network, BarChart3, History, MapPinned, Languages, ScanEye, Monitor, Glasses, Palette, Radio } from "lucide-vue-next";

export const useUiStore = defineStore('ui', () => {
  const appMode = ref<'pc' | 'vr' | null>(null);
  const activeTab = ref<string>('dashboard');
  const showCustomNavModal = ref(false);
  const showVrcxMenu = ref(false);
  const customNavConfig = ref<any[]>([]);

  const serverMenuPerms = ref<Record<string, boolean>>({});
  const serverThemePerms = ref<Record<string, boolean>>({});
  const serverModePerms = ref<Record<string, boolean>>({});

  const vrcServerStatus = ref<string>('');
  const vrcServerOk = ref(true);

  const modeSelectionError = ref('');

  const sidebarTabs = [
    { key: 'dashboard', label: 'sidebar.dashboard', icon: markRaw(LayoutDashboard) },
    { key: 'feed', label: 'sidebar.feed', icon: markRaw(Rss) },
    { key: 'locations', label: 'sidebar.locations', icon: markRaw(MapPinned) },
    { key: 'charts', label: 'sidebar.charts', icon: markRaw(BarChart3) },
    { key: 'playerlist', label: 'sidebar.playerlist', icon: markRaw(Network) },
    { key: 'gallery', label: 'sidebar.gallery', icon: markRaw(Image) },
    { key: 'social', label: 'sidebar.social', icon: markRaw(Users) },
    { key: 'friendslist', label: 'sidebar.friendslist', icon: markRaw(ScrollText) },
    { key: 'moderation', label: 'sidebar.moderation', icon: markRaw(ShieldAlert) },
    { key: 'search', label: 'sidebar.search', icon: markRaw(Search) },
    { key: 'notifications', label: 'sidebar.notifications', icon: markRaw(Bell) },
    { key: 'groups', label: 'sidebar.groups', icon: markRaw(UsersRound) },
    { key: 'avatars', label: 'sidebar.avatars', icon: markRaw(UserCircle) },
    { key: 'favorites', label: 'sidebar.favorites', icon: markRaw(Heart) },
    { key: 'heatmap', label: 'sidebar.heatmap', icon: markRaw(Flame) },
    { key: 'notes', label: 'sidebar.notes', icon: markRaw(StickyNote) },
    { key: 'presets', label: 'sidebar.presets', icon: markRaw(Sparkles) },
    { key: 'tools', label: 'sidebar.tools', icon: markRaw(Wrench) },
    { key: 'bilidown', label: 'sidebar.bilidown', icon: markRaw(Download) },
    { key: 'danmaku', label: 'sidebar.danmaku', icon: markRaw(Radio) },
    { key: 'translator', label: 'sidebar.translator', icon: markRaw(Languages) },
    { key: 'ovr', label: 'sidebar.ovr', icon: markRaw(ScanEye) },
    { key: 'remote', label: 'sidebar.remote', icon: markRaw(Radio) },
    { key: 'env', label: 'sidebar.env', icon: markRaw(Wrench) },
    { key: 'export', label: 'sidebar.export', icon: markRaw(Download) },
    { key: 'settings', label: 'sidebar.settings', icon: markRaw(Settings) },
  ];

  const activeSidebarTabs = computed(() => {
    const baseTabs = sidebarTabs.filter(tab => {
       if (Object.keys(serverMenuPerms.value).length > 0) {
          if (serverMenuPerms.value[tab.key] === false) return false;
       }
       return true;
    });
    if (customNavConfig.value.length === 0) return baseTabs;
    
    const result: any[] = [];
    customNavConfig.value.forEach((cfg: any) => {
        if (cfg.visible !== false) {
           const t = baseTabs.find(b => b.key === cfg.key);
           if (t) result.push(t);
        }
    });
    baseTabs.forEach(t => {
       if (!customNavConfig.value.find((c: any) => c.key === t.key)) result.push(t);
    });
    return result;
  });

  const editableNavConfig = computed(() => {
    const baseTabs = sidebarTabs.filter(tab => {
       if (Object.keys(serverMenuPerms.value).length > 0) {
          if (serverMenuPerms.value[tab.key] === false) return false;
       }
       return true;
    });
    if (customNavConfig.value.length === 0) return baseTabs.map(t => ({ ...t, visible: true }));
    
    const result: any[] = [];
    customNavConfig.value.forEach((cfg: any) => {
        const t = baseTabs.find(b => b.key === cfg.key);
        if (t) {
           result.push({ ...t, visible: cfg.visible !== false });
        }
    });
    baseTabs.forEach(t => {
       if (!customNavConfig.value.find((c: any) => c.key === t.key)) {
           result.push({ ...t, visible: true });
       }
    });
    return result;
  });

  const filteredThemes = computed(() => {
    if (Object.keys(serverThemePerms.value).length === 0) return themes;
    const result: typeof themes = {} as typeof themes;
    for (const [key, theme] of Object.entries(themes)) {
      if (serverThemePerms.value[key] !== false) (result as any)[key] = theme;
    }
    return result;
  });

  const loadCustomNavConfig = async () => {
    try {
       const str = await DbApi.getSetting({ key: 'custom_nav_config' });
       if (str) {
          customNavConfig.value = JSON.parse(str);
       }
    } catch(e) {}
  };

  const saveCustomNavConfig = async (newConfig: any[] | null) => {
     showCustomNavModal.value = false;
     if (newConfig === null) {
         customNavConfig.value = [];
         await DbApi.saveSetting({ key: 'custom_nav_config', value: '[]' });
         return;
     }
     customNavConfig.value = newConfig.map((n: any) => ({ key: n.key, visible: n.visible }));
     await DbApi.saveSetting({ key: 'custom_nav_config', value: JSON.stringify(customNavConfig.value) });
  };

  const fetchServerFeatures = async (baseUrl: string, user: any) => {
    if (!baseUrl || !user) return;
    const uid = user.id || user.displayName;
    try {
      const data = await VrcApi.request(`${baseUrl}/api/client/features/${uid}`, { method: 'GET' });
      serverMenuPerms.value = data.menus || {};
      serverThemePerms.value = data.themes || {};
      serverModePerms.value = data.modes || {};
      if (serverThemePerms.value[currentTheme.value.id] === false) {
        const available = Object.keys(serverThemePerms.value).find(k => serverThemePerms.value[k] !== false);
        if (available) setTheme(available as ThemeId);
      }
    } catch { /* ignore */ }
  };

  const fetchServerStatus = async () => {
    try {
      const res = await VrcApi.getServerStatus();
      if (res?.status?.description === 'All Systems Operational') {
        vrcServerOk.value = true;
        vrcServerStatus.value = '';
      } else {
        vrcServerOk.value = false;
        vrcServerStatus.value = res?.status?.description || 'Unknown';
      }
    } catch { vrcServerOk.value = true; }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'active': return 'bg-primary';
      case 'join me': return 'bg-primary';
      case 'ask me': return 'bg-orange-500';
      case 'busy': return 'bg-red-500';
      default: return 'bg-surface';
    }
  };

  return {
    appMode,
    activeTab,
    showCustomNavModal,
    showVrcxMenu,
    customNavConfig,
    serverMenuPerms,
    serverThemePerms,
    serverModePerms,
    vrcServerStatus,
    vrcServerOk,
    modeSelectionError,
    sidebarTabs,
    activeSidebarTabs,
    editableNavConfig,
    filteredThemes,
    loadCustomNavConfig,
    saveCustomNavConfig,
    fetchServerFeatures,
    fetchServerStatus,
    getStatusColor
  };
});
