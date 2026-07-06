<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted, watch } from "vue";
import { useI18n } from "vue-i18n";
import { X, MoreHorizontal, Star, Copy, RefreshCcw, Share2, ExternalLink, ShieldBan, UserMinus, VolumeX, MessageSquareOff, Eye, EyeOff, User, Users, UsersRound, Globe, Map, Cuboid, History, Code, Info, LogIn, Mail, Hand, Download, ZoomIn, ZoomOut, RotateCw, RotateCcw, Shield, Monitor, Smartphone, Flag, Check, MapPin, Clock, Calendar, AlignLeft, PencilLine, Save, ChevronDown, Languages, Loader2, Trash2 } from "lucide-vue-next";
import { useUserProfileStore } from "../stores/userProfile";
import { useAuthStore } from "../stores/authStore";
import { useToast } from "../composables/useToast";
import { VrcApi, DbApi, OvrApi } from "../api";
import VrcAvatar from "./VrcAvatar.vue";
import JsonTree from "./JsonTree.vue";

const { t, locale } = useI18n();
const profileStore = useUserProfileStore();
const authStore = useAuthStore();
const toast = useToast();

type TabId = "info" | "mutual" | "groups" | "created_worlds" | "fav_worlds" | "created_avatars" | "activity" | "raw_json";
const activeTab = ref<TabId>("info");
const isSelf = computed(() => profileStore.baseInfo?.id === profileStore.myId);

const tabs = computed(() => {
  const list: { id: TabId; label: string }[] = [{ id: "info", label: t("user_profile.tabs.info") }];
  if (!isSelf.value) list.push({ id: "mutual", label: t("user_profile.tabs.mutual") });
  list.push(
    { id: "groups", label: t("user_profile.tabs.groups") },
    { id: "created_worlds", label: t("user_profile.tabs.worlds") },
    { id: "fav_worlds", label: t("user_profile.tabs.fav_worlds") },
    { id: "created_avatars", label: t("user_profile.tabs.avatars") },
    { id: "activity", label: t("user_profile.tabs.activity") },
    { id: "raw_json", label: t("user_profile.tabs.raw") }
  );
  return list;
});

const showMoreMenu = ref(false);
const showFavPicker = ref(false);
const openedBadgeId = ref<string | null>(null);

function openBadgePopover(badgeId: string) {
  openedBadgeId.value = openedBadgeId.value === badgeId ? null : badgeId;
}

async function toggleBadgeHidden(badge: any, hidden: boolean) {
  const userId = profileStore.baseInfo?.id;
  if (!userId || !badge?.badgeId) return;
  const previousHidden = badge.hidden;
  const previousShowcased = badge.showcased;
  badge.hidden = hidden;
  if (hidden) badge.showcased = false;
  try {
    await VrcApi.updateBadge({
      userId,
      badgeId: badge.badgeId,
      hidden: badge.hidden,
      showcased: badge.showcased,
    });
    toast.success(t('user_profile.badges.sync_success'));
  } catch (e: any) {
    badge.hidden = previousHidden;
    badge.showcased = previousShowcased;
    toast.error(t('user_profile.badges.sync_failed', { error: e?.message || e }));
  }
}

async function toggleBadgeShowcased(badge: any, showcased: boolean) {
  const userId = profileStore.baseInfo?.id;
  if (!userId || !badge?.badgeId) return;
  const previousHidden = badge.hidden;
  const previousShowcased = badge.showcased;
  badge.showcased = showcased;
  if (showcased) badge.hidden = false;
  try {
    await VrcApi.updateBadge({
      userId,
      badgeId: badge.badgeId,
      hidden: badge.hidden,
      showcased: badge.showcased,
    });
    toast.success(t('user_profile.badges.sync_success'));
  } catch (e: any) {
    badge.hidden = previousHidden;
    badge.showcased = previousShowcased;
    toast.error(t('user_profile.badges.sync_failed', { error: e?.message || e }));
  }
}
const favGroups = ref<any[]>([]);

// ── 导航面包屑 (Navigation Breadcrumb) ──────────────────────────
interface BreadcrumbItem {
  userId: string;
  displayName: string;
}
const navHistory = ref<BreadcrumbItem[]>([]);

// 打开好友的好友（带面包屑导航）
const navigateToUser = (userId: string, displayName: string, prefillData?: any) => {
  // 保存当前用户到历史
  if (profileStore.baseInfo?.id) {
    navHistory.value.push({
      userId: profileStore.baseInfo.id,
      displayName: profileStore.baseInfo.displayName || t('user_profile.info.unknown'),
    });
  }
  profileStore.openProfile(userId, prefillData || null);
};

// 返回上一级
const navigateBack = () => {
  const prev = navHistory.value.pop();
  if (prev) {
    profileStore.openProfile(prev.userId, null);
  }
};

// 跳转到面包屑中的某一级
const navigateTo = (index: number) => {
  const target = navHistory.value[index];
  if (target) {
    navHistory.value = navHistory.value.slice(0, index);
    profileStore.openProfile(target.userId, null);
  }
};

// 关闭时清空导航历史
watch(() => profileStore.isOpen, v => {
  if (!v) navHistory.value = [];
});

// ── 好友序号 (Friend Number) ────────────────────────────────────
const friendNumber = ref(0);
const fetchFriendNumber = async () => {
  friendNumber.value = 0;
  if (!profileStore.baseInfo?.isFriend || !profileStore.targetUserId) return;
  try {
    const friends = await DbApi.getCachedFriends();
    if (Array.isArray(friends)) {
      const idx = friends.findIndex((f: any) => f.id === profileStore.targetUserId);
      if (idx >= 0) friendNumber.value = idx + 1;
    }
  } catch { /* ignore */ }
};

// Load real favorite groups from VRChat API
const loadFavGroups = async () => {
  try {
    const res: any = await VrcApi.getFavoriteGroups();
    if (Array.isArray(res)) {
      favGroups.value = res
        .filter((g: any) => g.type === 'friend')
        .map((g: any) => ({ id: g.name, name: g.displayName || g.name, count: g.count || 0, max: g.capacity || 150 }));
    }
  } catch { /* ignore */ }
};

const isEditingNote = ref(false);
const localNote = ref("");
watch(() => profileStore.localNote, v => { localNote.value = v; }, { immediate: true });
const saveNote = async () => {
  profileStore.localNote = localNote.value;
  await profileStore.saveLocalNote();
  isEditingNote.value = false;
  toast.success(t("user_profile.info.memo_saved"));
};

const showImagePreview = ref(false);
const imageScale = ref(1);
const imageRotation = ref(0);
const previewImageUrl = ref<string>('');
const toggleImagePreview = () => {
  showImagePreview.value = !showImagePreview.value;
  imageScale.value = 1;
  imageRotation.value = 0;
  if (showImagePreview.value) previewImageUrl.value = avatarImageUrl.value;
  else previewImageUrl.value = '';
};
const previewUserIcon = () => {
  const url = (profileStore.baseInfo as any)?.userIcon;
  if (!url) return;
  previewImageUrl.value = url;
  imageScale.value = 1;
  imageRotation.value = 0;
  showImagePreview.value = true;
};
const handleZoomIn = () => { imageScale.value = Math.min(imageScale.value + 0.25, 4); };
const handleZoomOut = () => { imageScale.value = Math.max(imageScale.value - 0.25, 0.5); };
const handleRotateCw = () => { imageRotation.value += 90; };
const handleResetImage = () => { imageScale.value = 1; imageRotation.value = 0; };
const handleDownloadImage = () => {
  const url = avatarImageUrl.value;
  if (!url) return;
  const a = document.createElement("a");
  a.href = url; a.download = `user_${profileStore.baseInfo?.id}.png`; a.click();
};

const copyRawJson = () => {
  const json = JSON.stringify(profileStore.baseInfo, null, 2);
  navigator.clipboard.writeText(json);
  toast.success(t('user_profile.messages.json_copied'));
};

// ========== Activity Tab Logic ==========
const activityPeriod = ref('30');
const excludeHomeWorld = ref(false);
const worldSortBy = ref('count');
const heatmapData = ref<number[][]>(Array.from({ length: 7 }, () => Array(24).fill(0)));
const dailyPlaytimeData = ref<number[]>([]);
const dailyPlaytimeLabels = ref<string[]>([]);
const topWorldsRaw = ref<{ id: string; name: string; imageUrl: string; visits: number; totalTime: number }[]>([]);

const heatmapDays = computed(() => [
  t('user_profile.activity.day_monday'),
  t('user_profile.activity.day_tuesday'),
  t('user_profile.activity.day_wednesday'),
  t('user_profile.activity.day_thursday'),
  t('user_profile.activity.day_friday'),
  t('user_profile.activity.day_saturday'),
  t('user_profile.activity.day_sunday'),
]);

const activityEventCount = computed(() => profileStore.activityLogs.length);

const activityPeakDay = computed(() => {
  if (heatmapData.value.every(row => row.every(v => v === 0))) return '';
  const daySums = heatmapData.value.map(row => row.reduce((a, b) => a + b, 0));
  const maxIdx = daySums.indexOf(Math.max(...daySums));
  return heatmapDays.value[maxIdx];
});

const activityPeakTime = computed(() => {
  if (heatmapData.value.every(row => row.every(v => v === 0))) return '';
  const hourSums = Array(24).fill(0);
  for (const row of heatmapData.value) {
    row.forEach((v, h) => { hourSums[h] += v; });
  }
  const maxHour = hourSums.indexOf(Math.max(...hourSums));
  const endHour = Math.min(maxHour + 4, 24);
  return `${String(maxHour).padStart(2, '0')}:00-${String(endHour).padStart(2, '0')}:00`;
});

const avgDailyPlaytime = computed(() => {
  if (dailyPlaytimeData.value.length === 0) return '0h';
  const total = dailyPlaytimeData.value.reduce((a, b) => a + b, 0);
  const avg = total / dailyPlaytimeData.value.length;
  return `${avg.toFixed(1)}h`;
});

const topWorldsList = computed(() => {
  let list = [...topWorldsRaw.value];
  if (excludeHomeWorld.value) {
    // 排除 VRChat 内置主世界（VRChat Home / The Black Cat / 个人 home location）
    const homeId = (profileStore.baseInfo as any)?.homeLocation || '';
    list = list.filter(w => {
      const n = (w.name || '').toLowerCase();
      if (homeId && (w.id === homeId || w.name === homeId)) return false;
      if (n.includes('vrchat home')) return false;
      if (n.includes('home') && n.includes('vrchat')) return false;
      if (n === 'home' || n === '出生点') return false;
      return true;
    });
  }
  if (worldSortBy.value === 'count') {
    list.sort((a, b) => b.visits - a.visits);
  } else {
    list.sort((a, b) => b.totalTime - a.totalTime);
  }
  return list.slice(0, 10);
});

function getHeatmapColor(dayIdx: number, hour: number): string {
  const val = heatmapData.value[dayIdx]?.[hour] || 0;
  if (val === 0) return 'var(--theme-surface-hover)';
  const maxVal = Math.max(...heatmapData.value.flat(), 1);
  const intensity = Math.min(val / maxVal, 1);
  // Green gradient like VRCX
  const alpha = 0.2 + intensity * 0.8;
  return `rgba(34, 197, 94, ${alpha})`;
}

function getWorldBarWidth(visits: number): string {
  if (topWorldsList.value.length === 0) return '0%';
  const maxVisits = Math.max(...topWorldsList.value.map(w => w.visits), 1);
  return `${(visits / maxVisits) * 100}%`;
}

async function refreshActivityData() {
  if (!profileStore.targetUserId) return;
  profileStore.fetchActivityLogs(profileStore.targetUserId);

  const periodDays = parseInt(activityPeriod.value);
  const now = Date.now();
  const cutoff = now - periodDays * 24 * 60 * 60 * 1000;

  // ── 热力图：看自己用 db_get_heatmap（基于 friend_activity 表，VRCX 等价）
  //         看他人用 activityLogs 估算 ──
  try {
    const newHeatmap = Array.from({ length: 7 }, () => Array(24).fill(0));
    if (isSelf.value) {
      const buckets = await DbApi.getHeatmap();
      if (Array.isArray(buckets)) {
        for (const b of buckets) {
          const day = (b as any).day;
          const hour = (b as any).hour;
          const count = (b as any).count;
          if (day >= 0 && day < 7 && hour >= 0 && hour < 24) {
            newHeatmap[day][hour] = count || 0;
          }
        }
      }
    } else {
      const logs = profileStore.activityLogs;
      for (const log of logs) {
        const at = (log as any).created_at;
        if (!at) continue;
        const ts = new Date(at).getTime();
        if (ts < cutoff) continue;
        const date = new Date(at);
        const dayOfWeek = (date.getDay() + 6) % 7; // Monday = 0
        newHeatmap[dayOfWeek][date.getHours()] += 1;
      }
    }
    heatmapData.value = newHeatmap;
  } catch (e) {
    console.warn('Failed to build heatmap', e);
  }

  // ── 每日游戏时长：看自己时从 game_log 解析 Instance Joined 事件 ──
  try {
    const days: string[] = [];
    const hoursPerDay: number[] = [];
    const dayMap: Record<string, number> = {};

    if (isSelf.value) {
      const records = await DbApi.getGameLogs({ limit: 5000 });
      if (Array.isArray(records)) {
        // 按时间倒序取，重新升序处理
        const ordered = [...records].reverse();
        let curSessionStart: number | null = null;
        for (let i = 0; i < ordered.length; i++) {
          const r: any = ordered[i];
          const ts = new Date(r.time).getTime();
          if (isNaN(ts) || ts < cutoff) continue;
          const type = r.event_type || '';
          if (type === 'Instance Joined' || type === 'Authenticated') {
            curSessionStart = ts;
          } else if (type === 'Application Quit' && curSessionStart) {
            const duration = ts - curSessionStart;
            if (duration > 0 && duration < 24 * 3600 * 1000) {
              const dateKey = new Date(curSessionStart).toISOString().split('T')[0];
              dayMap[dateKey] = (dayMap[dateKey] || 0) + duration;
            }
            curSessionStart = null;
          }
        }
        // 最后一个会话还在进行中（没遇到 Application Quit）
        if (curSessionStart) {
          const duration = Date.now() - curSessionStart;
          if (duration > 0 && duration < 24 * 3600 * 1000) {
            const dateKey = new Date(curSessionStart).toISOString().split('T')[0];
            dayMap[dateKey] = (dayMap[dateKey] || 0) + duration;
          }
        }
      }
    } else {
      // 看他人：每个相遇事件按 0.5h 估算
      const logs = profileStore.activityLogs;
      for (const log of logs) {
        const at = (log as any).created_at;
        if (!at) continue;
        const ts = new Date(at).getTime();
        if (ts < cutoff) continue;
        const dateKey = new Date(at).toISOString().split('T')[0];
        dayMap[dateKey] = (dayMap[dateKey] || 0) + 0.5 * 3600 * 1000;
      }
    }

    // 填满整个时段（即使某天没数据也显示 0 柱）
    for (let i = periodDays - 1; i >= 0; i--) {
      const d = new Date(now - i * 24 * 60 * 60 * 1000);
      const key = d.toISOString().split('T')[0];
      days.push(key.slice(5)); // 显示 MM-DD
      const ms = dayMap[key] || 0;
      hoursPerDay.push(Math.min(ms / 3600000, 24));
    }
    dailyPlaytimeData.value = hoursPerDay;
    dailyPlaytimeLabels.value = days;
  } catch (e) {
    console.warn('Failed to build daily playtime', e);
  }

  // ── 最常去的世界：看自己从 game_log 的 Instance Joined 事件聚合 ──
  try {
    if (isSelf.value) {
      const records = await DbApi.getGameLogs({ limit: 5000 });
      const worldMap: Record<string, { name: string; visits: number; totalTime: number; lastTs: number }> = {};
      if (Array.isArray(records)) {
        const ordered = [...records].reverse();
        let prevWorld: string | null = null;
        let prevTs: number | null = null;
        for (let i = 0; i < ordered.length; i++) {
          const r: any = ordered[i];
          const ts = new Date(r.time).getTime();
          if (isNaN(ts) || ts < cutoff) continue;
          if (r.event_type === 'Instance Joined') {
            // content 形如 "wrld_xxx:12345~public" 或仅世界名
            const content = String(r.content || '');
            // 提取世界名（截取冒号前部分；如果是 wrld_xxx 形式就用作 ID）
            const colonIdx = content.indexOf(':');
            const worldName = colonIdx > 0 ? content.substring(0, colonIdx) : content;
            const key = worldName || t('user_profile.info.unknown');
            if (!worldMap[key]) {
              worldMap[key] = { name: key, visits: 0, totalTime: 0, lastTs: 0 };
            }
            worldMap[key].visits += 1;
            worldMap[key].lastTs = ts;
            // 上一个世界停留时长 = 这次进入的时间 - 上次进入的时间
            if (prevWorld && prevTs && worldMap[prevWorld]) {
              const dur = ts - prevTs;
              if (dur > 0 && dur < 24 * 3600 * 1000) {
                worldMap[prevWorld].totalTime += dur;
              }
            }
            prevWorld = key;
            prevTs = ts;
          }
        }
      }
      topWorldsRaw.value = Object.entries(worldMap).map(([id, w]) => ({
        id, name: w.name, imageUrl: '', visits: w.visits, totalTime: w.totalTime,
      }));
    } else {
      // 看他人没有这数据，留空
      topWorldsRaw.value = [];
    }
  } catch (e) {
    console.warn('Failed to build top worlds', e);
  }
}

watch(() => profileStore.isOpen, (open) => {
  if (open) {
    setTimeout(refreshActivityData, 500);
  }
});

watch(() => profileStore.activityLogs, () => {
  refreshActivityData();
}, { deep: true });

const trustInfo = computed(() => {
  const tags = profileStore.baseInfo?.tags || [];
  if (tags.includes("system_trust_legend")) return { label: "Legend", color: "#ff69b4" };
  if (tags.includes("system_trust_veteran")) return { label: "Trusted User", color: "#8b5cf6" };
  if (tags.includes("system_trust_trusted")) return { label: "Known User", color: "#ff7b42" };
  if (tags.includes("system_trust_known")) return { label: "User", color: "#2bcf5c" };
  if (tags.includes("system_trust_basic")) return { label: "New User", color: "#1778ff" };
  return { label: "Visitor", color: "#9e9e9e" };
});

// ========== Language Flags (VRCX 对齐) ==========
// 映射 VRChat 语言代码 → ISO 国家代码（用于 flagcdn SVG）
const languageToCountryCode: Record<string, string> = {
  eng: 'us', kor: 'kr', rus: 'ru', spa: 'es', por: 'pt',
  zho: 'cn', deu: 'de', jpn: 'jp', fra: 'fr', swe: 'se',
  nld: 'nl', pol: 'pl', dan: 'dk', nor: 'no', ita: 'it',
  tha: 'th', fin: 'fi', hun: 'hu', ces: 'cz', tur: 'tr',
  ara: 'ae', ron: 'ro', vie: 'vn', ukr: 'ua', ind: 'id',
  hrv: 'hr', heb: 'il', bul: 'bg', ell: 'gr', fil: 'ph',
  hin: 'in', msa: 'my', slk: 'sk', slv: 'si', lit: 'lt',
  lav: 'lv', est: 'ee', cmn: 'cn', yue: 'cn', wuu: 'cn', tws: 'cn',
  ase: 'us', bfi: 'gb', jsl: 'jp', kvk: 'kr',
  afr: 'za', ben: 'bd', cym: 'gb', gla: 'gb', gle: 'ie',
  hye: 'am', isl: 'is', ltz: 'lu', mar: 'in', mkd: 'mk',
  mlt: 'mt', mri: 'nz', sco: 'gb', tel: 'in',
};

const userLanguageFlags = computed(() => {
  const tags = profileStore.baseInfo?.tags || [];
  const flags: { code: string; cc: string }[] = [];
  for (const tag of tags) {
    if (tag.startsWith('language_')) {
      const code = tag.substring(9);
      const cc = languageToCountryCode[code];
      if (cc) {
        flags.push({ code, cc });
      }
    }
  }
  return flags;
});

// ========== Platform Detection (VRCX 对齐) ==========
const userPlatform = computed(() => {
  const info = profileStore.baseInfo as any;
  if (!info) return '';
  const platform = info.platform || info.last_platform || info.lastPlatform || '';
  if (platform === 'standalonewindows') return 'pc';
  if (platform === 'android') return 'android';
  if (platform === 'ios') return 'ios';
  return platform || '';
});

// ========== Age Verification (VRCX 对齐) ==========
// VRChat API: ageVerified (bool) + ageVerificationStatus ('18+' / 'verified' / null)
const ageVerificationLabel = computed(() => {
  const info = profileStore.baseInfo as any;
  if (!info?.ageVerified) return '';
  const status = info.ageVerificationStatus;
  if (status === '18+') return '18+';
  if (status) return 'Verified';
  return '';
});

// ========== VRChat 官方团队 (VRCX 对齐：检测 admin_* / moderator 标签) ==========
const isVrchatTeam = computed(() => {
  const tags = profileStore.baseInfo?.tags || [];
  return tags.some(t => t === 'admin_moderator' || t === 'admin_official_thumbnail' || t.startsWith('admin_'));
});

// ========== 打开 Discord 个人页 ==========
function openDiscord(discordId: string) {
  if (!discordId) return;
  // VRCX 标准: 优先打开 discord:// 协议，浏览器降级到网页版
  const url = `https://discord.com/users/${encodeURIComponent(discordId)}`;
  window.open(url, '_blank');
}

// ========== 曾用名下拉（A1 收尾）==========
const showPreviousNames = ref(false);
const previousDisplayNames = computed<{ displayName: string; updated_at?: string }[]>(() => {
  const info = profileStore.baseInfo as any;
  const arr = info?.previousDisplayNames || info?.previous_display_names || [];
  if (!Array.isArray(arr)) return [];
  return arr.map((item: any) => {
    if (typeof item === 'string') return { displayName: item };
    return { displayName: item.displayName || item.display_name || '', updated_at: item.updated_at };
  }).filter(x => x.displayName);
});

// ========== 状态点 tooltip 文本 ==========
const statusTooltip = computed(() => {
  const info = profileStore.baseInfo as any;
  if (!info) return '';
  const state = info.state;            // online / active / offline
  const status = (info.status || '').toLowerCase();  // active / join me / ask me / busy / offline
  const stateText = state === 'active' ? t('user_profile.status.active')
    : state === 'offline' ? t('user_profile.status.offline')
    : '';
  const statusText =
    status === 'active' ? t('user_profile.status.active')
    : status === 'join me' ? t('user_profile.status.join_me')
    : status === 'ask me' ? t('user_profile.status.ask_me')
    : status === 'busy' ? t('user_profile.status.busy')
    : t('user_profile.status.offline');
  // VRCX 风格："活跃 (来加入我)"
  if (stateText && status && status !== 'active') return `${stateText} (${statusText})`;
  return stateText || statusText;
});

// ========== Bio 翻译（接 OvrApi.translate）==========
const bioTranslation = ref<string>('');
const bioTranslating = ref(false);
async function translateBio() {
  const info = profileStore.baseInfo as any;
  const bio = info?.bio?.trim();
  if (!bio || bioTranslating.value) return;
  if (bioTranslation.value) {
    bioTranslation.value = ''; // 再点取消显示
    return;
  }
  bioTranslating.value = true;
  try {
    const res: any = await OvrApi.translate({
      req: { text: bio, source: 'auto', target: locale.value || 'zh-CN' }
    });
    bioTranslation.value = res?.text || res?.translation || res?.result || '';
    if (!bioTranslation.value) toast.error(t('user_profile.messages.translation_empty'));
  } catch (e: any) {
    toast.error(t('user_profile.messages.translation_failed', { error: e?.message || e }));
  } finally {
    bioTranslating.value = false;
  }
}

// 切换用户时清空翻译缓存
watch(() => profileStore.baseInfo?.id, () => {
  bioTranslation.value = '';
  showPreviousNames.value = false;
});

// ========== bioLinks 解析 + favicon ==========
const bioLinksParsed = computed<string[]>(() => {
  const info = profileStore.baseInfo as any;
  const arr = info?.bioLinks || info?.bio_links || [];
  if (!Array.isArray(arr)) return [];
  return arr.filter((x: any) => typeof x === 'string' && /^https?:\/\//.test(x));
});

function getFaviconUrl(url: string): string {
  try {
    const u = new URL(url);
    return `https://www.google.com/s2/favicons?domain=${encodeURIComponent(u.hostname)}&sz=32`;
  } catch {
    return '';
  }
}

// ========== A2 Info Tab：见面次数 / 一起时长 / 加好友时间 ==========
const meetingCount = ref(0);
const togetherTime = ref(0);          // milliseconds
const dateFriended = ref<string>('');

async function loadFriendStats() {
  meetingCount.value = 0;
  togetherTime.value = 0;
  dateFriended.value = '';
  if (!profileStore.targetUserId || isSelf.value) return;
  try {
    const logs = await DbApi.getFriendLogs({ userId: profileStore.targetUserId, limit: 500 });
    if (!Array.isArray(logs)) return;
    let count = 0;
    let lastJoinTs: number | null = null;
    let totalMs = 0;
    let earliestFriendAdd: string | null = null;

    for (const log of logs) {
      const t = (log as any).type;
      const ts = (log as any).created_at ? new Date((log as any).created_at).getTime() : 0;
      if (!ts) continue;

      if (t === 'PlayerJoined' || t === 'player_joined' || t === 'joined') {
        count += 1;
        lastJoinTs = ts;
      } else if ((t === 'PlayerLeft' || t === 'player_left' || t === 'left') && lastJoinTs) {
        totalMs += Math.max(0, ts - lastJoinTs);
        lastJoinTs = null;
      } else if (t === 'FriendAdd' || t === 'friend_add') {
        if (!earliestFriendAdd || ts < new Date(earliestFriendAdd).getTime()) {
          earliestFriendAdd = (log as any).created_at;
        }
      }
    }
    meetingCount.value = count;
    togetherTime.value = totalMs;
    dateFriended.value = earliestFriendAdd || '';
  } catch (e) {
    console.warn('Failed to load friend stats', e);
  }
}

// 友好的时长字符串（ms → "X天 Y小时" / "Y小时 Z分钟"）
function timeToText(ms: number): string {
  if (!ms || ms < 0) return '—';
  const sec = Math.floor(ms / 1000);
  const day = Math.floor(sec / 86400);
  const hour = Math.floor((sec % 86400) / 3600);
  const min = Math.floor((sec % 3600) / 60);
  if (day > 0) return t('user_profile.duration.day_hour', { day, hour });
  if (hour > 0) return t('user_profile.duration.hour_minute', { hour, min });
  if (min > 0) return t('user_profile.duration.minute', { min });
  return t('user_profile.duration.second', { sec });
}

// 在线时长 / 离线时长（VRCX userOnlineFor 等价）
const onlineForText = computed(() => {
  const info = profileStore.baseInfo as any;
  if (!info) return '—';
  const state = info.state;
  // 优先 last_login（在线状态时）vs last_activity（离线时）
  const ts = state === 'online' ? info.last_login : info.last_activity || info.last_login;
  if (!ts) return '—';
  const diff = Date.now() - new Date(ts).getTime();
  return timeToText(diff);
});

const onlineForLabel = computed(() => {
  const info = profileStore.baseInfo as any;
  if (info?.state === 'online') return t('user_profile.activity.current_online_duration');
  return t('user_profile.activity.offline_duration');
});

// ========== 代表群组 (representedGroup) ==========
const representedGroup = ref<any>(null);
const isLoadingRepresentedGroup = ref(false);

async function loadRepresentedGroup() {
  representedGroup.value = null;
  if (!profileStore.targetUserId) return;
  isLoadingRepresentedGroup.value = true;
  try {
    const result: any = await VrcApi.getRepresentedGroup({ userId: profileStore.targetUserId });
    if (result?.groupId && result?.isRepresenting !== false) {
      representedGroup.value = result;
    }
  } catch {
    // 用户无代表群组或接口报错，忽略
  } finally {
    isLoadingRepresentedGroup.value = false;
  }
}

// ========== 自己专属：4 个开关 + VRC+ 余额 + 出生点 ==========
const homeWorldName = ref<string>('');
async function loadHomeWorldName() {
  homeWorldName.value = '';
  const info = profileStore.baseInfo as any;
  if (!isSelf.value || !info?.homeLocation) return;
  const homeId = info.homeLocation;
  try {
    const cached = await DbApi.getApiCache({ key: `world_name:${homeId}` });
    if (cached) homeWorldName.value = cached;
    const world = await VrcApi.getWorld({ worldId: homeId });
    if (world?.name) {
      homeWorldName.value = world.name;
      DbApi.saveApiCache({ key: `world_name:${homeId}`, data: world.name }).catch(() => {});
    }
  } catch { /* ignore */ }
}

// ========== ID 复制下拉 ==========
const showIdCopyMenu = ref(false);
function copyById(kind: 'id' | 'url' | 'name') {
  const info = profileStore.baseInfo as any;
  if (!info) return;
  let value = '';
  if (kind === 'id') value = info.id;
  else if (kind === 'url') value = `https://vrchat.com/home/user/${info.id}`;
  else if (kind === 'name') value = info.displayName || info.id;
  navigator.clipboard.writeText(value);
  toast.success(t('user_profile.actions.copied_value', { value }));
  showIdCopyMenu.value = false;
}

// ========== A8a/A8b 子对话框：编辑器 + 邀请 ==========
type EditorKind = '' | 'bio' | 'note_memo' | 'pronouns' | 'social_status' | 'language' | 'send_invite' | 'send_invite_request' | 'invite_group';
type InviteMessageType = 'message' | 'request';
type InviteMessageSlot = {
  slot: number;
  message: string;
  updatedAt?: string;
  messageType: InviteMessageType;
};
type GroupInviteOption = {
  id: string;
  name: string;
  iconUrl?: string;
  shortCode?: string;
  permissions: string[];
};

const activeEditor = ref<EditorKind>('');
const editorSaving = ref(false);

// Bio
const bioDraft = ref('');
const bioLinksDraft = ref<string[]>([]);
const bioLinkInput = ref('');

// Note + Memo
const noteDraft = ref('');
const memoDraft = ref('');

// Pronouns
const pronounsDraft = ref('');

// Social Status
const statusOptions = computed(() => [
  { value: 'active', label: t('user_profile.status.active_join_me') },
  { value: 'join me', label: t('user_profile.status.join_me') },
  { value: 'ask me', label: t('user_profile.status.ask_me') },
  { value: 'busy', label: t('user_profile.status.busy_do_not_disturb') },
]);
const socialStatusDraft = ref('active');
const statusDescDraft = ref('');

// Language（VRChat tag system: language_xxx，最多 3 个）
const languageOptions = [
  { code: 'eng', label: 'English' }, { code: 'jpn', label: '日本語' },
  { code: 'kor', label: '한국어' }, { code: 'zho', label: '简体中文' },
  { code: 'tws', label: '繁體中文' }, { code: 'fra', label: 'Français' },
  { code: 'deu', label: 'Deutsch' }, { code: 'spa', label: 'Español' },
  { code: 'por', label: 'Português' }, { code: 'rus', label: 'Русский' },
  { code: 'ita', label: 'Italiano' }, { code: 'nld', label: 'Nederlands' },
  { code: 'pol', label: 'Polski' }, { code: 'ukr', label: 'Українська' },
  { code: 'tha', label: 'ภาษาไทย' }, { code: 'vie', label: 'Tiếng Việt' },
  { code: 'tur', label: 'Türkçe' }, { code: 'ara', label: 'العربية' },
  { code: 'heb', label: 'עברית' }, { code: 'fin', label: 'Suomi' },
  { code: 'hun', label: 'Magyar' }, { code: 'ces', label: 'Čeština' },
];
const languagesDraft = ref<string[]>([]);

// Group invite
const groupInviteOptions = ref<GroupInviteOption[]>([]);
const groupInviteLoading = ref(false);
const groupInviteError = ref('');
const selectedGroupInviteId = ref('');
const groupInviteSearch = ref('');
const filteredGroupInviteOptions = computed(() => {
  const query = groupInviteSearch.value.trim().toLowerCase();
  if (!query) return groupInviteOptions.value;
  return groupInviteOptions.value.filter(group => {
    return group.name.toLowerCase().includes(query) || (group.shortCode || '').toLowerCase().includes(query);
  });
});

// A8b 邀请相关状态
const inviteMessageSlots = ref<InviteMessageSlot[]>([]);
const inviteMessagesLoading = ref(false);
const inviteMessagesError = ref('');
const selectedInviteSlot = ref<number | null>(null);
const inviteMessageDraft = ref('');
const inviteMessageType = computed<InviteMessageType>(() => activeEditor.value === 'send_invite_request' ? 'request' : 'message');
const selectedInviteMessage = computed(() => inviteMessageSlots.value.find(row => row.slot === selectedInviteSlot.value) || null);
const currentUserLocation = computed(() => {
  const loc = (authStore.currentUser as any)?.location || '';
  return typeof loc === 'string' ? loc : '';
});
const inviteHasInstance = computed(() => {
  const loc = currentUserLocation.value;
  return !!loc && loc.startsWith('wrld_') && loc !== 'private' && loc !== 'offline' && loc !== 'traveling';
});
const editorSubmitDisabled = computed(() => {
  if (activeEditor.value === 'send_invite' || activeEditor.value === 'send_invite_request') {
    return editorSaving.value || inviteMessagesLoading.value || selectedInviteSlot.value === null;
  }
  if (activeEditor.value === 'invite_group') {
    return editorSaving.value || groupInviteLoading.value || !selectedGroupInviteId.value;
  }
  return editorSaving.value;
});

function isUsableInviteLocation(location: string): boolean {
  return !!location && location.startsWith('wrld_') && location !== 'private' && location !== 'offline' && location !== 'traveling';
}

function extractGroupId(group: any): string {
  return String(group?.id || group?.groupId || group?.group?.id || group?.group?.groupId || '');
}

function extractGroupName(group: any): string {
  return String(group?.name || group?.displayName || group?.group?.name || group?.group?.displayName || extractGroupId(group));
}

function extractGroupPermissions(group: any, permissionMap: any): string[] {
  const groupId = extractGroupId(group);
  const direct =
    group?.permissions ||
    group?.myMember?.permissions ||
    group?.member?.permissions ||
    group?.group?.permissions ||
    group?.group?.myMember?.permissions;
  if (Array.isArray(direct)) return direct;

  if (Array.isArray(permissionMap)) {
    const entry = permissionMap.find((item: any) => item?.groupId === groupId || item?.id === groupId);
    if (Array.isArray(entry?.permissions)) return entry.permissions;
  } else if (permissionMap && typeof permissionMap === 'object') {
    const entry = permissionMap[groupId];
    if (Array.isArray(entry)) return entry;
    if (Array.isArray(entry?.permissions)) return entry.permissions;
  }

  return [];
}

function canInviteToGroup(permissions: string[]): boolean {
  return permissions.includes('*') || permissions.includes('group-invites-manage');
}

async function loadGroupInviteOptions() {
  groupInviteLoading.value = true;
  groupInviteError.value = '';
  groupInviteOptions.value = [];
  selectedGroupInviteId.value = '';
  try {
    const [groupsRes, permissionsRes] = await Promise.all([
      VrcApi.getGroups(),
      VrcApi.getUserGroupPermissions({ userId: 'me' }).catch(() => null),
    ]);
    const groups = Array.isArray(groupsRes) ? groupsRes.map((entry: any) => entry?.group || entry) : [];
    const options = groups
      .map((group: any) => {
        const id = extractGroupId(group);
        const permissions = extractGroupPermissions(group, permissionsRes);
        return {
          id,
          name: extractGroupName(group),
          iconUrl: group?.iconUrl || group?.group?.iconUrl,
          shortCode: group?.shortCode || group?.group?.shortCode,
          permissions,
        };
      })
      .filter((group: GroupInviteOption) => group.id && canInviteToGroup(group.permissions));

    groupInviteOptions.value = options.sort((a, b) => a.name.localeCompare(b.name));
    selectedGroupInviteId.value = groupInviteOptions.value[0]?.id || '';
    if (groupInviteOptions.value.length === 0) {
      groupInviteError.value = t('user_profile.editor.no_invitable_groups');
    }
  } catch (e: any) {
    groupInviteError.value = e?.message || String(e);
  } finally {
    groupInviteLoading.value = false;
  }
}

function normalizeInviteMessageSlots(raw: any, messageType: InviteMessageType): InviteMessageSlot[] {
  const source = raw?.json ?? raw;
  const rows = new globalThis.Map<number, InviteMessageSlot>();
  const addRow = (value: any, fallbackSlot: number) => {
    const slot = Number(value?.slot ?? fallbackSlot);
    if (!Number.isFinite(slot)) return;
    rows.set(slot, {
      slot,
      message: typeof value === 'string' ? value : String(value?.message ?? ''),
      updatedAt: value?.updatedAt || value?.updated_at || value?.updated,
      messageType: (value?.messageType || messageType) as InviteMessageType,
    });
  };

  if (Array.isArray(source)) {
    source.forEach((value, index) => addRow(value, index));
  } else if (source && typeof source === 'object') {
    Object.entries(source).forEach(([key, value]) => addRow(value, Number(key)));
  }

  for (let slot = 0; slot < 4; slot += 1) {
    if (!rows.has(slot)) rows.set(slot, { slot, message: '', messageType });
  }

  return Array.from(rows.values()).sort((a, b) => a.slot - b.slot);
}

function selectInviteMessageSlot(row: InviteMessageSlot) {
  selectedInviteSlot.value = row.slot;
  inviteMessageDraft.value = row.message || '';
}

function inviteSlotCooldownText(row: InviteMessageSlot): string {
  if (!row.updatedAt) return t('user_profile.editor.available');
  const updated = new Date(row.updatedAt).getTime();
  if (!Number.isFinite(updated)) return t('user_profile.editor.available');
  const remaining = updated + 60 * 60 * 1000 - Date.now();
  return remaining > 0 ? timeToText(remaining) : t('user_profile.editor.available');
}

async function loadInviteMessageSlots(kind: EditorKind) {
  if (kind !== 'send_invite' && kind !== 'send_invite_request') return;
  const localUserId = (authStore.currentUser as any)?.id || profileStore.myId;
  const messageType: InviteMessageType = kind === 'send_invite_request' ? 'request' : 'message';
  inviteMessagesLoading.value = true;
  inviteMessagesError.value = '';
  inviteMessageSlots.value = normalizeInviteMessageSlots(null, messageType);
  selectInviteMessageSlot(inviteMessageSlots.value[0]);
  try {
    if (!localUserId) throw new Error(t('user_profile.messages.missing_current_user'));
    const result = await VrcApi.getInviteMessages({ userId: localUserId, messageType });
    inviteMessageSlots.value = normalizeInviteMessageSlots(result, messageType);
    selectInviteMessageSlot(inviteMessageSlots.value[0]);
  } catch (e: any) {
    inviteMessagesError.value = e?.message || String(e);
  } finally {
    inviteMessagesLoading.value = false;
  }
}

async function refreshCurrentUserLocation(): Promise<string> {
  if (isUsableInviteLocation(currentUserLocation.value)) return currentUserLocation.value;
  try {
    const fresh: any = await VrcApi.getCurrentUser();
    if (fresh?.id) {
      authStore.currentUser = { ...(authStore.currentUser || {}), ...fresh } as any;
    }
    return typeof fresh?.location === 'string' ? fresh.location : currentUserLocation.value;
  } catch {
    return currentUserLocation.value;
  }
}

async function getInviteWorldName(location: string): Promise<string> {
  const worldId = location.split(':')[0];
  if (!worldId) return '';
  try {
    const cached = await DbApi.getApiCache({ key: `world_name:${worldId}` });
    if (cached) return cached;
  } catch { /* ignore */ }
  try {
    const world: any = await VrcApi.getWorld({ worldId });
    if (world?.name) {
      DbApi.saveApiCache({ key: `world_name:${worldId}`, data: world.name }).catch(() => {});
      return world.name;
    }
  } catch { /* ignore */ }
  return worldId;
}

async function saveInviteMessageSlotIfChanged(): Promise<number> {
  const row = selectedInviteMessage.value;
  const slot = row?.slot ?? selectedInviteSlot.value ?? 0;
  const message = inviteMessageDraft.value.trim();
  const localUserId = (authStore.currentUser as any)?.id || profileStore.myId;
    if (!localUserId) throw new Error(t('user_profile.messages.missing_current_user'));
  if (!row || message === (row.message || '')) return slot;

  await VrcApi.editInviteMessage({
    userId: localUserId,
    messageType: row.messageType || inviteMessageType.value,
    slot,
    message,
  });

  row.message = message;
  row.updatedAt = new Date().toISOString();
  return slot;
}

function openEditor(kind: EditorKind) {
  showMoreMenu.value = false;
  const info = profileStore.baseInfo as any;
  if (!info) return;
  if (kind === 'bio') {
    bioDraft.value = info.bio || '';
    bioLinksDraft.value = Array.isArray(info.bioLinks) ? [...info.bioLinks] : [];
    bioLinkInput.value = '';
  } else if (kind === 'note_memo') {
    noteDraft.value = info.note || '';
    memoDraft.value = info.memo || '';
  } else if (kind === 'pronouns') {
    pronounsDraft.value = info.pronouns || '';
  } else if (kind === 'social_status') {
    socialStatusDraft.value = info.status || 'active';
    statusDescDraft.value = info.statusDescription || '';
  } else if (kind === 'language') {
    const tags: string[] = info.tags || [];
    languagesDraft.value = tags
      .filter(t => t.startsWith('language_'))
      .map(t => t.substring(9));
  } else if (kind === 'send_invite' || kind === 'send_invite_request') {
    inviteMessageSlots.value = [];
    inviteMessageDraft.value = '';
    selectedInviteSlot.value = null;
    inviteMessagesError.value = '';
    loadInviteMessageSlots(kind);
  } else if (kind === 'invite_group') {
    groupInviteSearch.value = '';
    groupInviteError.value = '';
    loadGroupInviteOptions();
  }
  activeEditor.value = kind;
}

function closeEditor() { activeEditor.value = ''; }

function addBioLink() {
  const url = bioLinkInput.value.trim();
  if (!url) return;
  if (!/^https?:\/\//i.test(url)) {
    toast.error(t('user_profile.editor.link_invalid'));
    return;
  }
  if (bioLinksDraft.value.length >= 3) {
    toast.error(t('user_profile.editor.max_links'));
    return;
  }
  bioLinksDraft.value.push(url);
  bioLinkInput.value = '';
}

function removeBioLink(idx: number) {
  bioLinksDraft.value.splice(idx, 1);
}

function toggleLanguage(code: string) {
  const idx = languagesDraft.value.indexOf(code);
  if (idx >= 0) {
    languagesDraft.value.splice(idx, 1);
  } else {
    if (languagesDraft.value.length >= 3) {
      toast.error(t('user_profile.editor.max_languages'));
      return;
    }
    languagesDraft.value.push(code);
  }
}

async function submitEditor() {
  const kind = activeEditor.value;
  const info = profileStore.baseInfo as any;
  if (!kind || !info) return;
  editorSaving.value = true;
  try {
    if (kind === 'bio') {
      if (bioDraft.value.length > 512) { toast.error(t('user_profile.editor.bio_too_long')); return; }
      await VrcApi.saveCurrentUser({ bio: bioDraft.value, bioLinks: bioLinksDraft.value });
      info.bio = bioDraft.value;
      info.bioLinks = [...bioLinksDraft.value];
      toast.success(t('user_profile.editor.bio_saved'));
    } else if (kind === 'note_memo') {
      // VRChat 的 note 字段是给别人加备注用，看自己资料时没意义；这里把它当作 statusDescription 的备忘
      // 实际上 memo/note 本地存储更合适，这里走 saveCurrentUser 仅同步 note
      await VrcApi.saveCurrentUser({ note: noteDraft.value });
      info.note = noteDraft.value;
      info.memo = memoDraft.value;
      // 本地也存一份 memo（VRChat 没有这个字段）
      await DbApi.saveSetting({ key: `self_memo:${info.id}`, value: JSON.stringify(memoDraft.value) });
      toast.success(t('user_profile.editor.note_memo_saved'));
    } else if (kind === 'pronouns') {
      if (pronounsDraft.value.length > 32) { toast.error(t('user_profile.editor.pronouns_too_long')); return; }
      await VrcApi.saveCurrentUser({ pronouns: pronounsDraft.value });
      info.pronouns = pronounsDraft.value;
      toast.success(t('user_profile.editor.pronouns_saved'));
    } else if (kind === 'social_status') {
      if (statusDescDraft.value.length > 32) { toast.error(t('user_profile.editor.status_desc_too_long')); return; }
      await VrcApi.saveCurrentUser({
        status: socialStatusDraft.value,
        statusDescription: statusDescDraft.value,
      });
      info.status = socialStatusDraft.value;
      info.statusDescription = statusDescDraft.value;
      toast.success(t('user_profile.editor.social_status_saved'));
    } else if (kind === 'language') {
      // VRChat API: 语言通过 tags 数组的 language_xxx 来设置
      // 保留非 language_ 标签，加上新选的语言
      const otherTags = (info.tags || []).filter((t: string) => !t.startsWith('language_'));
      const newTags = [...otherTags, ...languagesDraft.value.map(c => `language_${c}`)];
      await VrcApi.saveCurrentUser({ tags: newTags });
      info.tags = newTags;
      toast.success(t('user_profile.editor.languages_saved'));
    } else if (kind === 'invite_group') {
      if (isSelf.value) { toast.error(t('user_profile.editor.cannot_invite_self_group')); return; }
      const group = groupInviteOptions.value.find(item => item.id === selectedGroupInviteId.value);
      if (!group) { toast.error(t('user_profile.editor.select_invitable_group')); return; }
      try {
        await VrcApi.sendGroupInvite({ groupId: group.id, userId: info.id });
        toast.success(t('user_profile.editor.group_invite_sent', { user: info.displayName, group: group.name }));
      } catch (e: any) {
        toast.error(t('user_profile.editor.group_invite_failed', { error: e?.message || e }));
        return;
      }
    } else if (kind === 'send_invite') {
      if (isSelf.value) { toast.error(t('user_profile.editor.cannot_invite_self')); return; }
      const location = await refreshCurrentUserLocation();
      if (!isUsableInviteLocation(location)) {
        toast.error(t('user_profile.editor.not_invitable_location'));
        return;
      }
      try {
        const slot = await saveInviteMessageSlotIfChanged();
        const worldName = await getInviteWorldName(location);
        await VrcApi.inviteUser({
          userId: info.id,
          instanceId: location,
          worldId: location,
          worldName,
          messageSlot: slot,
        });
        toast.success(t('user_profile.editor.invite_sent_to', { user: info.displayName }));
      } catch (e: any) {
        const msg = e?.message || '';
        if (msg.includes('cooldown') || msg.includes('429')) {
          toast.error(t('user_profile.editor.invite_cooldown'));
        } else {
          toast.error(t('user_profile.editor.invite_failed', { error: msg }));
        }
        return;
      }
    } else if (kind === 'send_invite_request') {
      if (isSelf.value) { toast.error(t('user_profile.editor.cannot_request_self')); return; }
      try {
        const slot = await saveInviteMessageSlotIfChanged();
        await VrcApi.requestInvite({
          userId: info.id,
          platform: 'standalonewindows',
          requestSlot: slot,
        });
        toast.success(t('user_profile.editor.request_sent_to', { user: info.displayName }));
      } catch (e: any) {
        const msg = e?.message || '';
        if (msg.includes('cooldown') || msg.includes('429')) {
          toast.error(t('user_profile.editor.request_cooldown'));
        } else {
          toast.error(t('user_profile.editor.request_failed', { error: msg }));
        }
        return;
      }
    }
    activeEditor.value = '';
  } catch (e: any) {
    toast.error(t('user_profile.editor.save_failed', { error: e?.message || e }));
  } finally {
    editorSaving.value = false;
  }
}

const statusColor = computed(() => {
  switch (profileStore.baseInfo?.status?.toLowerCase()) {
    case "active": return "#22c55e";
    case "join me": return "#3b82f6";
    case "ask me": return "#f97316";
    case "busy": case "do not disturb": return "#ef4444";
    default: return "#64748b";
  }
});

const avatarImageUrl = computed(() =>
  profileStore.baseInfo?.profilePicOverride ||
  profileStore.baseInfo?.currentAvatarImageUrl ||
  profileStore.baseInfo?.currentAvatarThumbnailImageUrl || ""
);

const executeAction = async (action: string) => {
  const userId = profileStore.baseInfo?.id;
  if (!userId) return;
  showMoreMenu.value = false;
  try {
    switch (action) {
      case "refresh": await profileStore.openProfile(userId); toast.success(t("user_profile.actions.refresh_success")); break;
      case "copy_id": navigator.clipboard.writeText(userId); toast.success(t("user_profile.actions.copy_id_success")); break;
      case "copy_url": navigator.clipboard.writeText("https://vrchat.com/home/user/" + userId); toast.success(t("user_profile.actions.copy_url_success")); break;
      case "view_vrc": window.open("https://vrchat.com/home/user/" + userId, "_blank"); break;
      case "invite": openEditor('send_invite'); break;
      case "request_invite": openEditor('send_invite_request'); break;
      case "invite_group": openEditor('invite_group'); break;
      case "unfriend": if (confirm(t("user_profile.actions.unfriend_confirm"))) { await VrcApi.unfriend({ userId }); toast.success(t("user_profile.actions.unfriend_success")); profileStore.closeProfile(); } break;
      case "block": await VrcApi.moderateUser({ moderated: userId, type: "block" }); toast.success(t("user_profile.actions.block_success")); break;
      case "mute": await VrcApi.moderateUser({ moderated: userId, type: "mute" }); toast.success(t("user_profile.actions.mute_success")); break;
      case "show_avatar": await VrcApi.moderateUser({ moderated: userId, type: "showAvatar" }); toast.success(t("user_profile.actions.show_avatar_success")); break;
      case "hide_avatar": await VrcApi.moderateUser({ moderated: userId, type: "hideAvatar" }); toast.success(t("user_profile.actions.hide_avatar_success")); break;
      // ── 看自己时的菜单（VRCX 对齐，先 toast 占位，子对话框后续实现）──
      case "show_avatar_info": {
        const info = profileStore.baseInfo as any;
        const url = info?.currentAvatarImageUrl;
        if (url) window.open(url, '_blank'); else toast.info(t('user_profile.actions.no_avatar_info'));
        break;
      }
      case "show_fallback_avatar_info": {
        const info = profileStore.baseInfo as any;
        const fb = info?.fallbackAvatar;
        if (fb) toast.info(t('user_profile.actions.fallback_avatar_id', { id: fb })); else toast.info(t('user_profile.actions.no_fallback_avatar'));
        break;
      }
      case "edit_social_status": openEditor('social_status'); break;
      case "edit_language": openEditor('language'); break;
      case "edit_bio": openEditor('bio'); break;
      case "edit_pronouns": openEditor('pronouns'); break;
      case "edit_note_memo": openEditor('note_memo'); break;
      case "toggle_avatar_cloning": {
        if (!isSelf.value) break;
        const info = profileStore.baseInfo as any;
        const newValue = !info?.allowAvatarCopying;
        try {
          await VrcApi.saveCurrentUser({ allowAvatarCopying: newValue });
          if (info) info.allowAvatarCopying = newValue;
          toast.success(newValue ? t('user_profile.actions.avatar_copy_enabled') : t('user_profile.actions.avatar_copy_disabled'));
        } catch (e: any) { toast.error(t('user_profile.actions.action_failed', { error: e?.message || e })); }
        break;
      }
      case "toggle_boop": {
        if (!isSelf.value) break;
        const info = profileStore.baseInfo as any;
        const newValue = !info?.isBoopingEnabled;
        try {
          await VrcApi.saveCurrentUser({ isBoopingEnabled: newValue });
          if (info) info.isBoopingEnabled = newValue;
          toast.success(newValue ? t('user_profile.actions.boop_enabled') : t('user_profile.actions.boop_disabled'));
        } catch (e: any) { toast.error(t('user_profile.actions.action_failed', { error: e?.message || e })); }
        break;
      }
      case "toggle_shared_connections": {
        if (!isSelf.value) break;
        const info = profileStore.baseInfo as any;
        const newOptOut = !info?.hasSharedConnectionsOptOut;
        try {
          await VrcApi.saveCurrentUser({ hasSharedConnectionsOptOut: newOptOut });
          if (info) info.hasSharedConnectionsOptOut = newOptOut;
          toast.success(newOptOut ? t('user_profile.actions.shared_connections_hidden') : t('user_profile.actions.shared_connections_visible'));
        } catch (e: any) { toast.error(t('user_profile.actions.action_failed', { error: e?.message || e })); }
        break;
      }
      case "toggle_discord_friends": {
        if (!isSelf.value) break;
        const info = profileStore.baseInfo as any;
        const newOptOut = !info?.hasDiscordFriendsOptOut;
        try {
          await VrcApi.saveCurrentUser({ hasDiscordFriendsOptOut: newOptOut });
          if (info) info.hasDiscordFriendsOptOut = newOptOut;
          toast.success(newOptOut ? t('user_profile.actions.discord_friends_hidden') : t('user_profile.actions.discord_friends_visible'));
        } catch (e: any) { toast.error(t('user_profile.actions.action_failed', { error: e?.message || e })); }
        break;
      }
      case "reset_home_location": {
        if (!isSelf.value) break;
        if (!confirm(t('user_profile.actions.confirm_reset_home'))) break;
        try {
          await VrcApi.saveCurrentUser({ homeLocation: '' });
          const info = profileStore.baseInfo as any;
          if (info) info.homeLocation = '';
          homeWorldName.value = '';
          toast.success(t('user_profile.actions.home_reset_success'));
        } catch (e: any) { toast.error(t('user_profile.actions.action_failed', { error: e?.message || e })); }
        break;
      }
      // A6: 点击世界/模型卡片打开详情（暂时降级到 VRChat 官网）
      default: {
        if (action.startsWith('open_world:')) {
          const wid = action.substring('open_world:'.length);
          window.open(`https://vrchat.com/home/world/${wid}`, '_blank');
        } else if (action.startsWith('open_avatar:')) {
          const aid = action.substring('open_avatar:'.length);
          window.open(`https://vrchat.com/home/avatar/${aid}`, '_blank');
        }
        break;
      }
    }
  } catch (e: any) { toast.error(e.message); }
};

const addToFavGroup = async (groupId: string) => {
  const userId = profileStore.baseInfo?.id;
  if (!userId) return;
  showFavPicker.value = false;
  try {
    await VrcApi.addFavorite({ type: "friend", favoriteId: userId, tags: [groupId] });
    toast.success(t("user_profile.actions.favorited"));
  } catch (e: any) { toast.error(e.message); }
};

const mutualSearch = ref("");
const groupSearch = ref("");
const worldSearch = ref("");
const avatarSearch = ref("");

const filteredMutual = computed(() => {
  const q = mutualSearch.value.toLowerCase();
  let list = q ? profileStore.mutualFriends.filter(f => f.displayName?.toLowerCase().includes(q)) : [...profileStore.mutualFriends];
  // A4 排序
  if (mutualSort.value === 'alphabetical') {
    list.sort((a, b) => (a.displayName || '').localeCompare(b.displayName || ''));
  } else if (mutualSort.value === 'lastActive') {
    list.sort((a: any, b: any) => {
      const ta = new Date(a.last_activity || a.last_login || 0).getTime();
      const tb = new Date(b.last_activity || b.last_login || 0).getTime();
      return tb - ta;
    });
  }
  // friendOrder 走原始顺序，不动
  return list;
});

// A4 排序状态
const mutualSort = ref<'friendOrder' | 'alphabetical' | 'lastActive'>('alphabetical');

// A5 群组三段分组（拥有 / 共同 / 其他）
const groupSort = ref<'alphabetical' | 'members'>('alphabetical');

// A6 Worlds / FavWorlds / Avatars 排序与顺序
const worldSort = ref<'updated' | 'created' | 'name' | 'visits' | 'favorites'>('updated');
const worldOrder = ref<'desc' | 'asc'>('desc');
const favWorldSort = ref<'updated' | 'created' | 'name'>('updated');
const favWorldOrder = ref<'desc' | 'asc'>('desc');
const avatarSort = ref<'updated' | 'created' | 'name'>('updated');
const avatarOrder = ref<'desc' | 'asc'>('desc');

function sortByField<T extends Record<string, any>>(arr: T[], field: string, dir: 'desc' | 'asc'): T[] {
  const sorted = [...arr].sort((a, b) => {
    const av = a[field];
    const bv = b[field];
    if (typeof av === 'number' && typeof bv === 'number') return av - bv;
    if (av instanceof Date || /\d{4}-\d{2}-\d{2}/.test(String(av || ''))) {
      return new Date(av).getTime() - new Date(bv).getTime();
    }
    return String(av || '').localeCompare(String(bv || ''));
  });
  return dir === 'desc' ? sorted.reverse() : sorted;
}

function getSortField(sort: string): string {
  switch (sort) {
    case 'updated': return 'updated_at';
    case 'created': return 'created_at';
    case 'name': return 'name';
    case 'visits': return 'visits';
    case 'favorites': return 'favorites';
    default: return 'updated_at';
  }
}

function sortGroupArray(arr: any[]): any[] {
  if (groupSort.value === 'alphabetical') {
    return [...arr].sort((a, b) => (a.name || '').localeCompare(b.name || ''));
  }
  if (groupSort.value === 'members') {
    return [...arr].sort((a, b) => (b.memberCount || 0) - (a.memberCount || 0));
  }
  return arr;
}

const groupedGroups = computed(() => {
  const q = groupSearch.value.toLowerCase();
  const filtered = q ? profileStore.groups.filter(g => (g.name || '').toLowerCase().includes(q)) : [...profileStore.groups];
  const ownGroups: any[] = [];
  const mutualGroups: any[] = [];
  const otherGroups: any[] = [];
  const targetId = profileStore.targetUserId;

  for (const g of filtered) {
    if (g.ownerId === targetId) {
      ownGroups.push(g);
    } else if (g.mutualGroup && !isSelf.value) {
      mutualGroups.push(g);
    } else {
      otherGroups.push(g);
    }
  }
  return {
    own: sortGroupArray(ownGroups),
    mutual: sortGroupArray(mutualGroups),
    other: sortGroupArray(otherGroups),
    total: filtered.length,
  };
});

const filteredGroups = computed(() => {
  const q = groupSearch.value.toLowerCase();
  return q ? profileStore.groups.filter(g => g.name?.toLowerCase().includes(q)) : profileStore.groups;
});
const filteredWorlds = computed(() => {
  const q = worldSearch.value.toLowerCase();
  const list = q ? profileStore.createdWorlds.filter(w => w.name?.toLowerCase().includes(q)) : [...profileStore.createdWorlds];
  return sortByField(list, getSortField(worldSort.value), worldOrder.value);
});
const filteredFavoriteWorlds = computed(() => {
  const q = worldSearch.value.toLowerCase();
  const arr = (profileStore as any).favoriteWorlds || [];
  const list = q ? arr.filter((w: any) => w.name?.toLowerCase().includes(q)) : [...arr];
  return sortByField(list, getSortField(favWorldSort.value), favWorldOrder.value);
});
const filteredAvatars = computed(() => {
  const q = avatarSearch.value.toLowerCase();
  const list = q ? profileStore.createdAvatars.filter(a => a.name?.toLowerCase().includes(q)) : [...profileStore.createdAvatars];
  return sortByField(list, getSortField(avatarSort.value), avatarOrder.value);
});

const fmt = (d?: string) => d ? new Date(d).toLocaleDateString() : "-";
const fmtTime = (d?: string) => d ? new Date(d).toLocaleString() : "-";

// Trust color helper for friend lists
const getFriendTrustColor = (tags: string[]) => {
  if (!tags || !tags.length) return 'var(--theme-text-strong)';
  if (tags.includes('system_trust_legend')) return '#ff69b4';
  if (tags.includes('system_trust_veteran')) return '#8b5cf6';
  if (tags.includes('system_trust_trusted')) return '#ff7b42';
  if (tags.includes('system_trust_known')) return '#2bcf5c';
  if (tags.includes('system_trust_basic')) return '#1778ff';
  return 'var(--theme-text-strong)';
};

const highlightJson = (obj: any) => {
  if (!obj) return "{}";
  let json = JSON.stringify(obj, null, 2).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  return json.replace(/("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g, m => {
    let cls = "json-num";
    if (/^"/.test(m)) cls = /:$/.test(m) ? "json-key" : "json-str";
    else if (/true|false/.test(m)) cls = "json-bool";
    else if (/null/.test(m)) cls = "json-null";
    return `<span class="${cls}">${m}</span>`;
  });
};

const closeMenus = (e: MouseEvent) => {
  if (showFavPicker.value || showMoreMenu.value || openedBadgeId.value || showPreviousNames.value || showIdCopyMenu.value) {
    const el = e.target as HTMLElement;
    if (!el.closest(".more-menu-wrap")) showMoreMenu.value = false;
    if (!el.closest(".fav-menu-wrap")) showFavPicker.value = false;
    if (!el.closest(".badge-popover") && !el.closest("img.cursor-pointer")) openedBadgeId.value = null;
    if (!el.closest(".prev-names-wrap")) showPreviousNames.value = false;
    if (!el.closest(".id-copy-wrap")) showIdCopyMenu.value = false;
  }
};

// ── Location / Instance info (like VRCX) ──────────────────────────
const locationWorldName = ref<string>('');
const locationInstanceInfo = ref<string>('');
const locationRegionFlag = ref<string>('');
const locationPlayerCount = ref<string>('');
const instanceUsers = ref<any[]>([]);
const isLoadingInstance = ref(false);
const currentAvatarName = ref<string>('');

// Fetch current avatar name from avatar ID
const fetchAvatarName = async () => {
  const info = profileStore.baseInfo as any;
  // currentAvatar field is only available for the current user's own profile
  // For other users, we try currentAvatarImageUrl or fallbackAvatar
  const avatarId = info?.currentAvatar || info?.fallbackAvatar;
  if (!avatarId || !avatarId.startsWith('avtr_')) {
    currentAvatarName.value = '';
    return;
  }
  try {
    // Try cache first
    const cached = await DbApi.getApiCache({ key: `avatar_name:${avatarId}` });
    if (cached) {
      currentAvatarName.value = cached;
      return;
    }
    const avatar = await VrcApi.getAvatar({ avatarId });
    if (avatar?.name) {
      currentAvatarName.value = avatar.name;
      DbApi.saveApiCache({ key: `avatar_name:${avatarId}`, data: avatar.name }).catch(() => {});
    }
  } catch {
    // Avatar might be private/deleted - show as unknown
    currentAvatarName.value = '';
  }
};

// Relative time formatting (like VRCX: "15秒", "18分钟", "2小时")
const relativeTime = (dateStr?: string) => {
  if (!dateStr) return "—";
  const diff = Date.now() - new Date(dateStr).getTime();
  if (diff < 0) return "—";
  const seconds = Math.floor(diff / 1000);
  if (seconds < 60) return t('user_profile.duration.second', { sec: seconds });
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return t('user_profile.duration.minute', { min: minutes });
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return t('user_profile.duration.hour_minute', { hour: hours, min: minutes % 60 });
  const days = Math.floor(hours / 24);
  return t('user_profile.duration.day_ago', { day: days });
};

// Parse location string like "wrld_xxx:12345~region(jp)~group(grp_xxx)~groupAccessType(plus)~nonce(xxx)"
const parseLocationInfo = async () => {
  const loc = profileStore.baseInfo?.location;
  if (!loc || loc === 'offline' || loc === 'private' || loc === 'traveling') {
    locationWorldName.value = '';
    locationInstanceInfo.value = '';
    locationRegionFlag.value = '';
    locationPlayerCount.value = '';
    instanceUsers.value = [];
    return;
  }

  // Extract worldId and instanceId
  const parts = loc.split(':');
  const worldId = parts[0];
  const instancePart = parts[1] || '';

  // Parse region
  const regionMatch = instancePart.match(/region\(([^)]+)\)/);
  const region = regionMatch ? regionMatch[1] : '';
  const regionFlags: Record<string, string> = { jp: '🇯🇵', us: '🇺🇸', use: '🇺🇸', usw: '🇺🇸', eu: '🇪🇺' };
  locationRegionFlag.value = regionFlags[region] || '🌐';

  // Parse access type
  const groupMatch = instancePart.match(/group\(([^)]+)\)/);
  const accessMatch = instancePart.match(/groupAccessType\(([^)]+)\)/);
  let accessLabel = '';
  if (accessMatch) {
    const accessMap: Record<string, string> = {
      public: t('user_profile.list.release_public'),
      plus: t('user_profile.info.access_friends_plus'),
      members: t('user_profile.info.access_members'),
      friends: t('user_profile.info.friend_badge'),
    };
    accessLabel = accessMap[accessMatch[1]] || accessMatch[1];
  } else if (instancePart.includes('~friends')) {
    accessLabel = t('user_profile.info.friend_badge');
  } else if (instancePart.includes('~hidden')) {
    accessLabel = `${t('user_profile.info.friend_badge')}+`;
  } else if (instancePart.includes('~private')) {
    accessLabel = t('user_profile.menu.invite');
  } else {
    accessLabel = t('user_profile.list.release_public');
  }

  locationInstanceInfo.value = accessLabel;

  // Fetch world name
  locationWorldName.value = worldId; // fallback
  try {
    const cached = await DbApi.getApiCache({ key: `world_name:${worldId}` });
    if (cached) {
      locationWorldName.value = cached;
    }
    // Background fetch real name
    const world = await VrcApi.getWorld({ worldId });
    if (world?.name) {
      locationWorldName.value = world.name;
      DbApi.saveApiCache({ key: `world_name:${worldId}`, data: world.name }).catch(() => {});
      if (world.capacity) {
        locationPlayerCount.value = `${world.capacity}`;
      }
    }
  } catch {
    // keep worldId as fallback
  }

  // Fetch instance info (players in instance)
  if (worldId && instancePart) {
    isLoadingInstance.value = true;
    try {
      const instance = await VrcApi.getInstance({ worldId, instanceId: instancePart.split('~')[0] });
      if (instance) {
        if (instance.n_users !== undefined && instance.capacity) {
          locationPlayerCount.value = `${instance.n_users}/${instance.capacity}`;
        }
        // Get users in instance
        if (instance.users && Array.isArray(instance.users)) {
          instanceUsers.value = instance.users;
        } else {
          instanceUsers.value = [];
        }
      }
    } catch {
      instanceUsers.value = [];
    } finally {
      isLoadingInstance.value = false;
    }
  }
};

// Watch for profile changes to update location
watch(() => profileStore.baseInfo?.location, () => {
  if (profileStore.isOpen && profileStore.baseInfo?.location) {
    parseLocationInfo();
  }
}, { immediate: false });
onMounted(() => {
  document.addEventListener("click", closeMenus);
  loadFavGroups();
});
onUnmounted(() => document.removeEventListener("click", closeMenus));
watch(() => profileStore.isOpen, v => { if (v) { activeTab.value = "info"; currentAvatarName.value = ''; friendNumber.value = 0; parseLocationInfo(); fetchAvatarName(); fetchFriendNumber(); loadFriendStats(); loadRepresentedGroup(); loadHomeWorldName(); } });

// A6: 切到收藏世界 tab 时按需拉取（避免开 dialog 就同时打 5 个并发请求）
watch(activeTab, (tab) => {
  if (tab === 'fav_worlds' && profileStore.targetUserId && (profileStore as any).fetchFavoriteWorlds) {
    (profileStore as any).fetchFavoriteWorlds(profileStore.targetUserId);
  }
});
</script>

<template>
  <transition name="modal-fade">
    <div v-if="profileStore.isOpen" class="fixed inset-0 z-[100] flex items-center justify-center p-4" style="background: rgba(0,0,0,0.5); backdrop-filter: blur(4px);" @click.self="profileStore.closeProfile">
      <transition name="modal-scale">
        <div class="profile-panel flex flex-col w-full max-w-[900px] max-h-[88vh] rounded-xl overflow-hidden shadow-2xl" style="background: var(--theme-bg-main); border: 1px solid var(--theme-border-soft);">

          <!-- ── BREADCRUMB NAVIGATION ──────────────────────────── -->
          <div v-if="navHistory.length > 0" class="flex items-center gap-1 px-4 pt-3 pb-1 text-sm shrink-0" style="color: var(--theme-text-muted);">
            <button class="flex items-center gap-1 hover:opacity-80 transition-opacity" style="color: var(--theme-primary);" @click="navigateBack">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M19 12H5M12 19l-7-7 7-7"/></svg>
            </button>
            <template v-for="(item, idx) in navHistory" :key="idx">
              <button class="hover:underline truncate max-w-[120px]" style="color: var(--theme-primary);" @click="navigateTo(idx)">{{ item.displayName }}</button>
              <span style="color: var(--theme-text-muted);">›</span>
            </template>
            <span class="font-bold truncate max-w-[150px]" style="color: var(--theme-text-strong);">{{ profileStore.baseInfo?.displayName }}</span>
          </div>

          <!-- ── HEADER ─────────────────────────────────────────── -->
          <div class="flex gap-4 p-4 shrink-0" style="border-bottom: 1px solid var(--theme-border-soft);">
            <!-- Avatar -->
            <div class="shrink-0 cursor-pointer" @click="toggleImagePreview">
              <div class="relative" style="width:160px; height:120px; border-radius:12px; overflow:hidden; background: var(--theme-surface);">
                <img v-if="avatarImageUrl" :src="avatarImageUrl" class="w-full h-full object-cover" loading="lazy" />
                <div v-else class="w-full h-full flex items-center justify-center" style="color: var(--theme-text-muted);">
                  <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><path d="m21 15-5-5L5 21"/></svg>
                </div>
              </div>
            </div>

            <!-- Info -->
            <div class="flex-1 min-w-0 flex flex-col gap-1">
              <!-- Name row -->
              <div class="flex items-center gap-2 flex-wrap">
                <span
                  class="w-2.5 h-2.5 rounded-full shrink-0 cursor-default"
                  :style="{ background: statusColor }"
                  :title="statusTooltip"
                ></span>
                <!-- 曾用名下拉触发器 -->
                <div v-if="previousDisplayNames.length > 0" class="relative prev-names-wrap">
                  <button
                    class="text-xs px-1 py-0.5 rounded inline-flex items-center gap-0.5 hover:bg-[var(--theme-surface-hover)] transition-colors"
                    style="color: var(--theme-text-muted);"
                    :title="t('user_profile.info.previous_names')"
                    @click.stop="showPreviousNames = !showPreviousNames"
                  >
                    <ChevronDown :size="12" />
                  </button>
                  <transition name="dropdown">
                    <div v-if="showPreviousNames" class="dropdown-panel" style="left: 0; top: 26px; width: 240px; max-height: 280px; overflow-y: auto;">
                      <div class="dropdown-title">{{ t('user_profile.info.previous_names') }}</div>
                      <div
                        v-for="(name, idx) in previousDisplayNames"
                        :key="idx"
                        class="px-3 py-1.5 text-sm flex flex-col"
                        style="border-bottom: 1px solid var(--theme-border-soft);"
                      >
                        <span style="color: var(--theme-text);">{{ name.displayName }}</span>
                        <span v-if="name.updated_at" class="text-xs font-mono" style="color: var(--theme-text-muted);">— {{ fmtTime(name.updated_at) }}</span>
                      </div>
                    </div>
                  </transition>
                </div>
                <span class="font-bold text-lg leading-tight cursor-pointer" :style="{ color: trustInfo.color }" @click="executeAction('copy_id')">{{ profileStore.baseInfo?.displayName || '...' }}</span>
                <img
                  v-for="lang in userLanguageFlags"
                  :key="lang.code"
                  :src="`https://flagcdn.com/w40/${lang.cc}.png`"
                  :alt="lang.code"
                  :title="lang.code"
                  class="inline-block align-middle"
                  style="width: 22px; height: 16px; border-radius: 2px; object-fit: cover;"
                  loading="lazy"
                  @error="($event.target as HTMLImageElement).style.display = 'none'"
                >
                <span v-if="profileStore.baseInfo?.pronouns" class="text-xs font-mono" style="color: var(--theme-text-muted);">{{ profileStore.baseInfo.pronouns }}</span>
              </div>
              <!-- Username -->
              <div class="text-xs font-mono" style="color: var(--theme-text-muted);">{{ profileStore.baseInfo?.username || '' }}</div>
              <!-- Badges row -->
              <div class="flex items-center gap-1.5 flex-wrap">
                <!-- Trust 等级 -->
                <span class="badge" :style="{ color: trustInfo.color, borderColor: trustInfo.color + '60', background: trustInfo.color + '15' }">
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="inline mr-1"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
                  {{ trustInfo.label }}
                </span>
                <!-- 18+ 验证（基于 ageVerified 字段，VRCX 标准）-->
                <span v-if="ageVerificationLabel" class="badge" style="color:#3b82f6; border-color:#3b82f6; background:#3b82f615;" title="Age Verified">
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="inline mr-1"><rect x="3" y="4" width="18" height="16" rx="2"/><circle cx="9" cy="10" r="2"/><path d="M15 8h2M15 12h2M7 16h10"/></svg>
                  {{ ageVerificationLabel }}
                </span>
                <!-- 好友序号 (Friend Number) -->
                <span v-if="!isSelf && friendNumber > 0" class="badge" style="color:#fbbf24; border-color:#fbbf24; background:#fbbf2415;" :title="t('user_profile.info.friend_number')">
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="inline mr-1"><path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M19 8v6M22 11h-6"/></svg>
                  {{ friendNumber }}
                </span>
                <!-- 共同好友数 -->
                <span v-if="!isSelf && profileStore.mutualFriends.length > 0" class="badge" style="color:#a3a3a3; border-color:#a3a3a360; background:#a3a3a315;" :title="t('user_profile.info.mutual_friends_count')">
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="inline mr-1"><path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>
                  {{ profileStore.mutualFriends.length }}
                </span>
                <!-- Discord -->
                <span v-if="(profileStore.baseInfo as any)?.discordId" class="badge cursor-pointer" style="color:#7289da; border-color:#7289da; background:#7289da15;" @click="openDiscord((profileStore.baseInfo as any).discordId)" :title="t('user_profile.info.view_discord')">
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" class="inline mr-1"><path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028 14.09 14.09 0 0 0 1.226-1.994.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z"/></svg>
                  Discord
                </span>
                <!-- Nuisance / Probable Troll -->
                <span v-if="(profileStore.baseInfo?.tags || []).includes('system_troll')" class="badge" style="color:#ef4444; border-color:#ef4444; background:#ef444415;" title="Nuisance">
                  Nuisance
                </span>
                <span v-else-if="(profileStore.baseInfo?.tags || []).includes('system_probable_troll')" class="badge" style="color:#f97316; border-color:#f97316; background:#f9731615;" title="Almost Nuisance">
                  Almost Nuisance
                </span>
                <!-- VRChat Team -->
                <span v-if="isVrchatTeam" class="badge" style="color:#fbbf24; border-color:#fbbf24; background:#fbbf2415;" title="VRChat Team">
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="inline mr-1"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>
                  VRChat Team
                </span>
                <!-- Platform: PC -->
                <span v-if="userPlatform === 'pc'" class="badge" style="color:#60a5fa; border-color:#60a5fa; background:#60a5fa15;" title="PC">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4"/></svg>
                </span>
                <!-- Platform: Quest / Android -->
                <span v-else-if="userPlatform === 'android'" class="badge" style="color:#4ade80; border-color:#4ade80; background:#4ade8015;" title="Quest / Android">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="5" y="2" width="14" height="20" rx="2"/><line x1="12" y1="18" x2="12.01" y2="18"/></svg>
                </span>
                <!-- Platform: iOS -->
                <span v-else-if="userPlatform === 'ios'" class="badge" style="color:#a78bfa; border-color:#a78bfa; background:#a78bfa15;" title="iOS">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M17.05 20.28c-.98.95-2.05.8-3.08.35-1.09-.46-2.09-.48-3.24 0-1.44.62-2.2.44-3.06-.35C2.79 15.25 3.51 7.59 9.05 7.31c1.35.07 2.29.74 3.08.8 1.18-.24 2.31-.93 3.57-.84 1.51.12 2.65.72 3.4 1.8-3.12 1.87-2.38 5.98.48 7.13-.57 1.5-1.31 2.99-2.54 4.09zM12.03 7.25c-.15-2.23 1.66-4.07 3.74-4.25.29 2.58-2.34 4.5-3.74 4.25z"/></svg>
                </span>
                <span v-else-if="userPlatform" class="badge" style="color: var(--theme-text-muted); border-color: var(--theme-border-soft);">{{ userPlatform }}</span>
              </div>
              <!-- VRChat Badges (点击弹出 popover) -->
              <div v-if="(profileStore.baseInfo as any)?.badges?.length" class="flex items-center gap-1 flex-wrap mt-1">
                <div
                  v-for="badge in (profileStore.baseInfo as any).badges"
                  :key="badge.badgeId"
                  class="relative"
                >
                  <img
                    :src="badge.badgeImageUrl"
                    :title="badge.badgeName"
                    class="w-7 h-7 rounded object-cover cursor-pointer transition-all"
                    :class="{ grayscale: badge.hidden }"
                    loading="lazy"
                    @click.stop="openBadgePopover(badge.badgeId)"
                  >
                  <!-- Badge popover -->
                  <transition name="dropdown">
                    <div
                      v-if="openedBadgeId === badge.badgeId"
                      class="badge-popover"
                      @click.stop
                    >
                      <img
                        :src="badge.badgeImageUrl"
                        class="w-full rounded mb-2 cursor-pointer"
                        loading="lazy"
                      >
                      <div class="font-bold text-sm" style="color: var(--theme-text);">{{ badge.badgeName }}</div>
                      <div v-if="badge.badgeDescription" class="text-xs mt-1" style="color: var(--theme-text-muted);">{{ badge.badgeDescription }}</div>
                      <div v-if="badge.assignedAt" class="text-xs mt-2 font-mono" style="color: var(--theme-text-muted);">
                        {{ t('user_profile.badges.assigned') }}: {{ fmtTime(badge.assignedAt) }}
                      </div>
                      <template v-if="isSelf">
                        <label class="flex items-center gap-2 mt-3 cursor-pointer text-sm" style="color: var(--theme-text);">
                          <input
                            type="checkbox"
                            :checked="badge.hidden"
                            class="w-4 h-4 rounded accent-[var(--theme-primary)]"
                            @change="toggleBadgeHidden(badge, ($event.target as HTMLInputElement).checked)"
                          >
                          {{ t('user_profile.badges.hidden') }}
                        </label>
                        <label class="flex items-center gap-2 mt-1 cursor-pointer text-sm" style="color: var(--theme-text);">
                          <input
                            type="checkbox"
                            :checked="badge.showcased"
                            class="w-4 h-4 rounded accent-[var(--theme-primary)]"
                            @change="toggleBadgeShowcased(badge, ($event.target as HTMLInputElement).checked)"
                          >
                          {{ t('user_profile.badges.showcased') }}
                        </label>
                      </template>
                    </div>
                  </transition>
                </div>
              </div>
              <!-- Status description -->
              <div class="text-sm" style="color: var(--theme-text-soft);">{{ profileStore.baseInfo?.statusDescription || '' }}</div>
            </div>

            <!-- User icon + action buttons -->
            <div class="shrink-0 flex flex-col items-end gap-2">
              <div
                v-if="profileStore.baseInfo?.userIcon"
                class="cursor-pointer hover:opacity-80 transition-opacity"
                style="width:120px; height:120px; border-radius:12px; overflow:hidden; background: var(--theme-surface);"
                :title="t('user_profile.info.zoom_image')"
                @click="previewUserIcon"
              >
                <img :src="profileStore.baseInfo.userIcon" class="w-full h-full object-cover" loading="lazy" />
              </div>
              <div class="flex gap-1 mt-auto">
                <!-- Star / Favorite (查看自己时不显示) -->
                <div v-if="!isSelf" class="relative fav-menu-wrap">
                  <button class="icon-btn" :class="{ active: profileStore.isFavorite }" @click.stop="showFavPicker = !showFavPicker; if (showFavPicker) loadFavGroups()">
                    <Star :size="16" :fill="profileStore.isFavorite ? 'currentColor' : 'none'" />
                  </button>
                  <!-- Fav group picker -->
                  <transition name="dropdown">
                    <div v-if="showFavPicker" class="dropdown-panel" style="right:0; top:36px; width:280px;">
                      <div class="dropdown-title">{{ t("user_profile.groups.favorite") }}</div>
                      <div class="dropdown-section-label">{{ t('user_profile.info.online_favorites') }}</div>
                      <button v-for="g in favGroups" :key="g.id" class="dropdown-item w-full text-left" @click="addToFavGroup(g.id)">
                        {{ g.name }} ({{ g.count }} / {{ g.max }})
                      </button>
                      <div class="dropdown-section-label mt-2">{{ t('user_profile.info.local_favorites') }}</div>
                      <button class="dropdown-item w-full text-left" @click="addToFavGroup('local')">Favorites (0)</button>
                    </div>
                  </transition>
                </div>
                <!-- More -->
                <div class="relative more-menu-wrap">
                  <button class="icon-btn" @click.stop="showMoreMenu = !showMoreMenu">
                    <MoreHorizontal :size="16" />
                  </button>
                  <transition name="dropdown">
                    <div v-if="showMoreMenu" class="dropdown-panel" style="right:0; top:36px; width:240px;">
                      <!-- 共有项 -->
                      <button class="dropdown-item" @click="executeAction('refresh')"><RefreshCcw :size="14" class="mr-2"/>{{ t('user_profile.menu.refresh') }}</button>
                      <button class="dropdown-item" @click="executeAction('copy_url')"><Share2 :size="14" class="mr-2"/>{{ t('user_profile.menu.share') }}</button>

                      <!-- 看自己时的菜单（VRCX 对齐） -->
                      <template v-if="isSelf">
                        <button class="dropdown-item" @click="executeAction('show_avatar_info')"><User :size="14" class="mr-2"/>{{ t('user_profile.menu.show_avatar_info') }}</button>
                        <button class="dropdown-item" @click="executeAction('show_fallback_avatar_info')"><User :size="14" class="mr-2"/>{{ t('user_profile.menu.show_fallback_avatar_info') }}</button>
                        <div class="dropdown-divider"></div>
                        <button class="dropdown-item" @click="executeAction('edit_social_status')"><PencilLine :size="14" class="mr-2"/>{{ t('user_profile.menu.social_status') }}</button>
                        <button class="dropdown-item" @click="executeAction('edit_language')"><PencilLine :size="14" class="mr-2"/>{{ t('user_profile.menu.language') }}</button>
                        <button class="dropdown-item" @click="executeAction('edit_bio')"><PencilLine :size="14" class="mr-2"/>{{ t('user_profile.menu.bio') }}</button>
                        <button class="dropdown-item" @click="executeAction('edit_pronouns')"><PencilLine :size="14" class="mr-2"/>{{ t('user_profile.menu.pronouns') }}</button>
                        <button class="dropdown-item" @click="executeAction('edit_note_memo')"><PencilLine :size="14" class="mr-2"/>{{ t('user_profile.menu.note_memo') }}</button>
                      </template>

                      <!-- 看别人时的菜单（VRCX 对齐） -->
                      <template v-else>
                        <button class="dropdown-item" @click="executeAction('request_invite')"><LogIn :size="14" class="mr-2"/>{{ t('user_profile.menu.request_invite') }}</button>
                        <button class="dropdown-item" @click="executeAction('invite')"><Mail :size="14" class="mr-2"/>{{ t('user_profile.menu.invite') }}</button>
                        <button class="dropdown-item" @click="executeAction('invite_group')"><UsersRound :size="14" class="mr-2"/>{{ t('user_profile.menu.invite_group') }}</button>
                        <div class="dropdown-divider"></div>
                        <button class="dropdown-item" @click="executeAction('copy_id')"><Copy :size="14" class="mr-2"/>{{ t('user_profile.menu.copy_id') }}</button>
                        <button class="dropdown-item" @click="executeAction('view_vrc')"><ExternalLink :size="14" class="mr-2"/>{{ t('user_profile.menu.view_vrc') }}</button>
                        <div class="dropdown-divider"></div>
                        <button class="dropdown-item" @click="executeAction('show_avatar')"><Eye :size="14" class="mr-2"/>{{ t('user_profile.menu.show_avatar') }}</button>
                        <button class="dropdown-item" @click="executeAction('hide_avatar')"><EyeOff :size="14" class="mr-2"/>{{ t('user_profile.menu.hide_avatar') }}</button>
                        <button class="dropdown-item" @click="executeAction('mute')"><VolumeX :size="14" class="mr-2"/>{{ t('user_profile.menu.mute') }}</button>
                        <button class="dropdown-item" @click="executeAction('block')"><ShieldBan :size="14" class="mr-2"/>{{ t('user_profile.menu.block') }}</button>
                        <div class="dropdown-divider"></div>
                        <button class="dropdown-item danger" @click="executeAction('unfriend')"><UserMinus :size="14" class="mr-2"/>{{ t('user_profile.menu.unfriend') }}</button>
                      </template>
                    </div>
                  </transition>
                </div>
              </div>
            </div>

            <!-- Close -->
            <button class="icon-btn self-start ml-1" @click="profileStore.closeProfile"><X :size="16" /></button>
          </div>

          <!-- ── TABS ───────────────────────────────────────────── -->
          <div class="flex overflow-x-auto no-scrollbar shrink-0" style="border-bottom: 1px solid var(--theme-border-soft);">
            <button
              v-for="tab in tabs" :key="tab.id"
              class="tab-btn"
              :class="{ active: activeTab === tab.id }"
              @click="activeTab = tab.id"
            >{{ tab.label }}</button>
          </div>

          <!-- ── CONTENT ────────────────────────────────────────── -->
          <div class="flex-1 overflow-y-auto custom-scrollbar p-4">
            <!-- INFO TAB -->
            <template v-if="activeTab === 'info'">
              <div v-if="profileStore.isLoadingBase" class="flex justify-center py-8" style="color: var(--theme-primary);">
                <RefreshCcw class="animate-spin" :size="24" />
              </div>
              <div v-else class="space-y-0">
                <!-- Location (VRCX style: world name + instance info + players) -->
                <div v-if="locationWorldName" class="info-section mb-3 p-3 rounded-xl" style="background: var(--theme-surface); border: 1px solid var(--theme-border-soft);">
                  <div class="flex items-center gap-2 text-sm font-bold" style="color: var(--theme-text-strong);">
                    <MapPin :size="14" style="color: var(--theme-primary);" />
                    <span>{{ locationRegionFlag }}</span>
                    <span v-if="locationPlayerCount" style="color: var(--theme-text-muted);">{{ t('user_profile.info.players_count', { count: locationPlayerCount }) }}</span>
                    <span class="text-xs px-1.5 py-0.5 rounded" style="background: var(--theme-primary); color: white; opacity: 0.8;">{{ locationInstanceInfo }}</span>
                  </div>
                  <div class="mt-1 text-sm font-medium truncate" style="color: var(--theme-primary);">
                    {{ locationWorldName }}
                  </div>
                  <!-- Instance users -->
                  <div v-if="instanceUsers.length > 0" class="flex flex-wrap gap-2 mt-2 pt-2" style="border-top: 1px solid var(--theme-border-soft);">
                    <div v-for="u in instanceUsers.slice(0, 8)" :key="u.id" class="flex items-center gap-1.5 px-2 py-1 rounded-lg cursor-pointer hover:opacity-80" style="background: var(--theme-surface-hover);" @click="navigateToUser(u.id, u.displayName, u)">
                      <VrcAvatar :user="u" custom-class="w-5 h-5 rounded-full object-cover" />
                      <span class="text-xs font-medium truncate max-w-[80px]" style="color: var(--theme-text);">{{ u.displayName }}</span>
                    </div>
                    <span v-if="instanceUsers.length > 8" class="text-xs self-center" style="color: var(--theme-text-muted);">+{{ instanceUsers.length - 8 }}</span>
                  </div>
                  <div v-else-if="isLoadingInstance" class="mt-2 text-xs" style="color: var(--theme-text-muted);">
                    {{ t('user_profile.info.loading_room_players') }}
                  </div>
                </div>
                <div v-else-if="profileStore.baseInfo?.location === 'private'" class="info-section mb-3 p-3 rounded-xl" style="background: var(--theme-surface); border: 1px solid var(--theme-border-soft);">
                  <div class="flex items-center gap-2 text-sm" style="color: var(--theme-text-muted);">
                    <MapPin :size="14" />
                    <span>🔒 {{ t('user_profile.info.private_room') }}</span>
                  </div>
                </div>

                <!-- Online note (VRChat's note field) -->
                <div class="info-row">
                  <div class="info-label">{{ t('user_profile.info.online_note') }}</div>
                  <div class="info-value">{{ (profileStore.baseInfo as any)?.note || "—" }}</div>
                </div>

                <!-- Local note -->
                <div class="info-row">
                  <div class="info-label">{{ t('user_profile.info.local_note') }}</div>
                  <div class="info-value">
                    <div v-if="!isEditingNote" class="flex items-start gap-2">
                      <span class="flex-1 whitespace-pre-wrap">{{ localNote || "—" }}</span>
                      <button class="text-xs px-2 py-0.5 rounded" style="background: var(--theme-surface-hover); color: var(--theme-text-muted);" @click="isEditingNote = true">{{ t('user_profile.editor.edit') }}</button>
                    </div>
                    <div v-else class="space-y-2">
                      <textarea v-model="localNote" rows="3" class="w-full p-2 rounded text-sm resize-none" style="background: var(--theme-surface); border: 1px solid var(--theme-border-soft); color: var(--theme-text); outline: none;"></textarea>
                      <div class="flex gap-2">
                        <button class="text-xs px-3 py-1 rounded" style="background: var(--theme-primary); color: white;" @click="saveNote">{{ t('user_profile.editor.save') }}</button>
                        <button class="text-xs px-3 py-1 rounded" style="background: var(--theme-surface-hover); color: var(--theme-text-muted);" @click="isEditingNote = false">{{ t('user_profile.editor.cancel') }}</button>
                      </div>
                    </div>
                  </div>
                </div>

                <!-- Current avatar (fetched from avatar API) -->
                <div class="info-row">
                  <div class="info-label">{{ t('user_profile.info.current_avatar') }}</div>
                  <div class="info-value" style="color: var(--theme-primary);">
                    {{ currentAvatarName || t('user_profile.info.unknown_avatar') }}
                  </div>
                </div>

                <!-- Bio + 翻译按钮 -->
                <div class="info-row">
                  <div class="info-label flex items-center justify-between">
                    <span>{{ t('user_profile.info.bio') }}</span>
                    <button
                      v-if="profileStore.baseInfo?.bio"
                      class="icon-btn-sm"
                      :title="bioTranslation ? t('user_profile.info.hide_translation') : t('user_profile.info.translate')"
                      :disabled="bioTranslating"
                      @click="translateBio"
                    >
                      <Loader2 v-if="bioTranslating" :size="12" class="animate-spin" />
                      <Languages v-else :size="12" />
                    </button>
                  </div>
                  <div class="info-value whitespace-pre-wrap">{{ profileStore.baseInfo?.bio || "—" }}</div>
                  <div
                    v-if="bioTranslation"
                    class="info-value whitespace-pre-wrap mt-2 px-2 py-1.5 rounded text-sm"
                    style="background: var(--theme-surface-hover); border-left: 2px solid var(--theme-primary); color: var(--theme-text-soft);"
                  >
                    {{ bioTranslation }}
                  </div>
                  <!-- bioLinks favicon 列表 -->
                  <div v-if="bioLinksParsed.length > 0" class="flex items-center gap-1.5 mt-2 flex-wrap">
                    <a
                      v-for="(link, idx) in bioLinksParsed"
                      :key="idx"
                      :href="link"
                      target="_blank"
                      rel="noopener noreferrer"
                      class="inline-flex items-center hover:opacity-80 transition-opacity"
                      :title="link"
                    >
                      <img
                        :src="getFaviconUrl(link)"
                        class="w-4 h-4 rounded"
                        loading="lazy"
                        @error="($event.target as HTMLImageElement).style.display = 'none'"
                      >
                    </a>
                  </div>
                </div>

                <!-- Stats grid (VRCX 对齐：15 块信息) -->
                <div class="grid grid-cols-3 gap-x-4 gap-y-3 mt-3 pt-3" style="border-top: 1px solid var(--theme-border-soft);">
                  <!-- 看他人时显示 -->
                  <div v-if="!isSelf">
                    <div class="stat-label">{{ t('user_profile.info.last_seen') }}</div>
                    <div class="stat-value">{{ fmtTime(profileStore.baseInfo?.last_login) || "—" }}</div>
                  </div>
                  <div v-if="!isSelf">
                    <div class="stat-label">{{ t('user_profile.info.meeting_count') }}</div>
                    <div class="stat-value">{{ meetingCount > 0 ? meetingCount : '—' }}</div>
                  </div>
                  <div v-if="!isSelf">
                    <div class="stat-label">{{ t('user_profile.info.time_together') }}</div>
                    <div class="stat-value">{{ togetherTime > 0 ? timeToText(togetherTime) : '—' }}</div>
                  </div>

                  <!-- 在线/离线时长 -->
                  <div :title="fmtTime(profileStore.baseInfo?.last_login)">
                    <div class="stat-label">{{ onlineForLabel }}</div>
                    <div class="stat-value">{{ onlineForText }}</div>
                  </div>
                  <div :title="fmtTime((profileStore.baseInfo as any)?.last_activity)">
                    <div class="stat-label">{{ t('user_profile.info.last_activity') }}</div>
                    <div class="stat-value">{{ relativeTime((profileStore.baseInfo as any)?.last_activity || profileStore.baseInfo?.last_login) }}</div>
                  </div>
                  <div>
                    <div class="stat-label">{{ t('user_profile.info.account_created') }}</div>
                    <div class="stat-value">{{ fmt(profileStore.baseInfo?.date_joined) }}</div>
                  </div>

                  <!-- 加好友时间（仅看他人）-->
                  <div v-if="!isSelf">
                    <div class="stat-label">{{ t('user_profile.info.date_friended') }}</div>
                    <div class="stat-value">{{ dateFriended ? fmt(dateFriended) : '—' }}</div>
                  </div>

                  <!-- 模型克隆开关（自己可切，他人只读）-->
                  <div :class="{ 'cursor-pointer hover:opacity-80': isSelf }" @click="isSelf && executeAction('toggle_avatar_cloning')">
                    <div class="stat-label">{{ isSelf ? t('user_profile.info.avatar_cloning_toggle') : t('user_profile.info.avatar_cloning_allowed') }}</div>
                    <div class="stat-value">{{ profileStore.baseInfo?.allowAvatarCopying ? t('user_profile.info.allowed') : t('user_profile.info.not_allowed') }}</div>
                  </div>

                  <!-- 自己专属：boop / 共同好友 / Discord 好友 -->
                  <div v-if="isSelf" class="cursor-pointer hover:opacity-80" @click="executeAction('toggle_boop')">
                    <div class="stat-label">{{ t('user_profile.info.boop_toggle') }}</div>
                    <div class="stat-value">{{ (profileStore.baseInfo as any)?.isBoopingEnabled ? t('user_profile.info.allowed') : t('user_profile.info.not_allowed') }}</div>
                  </div>
                  <div v-if="isSelf" class="cursor-pointer hover:opacity-80" @click="executeAction('toggle_shared_connections')">
                    <div class="stat-label">{{ t('user_profile.info.shared_connections_toggle') }}</div>
                    <div class="stat-value">{{ !(profileStore.baseInfo as any)?.hasSharedConnectionsOptOut ? t('user_profile.info.allowed') : t('user_profile.info.not_allowed') }}</div>
                  </div>
                  <div v-if="isSelf" class="cursor-pointer hover:opacity-80" @click="executeAction('toggle_discord_friends')">
                    <div class="stat-label">{{ t('user_profile.info.discord_friends_toggle') }}</div>
                    <div class="stat-value">{{ !(profileStore.baseInfo as any)?.hasDiscordFriendsOptOut ? t('user_profile.info.allowed') : t('user_profile.info.not_allowed') }}</div>
                  </div>
                </div>

                <!-- 代表群组 -->
                <div v-if="representedGroup" class="info-row mt-3 pt-3" style="border-top: 1px solid var(--theme-border-soft);">
                  <div class="info-label">{{ t('user_profile.info.represented_group') }}</div>
                  <div class="info-value flex items-center gap-3 mt-1">
                    <img
                      v-if="representedGroup.iconUrl || representedGroup.thumbnailUrl"
                      :src="representedGroup.iconUrl || representedGroup.thumbnailUrl"
                      class="w-12 h-12 rounded-lg object-cover shrink-0"
                      style="background: var(--theme-surface);"
                      loading="lazy"
                      @error="($event.target as HTMLImageElement).style.display = 'none'"
                    >
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center gap-1">
                        <span v-if="representedGroup.ownerId === profileStore.targetUserId" class="shrink-0">👑</span>
                        <span class="font-medium truncate" style="color: var(--theme-text);">{{ representedGroup.name || '—' }}</span>
                      </div>
                      <div class="text-xs" style="color: var(--theme-text-muted);">
                        {{ t('user_profile.list.members', { count: representedGroup.memberCount || 0 }) }}
                      </div>
                    </div>
                  </div>
                </div>
                <div v-else-if="isLoadingRepresentedGroup" class="info-row mt-3 pt-3" style="border-top: 1px solid var(--theme-border-soft);">
                  <div class="info-label">{{ t('user_profile.info.represented_group') }}</div>
                  <div class="info-value text-xs" style="color: var(--theme-text-muted);">{{ t('user_profile.info.loading') }}</div>
                </div>

                <!-- 出生点（仅自己）-->
                <div v-if="isSelf && (profileStore.baseInfo as any)?.homeLocation" class="info-row mt-3 pt-3" style="border-top: 1px solid var(--theme-border-soft);">
                  <div class="info-label">{{ t('user_profile.info.home_location') }}</div>
                  <div class="info-value flex items-center gap-2 mt-1">
                    <span class="flex-1 truncate text-sm" style="color: var(--theme-text);">{{ homeWorldName || (profileStore.baseInfo as any).homeLocation }}</span>
                    <button
                      class="icon-btn-sm"
                      :title="t('user_profile.info.clear_home_location')"
                      @click="executeAction('reset_home_location')"
                    ><Trash2 :size="12" /></button>
                  </div>
                </div>

                <!-- User ID + 三选一复制下拉 -->
                <div class="mt-3 pt-3" style="border-top: 1px solid var(--theme-border-soft);">
                  <div class="stat-label">{{ t('user_profile.info.user_id') }}</div>
                  <div class="flex items-center gap-2 mt-1">
                    <span class="font-mono text-sm flex-1 truncate" style="color: var(--theme-text);">{{ profileStore.baseInfo?.id }}</span>
                    <div class="relative id-copy-wrap">
                      <button class="icon-btn-sm" :title="t('user_profile.menu.copy_id')" @click.stop="showIdCopyMenu = !showIdCopyMenu">
                        <Copy :size="12" />
                      </button>
                      <transition name="dropdown">
                        <div v-if="showIdCopyMenu" class="dropdown-panel" style="right: 0; top: 28px; width: 180px;">
                          <button class="dropdown-item" @click="copyById('id')"><Copy :size="12" class="mr-2"/>{{ t('user_profile.menu.copy_id') }}</button>
                          <button class="dropdown-item" @click="copyById('url')"><ExternalLink :size="12" class="mr-2"/>{{ t('user_profile.menu.copy_url') }}</button>
                          <button class="dropdown-item" @click="copyById('name')"><User :size="12" class="mr-2"/>{{ t('user_profile.menu.copy_name') }}</button>
                        </div>
                      </transition>
                    </div>
                  </div>
                </div>
              </div>
            </template>

            <!-- MUTUAL FRIENDS TAB -->
            <template v-else-if="activeTab === 'mutual'">
              <div class="flex items-center justify-between mb-3 gap-2 flex-wrap">
                <div class="flex items-center gap-2">
                  <button class="icon-btn-sm" :disabled="profileStore.isLoadingMutual" @click="profileStore.fetchMutualFriends(profileStore.targetUserId!)">
                    <RefreshCcw :size="14" :class="{ 'animate-spin': profileStore.isLoadingMutual }" />
                  </button>
                  <span class="text-sm" style="color: var(--theme-text-soft);">
                    <Users :size="13" class="inline mr-1" />{{ t('user_profile.list.total', { count: profileStore.mutualFriends.length }) }}
                  </span>
                </div>
                <div class="flex items-center gap-2">
                  <input v-model="mutualSearch" class="search-input" :placeholder="t('user_profile.list.search_friends')" />
                  <select v-model="mutualSort" class="search-input" style="width: auto; padding: 4px 8px;">
                    <option value="alphabetical">{{ t('user_profile.list.sort_alphabetical') }}</option>
                    <option value="lastActive">{{ t('user_profile.list.sort_last_active') }}</option>
                    <option value="friendOrder">{{ t('user_profile.list.sort_friend_order') }}</option>
                  </select>
                </div>
              </div>
              <div v-if="profileStore.isLoadingMutual" class="flex justify-center py-8" style="color: var(--theme-primary);"><RefreshCcw class="animate-spin" :size="20" /></div>
              <div v-else class="flex flex-wrap">
                <div v-for="f in filteredMutual" :key="f.id" class="friend-item" @click="navigateToUser(f.id, f.displayName, f)">
                  <div class="relative shrink-0">
                    <VrcAvatar :user="f" custom-class="w-9 h-9 rounded-full object-cover" />
                  </div>
                  <div class="flex-1 min-w-0">
                    <div class="truncate text-sm font-medium" :style="{ color: getFriendTrustColor(f.tags || []) }">{{ f.displayName }}</div>
                  </div>
                </div>
                <div v-if="filteredMutual.length === 0" class="w-full text-center py-8 text-sm" style="color: var(--theme-text-muted);">{{ t('user_profile.no_data') }}</div>
              </div>
            </template>

            <!-- GROUPS TAB (A5: 三段分组) -->
            <template v-else-if="activeTab === 'groups'">
              <div class="flex items-center justify-between mb-3 gap-2 flex-wrap">
                <div class="flex items-center gap-2">
                  <button class="icon-btn-sm" :disabled="profileStore.isLoadingGroups" @click="profileStore.fetchGroups(profileStore.targetUserId!)">
                    <RefreshCcw :size="14" :class="{ 'animate-spin': profileStore.isLoadingGroups }" />
                  </button>
                  <span class="text-sm" style="color: var(--theme-text-soft);">{{ t('user_profile.list.total', { count: groupedGroups.total }) }}</span>
                </div>
                <div class="flex items-center gap-2">
                  <input v-model="groupSearch" class="search-input" :placeholder="t('user_profile.list.search_groups')" />
                  <select v-model="groupSort" class="search-input" style="width: auto; padding: 4px 8px;">
                    <option value="alphabetical">{{ t('user_profile.list.sort_alphabetical') }}</option>
                    <option value="members">{{ t('user_profile.list.sort_members') }}</option>
                  </select>
                </div>
              </div>
              <div v-if="profileStore.isLoadingGroups" class="flex justify-center py-8" style="color: var(--theme-primary);"><RefreshCcw class="animate-spin" :size="20" /></div>
              <div v-else>
                <div v-if="groupedGroups.total === 0" class="text-center py-8 text-sm" style="color: var(--theme-text-muted);">{{ t('user_profile.no_data') }}</div>

                <!-- 拥有的群组 -->
                <template v-if="groupedGroups.own.length > 0">
                  <div class="text-base font-bold mt-2" style="color: var(--theme-text);">
                    {{ t('user_profile.list.owned_groups') }} <span class="text-xs ml-1" style="color: var(--theme-text-muted);">{{ groupedGroups.own.length }}</span>
                  </div>
                  <div class="flex flex-wrap mt-2 mb-3">
                    <div v-for="g in groupedGroups.own" :key="g.id" class="friend-item">
                      <div class="relative shrink-0">
                        <VrcAvatar :user="g" :url="g.iconUrl || g.thumbnailUrl" custom-class="w-9 h-9 rounded-lg object-cover" />
                        <span class="absolute -top-1 -right-1 text-xs">👑</span>
                      </div>
                      <div class="flex-1 min-w-0">
                        <div class="truncate text-sm font-medium" style="color: var(--theme-text);">{{ g.name }}</div>
                        <div class="text-xs" style="color: var(--theme-text-muted);">{{ t('user_profile.list.members', { count: g.memberCount || 0 }) }}</div>
                      </div>
                    </div>
                  </div>
                </template>

                <!-- 共同群组（仅看他人） -->
                <template v-if="!isSelf && groupedGroups.mutual.length > 0">
                  <div class="text-base font-bold mt-2" style="color: var(--theme-text);">
                    {{ t('user_profile.list.mutual_groups') }} <span class="text-xs ml-1" style="color: var(--theme-text-muted);">{{ groupedGroups.mutual.length }}</span>
                  </div>
                  <div class="flex flex-wrap mt-2 mb-3">
                    <div v-for="g in groupedGroups.mutual" :key="g.id" class="friend-item">
                      <VrcAvatar :user="g" :url="g.iconUrl || g.thumbnailUrl" custom-class="w-9 h-9 rounded-lg object-cover" />
                      <div class="flex-1 min-w-0">
                        <div class="truncate text-sm font-medium" style="color: var(--theme-text);">{{ g.name }}</div>
                        <div class="text-xs" style="color: var(--theme-text-muted);">{{ t('user_profile.list.members', { count: g.memberCount || 0 }) }}</div>
                      </div>
                    </div>
                  </div>
                </template>

                <!-- 其他群组 -->
                <template v-if="groupedGroups.other.length > 0">
                  <div class="text-base font-bold mt-2" style="color: var(--theme-text);">
                    {{ isSelf ? t('user_profile.list.my_groups') : t('user_profile.list.groups') }} <span class="text-xs ml-1" style="color: var(--theme-text-muted);">{{ groupedGroups.other.length }}</span>
                  </div>
                  <div class="flex flex-wrap mt-2">
                    <div v-for="g in groupedGroups.other" :key="g.id" class="friend-item">
                      <VrcAvatar :user="g" :url="g.iconUrl || g.thumbnailUrl" custom-class="w-9 h-9 rounded-lg object-cover" />
                      <div class="flex-1 min-w-0">
                        <div class="truncate text-sm font-medium" style="color: var(--theme-text);">{{ g.name }}</div>
                        <div class="text-xs" style="color: var(--theme-text-muted);">{{ t('user_profile.list.members', { count: g.memberCount || 0 }) }}</div>
                      </div>
                    </div>
                  </div>
                </template>
              </div>
            </template>

            <!-- CREATED WORLDS TAB (A6) -->
            <template v-else-if="activeTab === 'created_worlds'">
              <div class="flex items-center justify-between mb-3 gap-2 flex-wrap">
                <div class="flex items-center gap-2">
                  <button class="icon-btn-sm" :disabled="profileStore.isLoadingWorlds" @click="profileStore.fetchCreatedWorlds(profileStore.targetUserId!)">
                    <RefreshCcw :size="14" :class="{ 'animate-spin': profileStore.isLoadingWorlds }" />
                  </button>
                  <span class="text-sm" style="color: var(--theme-text-soft);">{{ t('user_profile.list.total', { count: profileStore.createdWorlds.length }) }}</span>
                </div>
                <div class="flex items-center gap-2 flex-wrap">
                  <input v-model="worldSearch" class="search-input" :placeholder="t('user_profile.list.search_worlds')" />
                  <span class="text-xs" style="color: var(--theme-text-muted);">{{ t('user_profile.list.sort_label') }}</span>
                  <select v-model="worldSort" class="search-input" style="width: auto; padding: 4px 8px;">
                    <option value="updated">{{ t('user_profile.list.sort_updated') }}</option>
                    <option value="created">{{ t('user_profile.list.sort_created') }}</option>
                    <option value="name">{{ t('user_profile.list.sort_name') }}</option>
                    <option value="visits">{{ t('user_profile.list.sort_visits') }}</option>
                    <option value="favorites">{{ t('user_profile.list.sort_favorites') }}</option>
                  </select>
                  <span class="text-xs" style="color: var(--theme-text-muted);">{{ t('user_profile.list.order_label') }}</span>
                  <select v-model="worldOrder" class="search-input" style="width: auto; padding: 4px 8px;">
                    <option value="desc">{{ t('user_profile.list.order_desc') }}</option>
                    <option value="asc">{{ t('user_profile.list.order_asc') }}</option>
                  </select>
                </div>
              </div>
              <div v-if="profileStore.isLoadingWorlds" class="flex justify-center py-8" style="color: var(--theme-primary);"><RefreshCcw class="animate-spin" :size="20" /></div>
              <div v-else-if="filteredWorlds.length === 0" class="w-full text-center py-8 text-sm" style="color: var(--theme-text-muted);">{{ t('user_profile.no_data') }}</div>
              <div v-else class="grid grid-cols-2 gap-3">
                <div
                  v-for="w in filteredWorlds"
                  :key="w.id"
                  class="rounded-xl overflow-hidden cursor-pointer hover:opacity-90 transition-all"
                  style="background: var(--theme-surface); border: 1px solid var(--theme-border-soft);"
                  @click="executeAction('open_world:' + w.id)"
                >
                  <div class="relative w-full" style="padding-bottom: 56.25%; background: var(--theme-surface-hover);">
                    <img v-if="(w as any).thumbnailImageUrl || (w as any).imageUrl" :src="(w as any).thumbnailImageUrl || (w as any).imageUrl" class="absolute inset-0 w-full h-full object-cover" loading="lazy" @error="($event.target as HTMLImageElement).style.display = 'none'">
                    <div class="absolute top-2 right-2 flex items-center gap-1">
                      <span v-if="(w as any).releaseStatus === 'private'" class="text-xs px-1.5 py-0.5 rounded font-medium" style="background: rgba(0,0,0,0.6); color: white;">{{ t('user_profile.list.release_private') }}</span>
                      <span v-else-if="(w as any).releaseStatus === 'public'" class="text-xs px-1.5 py-0.5 rounded font-medium" style="background: rgba(34,197,94,0.85); color: white;">{{ t('user_profile.list.release_public') }}</span>
                    </div>
                  </div>
                  <div class="p-2">
                    <div class="truncate text-sm font-medium" style="color: var(--theme-text);">{{ w.name }}</div>
                    <div class="flex items-center gap-2 mt-1 text-xs" style="color: var(--theme-text-muted);">
                      <span v-if="(w as any).visits != null">👁 {{ (w as any).visits }}</span>
                      <span v-if="(w as any).favorites != null">⭐ {{ (w as any).favorites }}</span>
                      <span v-if="(w as any).occupants != null">👥 {{ (w as any).occupants }}</span>
                    </div>
                  </div>
                </div>
              </div>
            </template>

            <!-- FAV WORLDS TAB (A6) -->
            <template v-else-if="activeTab === 'fav_worlds'">
              <div class="flex items-center justify-between mb-3 gap-2 flex-wrap">
                <div class="flex items-center gap-2">
                  <button class="icon-btn-sm" :disabled="(profileStore as any).isLoadingFavWorlds" @click="(profileStore as any).fetchFavoriteWorlds(profileStore.targetUserId!)">
                    <RefreshCcw :size="14" :class="{ 'animate-spin': (profileStore as any).isLoadingFavWorlds }" />
                  </button>
                  <span class="text-sm" style="color: var(--theme-text-soft);">{{ t('user_profile.list.total', { count: ((profileStore as any).favoriteWorlds || []).length }) }}</span>
                </div>
                <div class="flex items-center gap-2 flex-wrap">
                  <input v-model="worldSearch" class="search-input" :placeholder="t('user_profile.list.search_favorite_worlds')" />
                  <span class="text-xs" style="color: var(--theme-text-muted);">{{ t('user_profile.list.sort_label') }}</span>
                  <select v-model="favWorldSort" class="search-input" style="width: auto; padding: 4px 8px;">
                    <option value="updated">{{ t('user_profile.list.sort_updated') }}</option>
                    <option value="created">{{ t('user_profile.list.sort_created') }}</option>
                    <option value="name">{{ t('user_profile.list.sort_name') }}</option>
                  </select>
                  <span class="text-xs" style="color: var(--theme-text-muted);">{{ t('user_profile.list.order_label') }}</span>
                  <select v-model="favWorldOrder" class="search-input" style="width: auto; padding: 4px 8px;">
                    <option value="desc">{{ t('user_profile.list.order_desc') }}</option>
                    <option value="asc">{{ t('user_profile.list.order_asc') }}</option>
                  </select>
                </div>
              </div>
              <div v-if="(profileStore as any).isLoadingFavWorlds" class="flex justify-center py-8" style="color: var(--theme-primary);"><RefreshCcw class="animate-spin" :size="20" /></div>
              <div v-else-if="filteredFavoriteWorlds.length === 0" class="w-full text-center py-8 text-sm" style="color: var(--theme-text-muted);">
                {{ isSelf ? t('user_profile.list.no_favorite_worlds') : t('user_profile.list.favorite_worlds_private') }}
              </div>
              <div v-else class="grid grid-cols-2 gap-3">
                <div
                  v-for="w in filteredFavoriteWorlds"
                  :key="w.id"
                  class="rounded-xl overflow-hidden cursor-pointer hover:opacity-90 transition-all"
                  style="background: var(--theme-surface); border: 1px solid var(--theme-border-soft);"
                  @click="executeAction('open_world:' + w.id)"
                >
                  <div class="relative w-full" style="padding-bottom: 56.25%; background: var(--theme-surface-hover);">
                    <img v-if="(w as any).thumbnailImageUrl || (w as any).imageUrl" :src="(w as any).thumbnailImageUrl || (w as any).imageUrl" class="absolute inset-0 w-full h-full object-cover" loading="lazy" @error="($event.target as HTMLImageElement).style.display = 'none'">
                  </div>
                  <div class="p-2">
                    <div class="truncate text-sm font-medium" style="color: var(--theme-text);">{{ w.name }}</div>
                    <div v-if="(w as any).authorName" class="text-xs truncate" style="color: var(--theme-text-muted);">{{ (w as any).authorName }}</div>
                  </div>
                </div>
              </div>
            </template>

            <!-- CREATED AVATARS TAB (A6) -->
            <template v-else-if="activeTab === 'created_avatars'">
              <div class="flex items-center justify-between mb-3 gap-2 flex-wrap">
                <div class="flex items-center gap-2">
                  <button class="icon-btn-sm" :disabled="profileStore.isLoadingAvatars" @click="profileStore.fetchCreatedAvatars(profileStore.targetUserId!)">
                    <RefreshCcw :size="14" :class="{ 'animate-spin': profileStore.isLoadingAvatars }" />
                  </button>
                  <span class="text-sm" style="color: var(--theme-text-soft);">{{ t('user_profile.list.total', { count: profileStore.createdAvatars.length }) }}</span>
                </div>
                <div class="flex items-center gap-2 flex-wrap">
                  <input v-model="avatarSearch" class="search-input" :placeholder="t('user_profile.list.search_avatars')" />
                  <span class="text-xs" style="color: var(--theme-text-muted);">{{ t('user_profile.list.sort_label') }}</span>
                  <select v-model="avatarSort" class="search-input" style="width: auto; padding: 4px 8px;">
                    <option value="updated">{{ t('user_profile.list.sort_updated') }}</option>
                    <option value="created">{{ t('user_profile.list.sort_created') }}</option>
                    <option value="name">{{ t('user_profile.list.sort_name') }}</option>
                  </select>
                  <span class="text-xs" style="color: var(--theme-text-muted);">{{ t('user_profile.list.order_label') }}</span>
                  <select v-model="avatarOrder" class="search-input" style="width: auto; padding: 4px 8px;">
                    <option value="desc">{{ t('user_profile.list.order_desc') }}</option>
                    <option value="asc">{{ t('user_profile.list.order_asc') }}</option>
                  </select>
                </div>
              </div>
              <div v-if="profileStore.isLoadingAvatars" class="flex justify-center py-8" style="color: var(--theme-primary);"><RefreshCcw class="animate-spin" :size="20" /></div>
              <div v-else-if="filteredAvatars.length === 0" class="w-full text-center py-8 text-sm" style="color: var(--theme-text-muted);">{{ t('user_profile.no_data') }}</div>
              <div v-else class="grid grid-cols-2 gap-3">
                <div
                  v-for="a in filteredAvatars"
                  :key="a.id"
                  class="rounded-xl overflow-hidden cursor-pointer hover:opacity-90 transition-all"
                  style="background: var(--theme-surface); border: 1px solid var(--theme-border-soft);"
                  @click="executeAction('open_avatar:' + a.id)"
                >
                  <div class="relative w-full" style="padding-bottom: 75%; background: var(--theme-surface-hover);">
                    <img v-if="(a as any).thumbnailImageUrl || (a as any).imageUrl" :src="(a as any).thumbnailImageUrl || (a as any).imageUrl" class="absolute inset-0 w-full h-full object-cover" loading="lazy" @error="($event.target as HTMLImageElement).style.display = 'none'">
                    <div class="absolute top-2 right-2 flex items-center gap-1">
                      <span v-if="(a as any).releaseStatus === 'private'" class="text-xs px-1.5 py-0.5 rounded font-medium" style="background: rgba(0,0,0,0.6); color: white;">{{ t('user_profile.list.release_private') }}</span>
                      <span v-else-if="(a as any).releaseStatus === 'public'" class="text-xs px-1.5 py-0.5 rounded font-medium" style="background: rgba(34,197,94,0.85); color: white;">{{ t('user_profile.list.release_public') }}</span>
                    </div>
                  </div>
                  <div class="p-2">
                    <div class="truncate text-sm font-medium" style="color: var(--theme-text);">{{ a.name }}</div>
                    <div v-if="(a as any).description" class="text-xs truncate" style="color: var(--theme-text-muted);">{{ (a as any).description }}</div>
                  </div>
                </div>
              </div>
            </template>

            <!-- ACTIVITY TAB -->
            <template v-else-if="activeTab === 'activity'">
              <div class="flex items-center justify-between mb-3">
                <div class="flex items-center gap-2">
                  <button class="icon-btn-sm" @click="refreshActivityData">
                    <RefreshCcw :size="14" :class="{ 'animate-spin': profileStore.isLoadingActivity }" />
                  </button>
                  <span class="text-sm" style="color: var(--theme-text-soft);">{{ t('user_profile.activity.online_events', { count: activityEventCount }) }}</span>
                </div>
                <select v-model="activityPeriod" class="search-input" style="width: auto; padding: 4px 8px;" @change="refreshActivityData">
                  <option value="7">{{ t('user_profile.activity.period_7') }}</option>
                  <option value="30">{{ t('user_profile.activity.period_30') }}</option>
                  <option value="90">{{ t('user_profile.activity.period_90') }}</option>
                </select>
              </div>

              <!-- Peak stats -->
              <div v-if="activityPeakDay || activityPeakTime" class="flex gap-4 mb-3 text-sm">
                <div v-if="activityPeakDay">
                  <span style="color: var(--theme-text-muted);">{{ t('user_profile.activity.peak_day') }}</span>
                  <span class="font-bold ml-1" style="color: var(--theme-text);">{{ activityPeakDay }}</span>
                </div>
                <div v-if="activityPeakTime">
                  <span style="color: var(--theme-text-muted);">{{ t('user_profile.activity.peak_time') }}</span>
                  <span class="font-bold ml-1" style="color: var(--theme-text);">{{ activityPeakTime }}</span>
                </div>
              </div>

              <!-- Heatmap -->
              <div class="rounded-lg p-4 mb-4" style="background: var(--theme-surface); border: 1px solid var(--theme-border-soft);">
                <div class="grid gap-[2px]" style="grid-template-columns: 50px repeat(24, 1fr);">
                  <div v-for="(day, dayIdx) in heatmapDays" :key="dayIdx" class="contents">
                    <div class="text-xs flex items-center" style="color: var(--theme-text-muted); font-size: 10px;">{{ day }}</div>
                    <div
                      v-for="h in 24"
                      :key="h"
                      class="rounded-sm cursor-default transition-colors"
                      style="height: 16px;"
                      :style="{ background: getHeatmapColor(dayIdx, h - 1) }"
                      :title="t('user_profile.activity.online_minutes_title', { day, hour: String(h-1).padStart(2,'0'), count: heatmapData[dayIdx]?.[h-1] || 0 })"
                    ></div>
                  </div>
                </div>
                <div class="flex justify-between mt-2 text-xs" style="color: var(--theme-text-muted); padding-left: 50px;">
                  <span>00:00</span><span>03:00</span><span>06:00</span><span>09:00</span><span>12:00</span><span>15:00</span><span>18:00</span><span>21:00</span><span>24:00</span>
                </div>
              </div>

              <!-- Daily Playtime -->
              <div class="rounded-lg p-4 mb-4" style="background: var(--theme-surface); border: 1px solid var(--theme-border-soft);">
                <div class="flex items-center justify-between mb-3">
                  <span class="text-sm font-bold" style="color: var(--theme-text);">{{ t('user_profile.activity.daily_playtime') }}</span>
                  <span class="text-xs" style="color: var(--theme-text-muted);">{{ t('user_profile.activity.daily_average') }} <b style="color: var(--theme-text);">{{ avgDailyPlaytime }}</b></span>
                </div>
                <div class="relative" style="height: 120px;">
                  <!-- Y axis labels -->
                  <div class="absolute left-0 top-0 bottom-0 flex flex-col justify-between text-xs pr-2" style="color: var(--theme-text-muted); width: 30px;">
                    <span>24h</span><span>18h</span><span>12h</span><span>6h</span><span>0h</span>
                  </div>
                  <!-- Chart area -->
                  <div class="absolute left-[32px] right-0 top-0 bottom-0 flex items-end gap-[1px]">
                    <div
                      v-for="(hours, idx) in dailyPlaytimeData"
                      :key="idx"
                      class="flex-1 rounded-t-sm transition-all"
                      :style="{ height: `${(hours / 24) * 100}%`, background: hours > 0 ? 'var(--theme-primary)' : 'var(--theme-surface-hover)', opacity: hours > 0 ? 0.7 : 0.3 }"
                      :title="`${dailyPlaytimeLabels[idx]}: ${hours.toFixed(1)}h`"
                    ></div>
                  </div>
                </div>
                <div v-if="dailyPlaytimeLabels.length > 0" class="flex justify-between mt-1 text-xs" style="color: var(--theme-text-muted); padding-left: 32px;">
                  <span>{{ dailyPlaytimeLabels[0] }}</span>
                  <span>{{ dailyPlaytimeLabels[Math.floor(dailyPlaytimeLabels.length / 2)] }}</span>
                  <span>{{ dailyPlaytimeLabels[dailyPlaytimeLabels.length - 1] }}</span>
                </div>
              </div>

              <!-- Most Visited Worlds -->
              <div class="rounded-lg p-4" style="background: var(--theme-surface); border: 1px solid var(--theme-border-soft);">
                <div class="flex items-center justify-between mb-3">
                  <span class="text-sm font-bold" style="color: var(--theme-text);">{{ t('user_profile.activity.most_played_worlds') }}</span>
                  <div class="flex items-center gap-3">
                    <label class="flex items-center gap-1.5 text-xs cursor-pointer" style="color: var(--theme-text-muted);">
                      <input v-model="excludeHomeWorld" type="checkbox" class="w-3 h-3 rounded accent-[var(--theme-primary)]">
                      {{ t('user_profile.activity.exclude_home_world') }}
                    </label>
                    <select v-model="worldSortBy" class="search-input" style="width: auto; padding: 2px 6px; font-size: 12px;">
                      <option value="count">{{ t('user_profile.activity.sort_by_visits') }}</option>
                      <option value="time">{{ t('user_profile.activity.sort_by_time') }}</option>
                    </select>
                  </div>
                </div>
                <div v-if="topWorldsList.length > 0" class="space-y-2">
                  <div
                    v-for="(world, idx) in topWorldsList"
                    :key="world.id"
                    class="flex items-center gap-3 py-1.5 rounded-lg px-2 hover:opacity-80 cursor-pointer transition-opacity"
                  >
                    <span class="text-xs font-bold w-5 text-right shrink-0" :style="{ color: idx === 0 ? 'var(--theme-primary)' : 'var(--theme-text-muted)' }">#{{ idx + 1 }}</span>
                    <img
                      v-if="world.imageUrl"
                      :src="world.imageUrl"
                      class="w-8 h-8 rounded object-cover shrink-0"
                      style="background: var(--theme-surface-hover);"
                      @error="($event.target as HTMLImageElement).style.display = 'none'"
                    >
                    <div v-else class="w-8 h-8 rounded shrink-0 flex items-center justify-center" style="background: var(--theme-surface-hover);">
                      <Globe :size="14" style="color: var(--theme-text-muted);" />
                    </div>
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center justify-between gap-2">
                        <span class="text-sm font-medium truncate" style="color: var(--theme-text);">{{ world.name }}</span>
                        <span class="text-xs shrink-0" style="color: var(--theme-text-muted);">{{ t('user_profile.activity.visit_count', { count: world.visits }) }}</span>
                      </div>
                      <div class="mt-1 h-1.5 w-full rounded-full overflow-hidden" style="background: var(--theme-surface-hover);">
                        <div class="h-full rounded-full" style="background: var(--theme-text-muted); opacity: 0.5;" :style="{ width: getWorldBarWidth(world.visits) }"></div>
                      </div>
                    </div>
                  </div>
                </div>
                <div v-else class="text-center py-4 text-sm" style="color: var(--theme-text-muted);">{{ t('user_profile.activity.no_world_visits') }}</div>
              </div>
            </template>

            <!-- RAW JSON TAB (Collapsible Tree) -->
            <template v-else-if="activeTab === 'raw_json'">
              <div class="flex items-center gap-2 mb-3">
                <button class="icon-btn-sm" @click="profileStore.openProfile(profileStore.targetUserId!)"><RefreshCcw :size="14" /></button>
                <button class="icon-btn-sm" @click="copyRawJson"><Download :size="14" /></button>
              </div>
              <div class="rounded-lg p-3 overflow-auto font-mono text-xs" style="background: var(--theme-surface); border: 1px solid var(--theme-border-soft); max-height: 500px; color: var(--theme-text);">
                <JsonTree :data="profileStore.baseInfo" :depth="0" />
              </div>
            </template>

          </div><!-- end content -->
        </div>
      </transition>
    </div>
  </transition>

  <!-- A8a: 5 个编辑器对话框 -->
  <transition name="modal-fade">
    <div v-if="activeEditor" class="fixed inset-0 z-[180] flex items-center justify-center" style="background: rgba(0,0,0,0.6); backdrop-filter: blur(4px);" @click="closeEditor">
      <div
        class="rounded-2xl shadow-2xl flex flex-col"
        style="background: var(--theme-surface); border: 1px solid var(--theme-border-soft); width: 480px; max-width: 90vw; max-height: 85vh;"
        @click.stop
      >
        <!-- Header -->
        <div class="flex items-center justify-between p-4" style="border-bottom: 1px solid var(--theme-border-soft);">
          <h3 class="font-bold text-lg" style="color: var(--theme-text);">
            <template v-if="activeEditor === 'bio'">{{ t('user_profile.menu.bio') }}</template>
            <template v-else-if="activeEditor === 'note_memo'">{{ t('user_profile.menu.note_memo') }}</template>
            <template v-else-if="activeEditor === 'pronouns'">{{ t('user_profile.menu.pronouns') }}</template>
            <template v-else-if="activeEditor === 'social_status'">{{ t('user_profile.menu.social_status') }}</template>
            <template v-else-if="activeEditor === 'language'">{{ t('user_profile.menu.language') }}</template>
            <template v-else-if="activeEditor === 'invite_group'">{{ t('user_profile.menu.invite_group') }}</template>
            <template v-else-if="activeEditor === 'send_invite'">{{ t('user_profile.editor.send_invite_title') }}</template>
            <template v-else-if="activeEditor === 'send_invite_request'">{{ t('user_profile.editor.send_request_title') }}</template>
          </h3>
          <button class="icon-btn-sm" @click="closeEditor"><X :size="14" /></button>
        </div>

        <!-- Body -->
        <div class="flex-1 overflow-y-auto p-4 space-y-3">
          <!-- BIO -->
          <template v-if="activeEditor === 'bio'">
            <div>
              <label class="block text-xs font-medium mb-1" style="color: var(--theme-text-muted);">{{ t('user_profile.editor.bio_label', { count: bioDraft.length }) }}</label>
              <textarea
                v-model="bioDraft"
                rows="6"
                maxlength="512"
                class="w-full p-2 rounded text-sm resize-none"
                style="background: var(--theme-bg-main); border: 1px solid var(--theme-border-soft); color: var(--theme-text); outline: none;"
                :placeholder="t('user_profile.editor.bio_placeholder')"
              ></textarea>
            </div>
            <div>
              <label class="block text-xs font-medium mb-1" style="color: var(--theme-text-muted);">{{ t('user_profile.editor.links_label', { count: bioLinksDraft.length }) }}</label>
              <div class="space-y-1.5">
                <div v-for="(l, i) in bioLinksDraft" :key="i" class="flex items-center gap-2 p-2 rounded text-sm" style="background: var(--theme-surface-hover);">
                  <img :src="getFaviconUrl(l)" class="w-4 h-4 rounded shrink-0" @error="($event.target as HTMLImageElement).style.display = 'none'">
                  <span class="flex-1 truncate" style="color: var(--theme-text);">{{ l }}</span>
                  <button class="icon-btn-sm" @click="removeBioLink(i)"><Trash2 :size="12" /></button>
                </div>
                <div v-if="bioLinksDraft.length < 3" class="flex items-center gap-2">
                  <input
                    v-model="bioLinkInput"
                    type="text"
                    placeholder="https://example.com"
                    class="flex-1 p-2 rounded text-sm"
                    style="background: var(--theme-bg-main); border: 1px solid var(--theme-border-soft); color: var(--theme-text); outline: none;"
                    @keyup.enter="addBioLink"
                  >
                  <button
                    class="px-3 py-2 rounded text-sm"
                    style="background: var(--theme-primary); color: white;"
                    @click="addBioLink"
                  >{{ t('user_profile.editor.add') }}</button>
                </div>
              </div>
            </div>
          </template>

          <!-- NOTE + MEMO -->
          <template v-else-if="activeEditor === 'note_memo'">
            <div>
              <label class="block text-xs font-medium mb-1" style="color: var(--theme-text-muted);">{{ t('user_profile.editor.cloud_note_label') }}</label>
              <textarea
                v-model="noteDraft"
                rows="3"
                class="w-full p-2 rounded text-sm resize-none"
                style="background: var(--theme-bg-main); border: 1px solid var(--theme-border-soft); color: var(--theme-text); outline: none;"
                :placeholder="t('user_profile.editor.cloud_note_placeholder')"
              ></textarea>
            </div>
            <div>
              <label class="block text-xs font-medium mb-1" style="color: var(--theme-text-muted);">{{ t('user_profile.editor.local_memo_label') }}</label>
              <textarea
                v-model="memoDraft"
                rows="4"
                class="w-full p-2 rounded text-sm resize-none"
                style="background: var(--theme-bg-main); border: 1px solid var(--theme-border-soft); color: var(--theme-text); outline: none;"
                :placeholder="t('user_profile.editor.local_memo_placeholder')"
              ></textarea>
            </div>
          </template>

          <!-- PRONOUNS -->
          <template v-else-if="activeEditor === 'pronouns'">
            <div>
              <label class="block text-xs font-medium mb-1" style="color: var(--theme-text-muted);">{{ t('user_profile.editor.pronouns_label', { count: pronounsDraft.length }) }}</label>
              <input
                v-model="pronounsDraft"
                type="text"
                maxlength="32"
                class="w-full p-2 rounded text-sm"
                style="background: var(--theme-bg-main); border: 1px solid var(--theme-border-soft); color: var(--theme-text); outline: none;"
                :placeholder="t('user_profile.editor.pronouns_placeholder')"
              >
              <p class="text-xs mt-2" style="color: var(--theme-text-muted);">{{ t('user_profile.editor.pronouns_hint') }}</p>
            </div>
          </template>

          <!-- SOCIAL STATUS -->
          <template v-else-if="activeEditor === 'social_status'">
            <div>
              <label class="block text-xs font-medium mb-2" style="color: var(--theme-text-muted);">{{ t('user_profile.editor.status_label') }}</label>
              <div class="space-y-1.5">
                <label
                  v-for="opt in statusOptions"
                  :key="opt.value"
                  class="flex items-center gap-2 p-2 rounded cursor-pointer"
                  :style="{ background: socialStatusDraft === opt.value ? 'var(--theme-primary)' : 'var(--theme-surface-hover)', color: socialStatusDraft === opt.value ? 'white' : 'var(--theme-text)' }"
                >
                  <input
                    v-model="socialStatusDraft"
                    type="radio"
                    :value="opt.value"
                    class="accent-white"
                  >
                  <span class="text-sm font-medium">{{ opt.label }}</span>
                </label>
              </div>
            </div>
            <div>
              <label class="block text-xs font-medium mb-1" style="color: var(--theme-text-muted);">{{ t('user_profile.editor.status_desc_label', { count: statusDescDraft.length }) }}</label>
              <input
                v-model="statusDescDraft"
                type="text"
                maxlength="32"
                class="w-full p-2 rounded text-sm"
                style="background: var(--theme-bg-main); border: 1px solid var(--theme-border-soft); color: var(--theme-text); outline: none;"
                :placeholder="t('user_profile.editor.status_desc_placeholder')"
              >
            </div>
          </template>

          <!-- LANGUAGE -->
          <template v-else-if="activeEditor === 'language'">
            <p class="text-xs" style="color: var(--theme-text-muted);">{{ t('user_profile.editor.language_hint', { count: languagesDraft.length }) }}</p>
            <div class="grid grid-cols-2 gap-1.5">
              <button
                v-for="lang in languageOptions"
                :key="lang.code"
                class="flex items-center gap-2 p-2 rounded text-sm transition-colors"
                :style="{
                  background: languagesDraft.includes(lang.code) ? 'var(--theme-primary)' : 'var(--theme-surface-hover)',
                  color: languagesDraft.includes(lang.code) ? 'white' : 'var(--theme-text)',
                  border: '1px solid transparent',
                }"
                @click="toggleLanguage(lang.code)"
              >
                <Check :size="12" v-if="languagesDraft.includes(lang.code)" />
                <span class="flex-1 text-left">{{ lang.label }}</span>
                <span class="text-xs opacity-60">{{ lang.code }}</span>
              </button>
            </div>
          </template>

          <!-- GROUP INVITE -->
          <template v-else-if="activeEditor === 'invite_group'">
            <div class="rounded-lg p-3 text-sm" style="background: var(--theme-bg-main); border: 1px solid var(--theme-border-soft); color: var(--theme-text-muted);">
              {{ t('user_profile.editor.group_invite_hint', { user: profileStore.baseInfo?.displayName || '' }) }}
            </div>

            <div class="flex items-center justify-between">
              <label class="block text-xs font-medium" style="color: var(--theme-text-muted);">{{ t('user_profile.editor.invitable_groups') }}</label>
              <button
                class="px-2 py-1 rounded text-xs flex items-center gap-1"
                style="background: var(--theme-surface-hover); color: var(--theme-text);"
                :disabled="groupInviteLoading"
                @click="loadGroupInviteOptions"
              >
                <Loader2 v-if="groupInviteLoading" :size="12" class="animate-spin" />
                <RefreshCcw v-else :size="12" />
                {{ t('user_profile.editor.refresh') }}
              </button>
            </div>

            <input
              v-model="groupInviteSearch"
              class="w-full p-2 rounded text-sm"
              style="background: var(--theme-bg-main); border: 1px solid var(--theme-border-soft); color: var(--theme-text); outline: none;"
              :placeholder="t('user_profile.editor.search_groups_placeholder')"
            >

            <div v-if="groupInviteError" class="rounded-lg p-3 text-xs" style="background:#ef444415; color:#ef4444; border:1px solid #ef444455;">
              {{ groupInviteError }}
            </div>

            <div class="space-y-2">
              <button
                v-for="group in filteredGroupInviteOptions"
                :key="group.id"
                class="w-full p-3 rounded-lg text-left transition-colors flex items-center gap-3"
                :style="{
                  background: selectedGroupInviteId === group.id ? 'var(--theme-primary)' : 'var(--theme-surface-hover)',
                  color: selectedGroupInviteId === group.id ? 'white' : 'var(--theme-text)',
                  border: selectedGroupInviteId === group.id ? '1px solid var(--theme-primary)' : '1px solid var(--theme-border-soft)',
                }"
                @click="selectedGroupInviteId = group.id"
              >
                <img
                  v-if="group.iconUrl"
                  :src="group.iconUrl"
                  class="w-9 h-9 rounded object-cover shrink-0"
                  loading="lazy"
                >
                <div v-else class="w-9 h-9 rounded flex items-center justify-center shrink-0" style="background: var(--theme-bg-main);">
                  <UsersRound :size="16" />
                </div>
                <div class="min-w-0 flex-1">
                  <div class="font-semibold text-sm truncate">{{ group.name }}</div>
                  <div class="text-xs opacity-75 truncate">{{ group.shortCode || group.id }}</div>
                </div>
                <Check v-if="selectedGroupInviteId === group.id" :size="16" class="shrink-0" />
              </button>
            </div>
          </template>

          <!-- INVITE / REQUEST INVITE -->
          <template v-else-if="activeEditor === 'send_invite' || activeEditor === 'send_invite_request'">
            <div class="rounded-lg p-3 text-sm" style="background: var(--theme-bg-main); border: 1px solid var(--theme-border-soft); color: var(--theme-text-muted);">
              <template v-if="activeEditor === 'send_invite'">
                {{ t('user_profile.editor.invite_message_hint', { user: profileStore.baseInfo?.displayName || '' }) }}
              </template>
              <template v-else>
                {{ t('user_profile.editor.request_message_hint', { user: profileStore.baseInfo?.displayName || '' }) }}
              </template>
              <div v-if="activeEditor === 'send_invite'" class="mt-2 font-mono text-xs">
                {{ t('user_profile.editor.current_instance', { location: currentUserLocation || t('user_profile.info.unknown') }) }}
              </div>
              <div v-if="activeEditor === 'send_invite' && !inviteHasInstance" class="mt-2 text-xs" style="color:#f97316;">
                {{ t('user_profile.editor.not_invitable_warning') }}
              </div>
            </div>

            <div class="flex items-center justify-between">
              <label class="block text-xs font-medium" style="color: var(--theme-text-muted);">{{ t('user_profile.editor.message_slots') }}</label>
              <button
                class="px-2 py-1 rounded text-xs flex items-center gap-1"
                style="background: var(--theme-surface-hover); color: var(--theme-text);"
                :disabled="inviteMessagesLoading"
                @click="loadInviteMessageSlots(activeEditor)"
              >
                <Loader2 v-if="inviteMessagesLoading" :size="12" class="animate-spin" />
                <RefreshCcw v-else :size="12" />
                {{ t('user_profile.editor.refresh') }}
              </button>
            </div>

            <div v-if="inviteMessagesError" class="rounded-lg p-3 text-xs" style="background:#ef444415; color:#ef4444; border:1px solid #ef444455;">
              {{ t('user_profile.editor.message_slots_load_failed', { error: inviteMessagesError }) }}
            </div>

            <div class="space-y-2">
              <button
                v-for="row in inviteMessageSlots"
                :key="row.slot"
                class="w-full p-3 rounded-lg text-left transition-colors"
                :style="{
                  background: selectedInviteSlot === row.slot ? 'var(--theme-primary)' : 'var(--theme-surface-hover)',
                  color: selectedInviteSlot === row.slot ? 'white' : 'var(--theme-text)',
                  border: selectedInviteSlot === row.slot ? '1px solid var(--theme-primary)' : '1px solid var(--theme-border-soft)',
                }"
                @click="selectInviteMessageSlot(row)"
              >
                <div class="flex items-center justify-between gap-3">
                  <span class="font-semibold text-sm">{{ t('user_profile.editor.slot_label', { slot: row.slot + 1 }) }}</span>
                  <span class="text-xs opacity-75">{{ inviteSlotCooldownText(row) }}</span>
                </div>
                <div class="mt-1 text-xs opacity-85 line-clamp-2">
                  {{ row.message || t('user_profile.editor.empty_message') }}
                </div>
              </button>
            </div>

            <div>
              <label class="block text-xs font-medium mb-1" style="color: var(--theme-text-muted);">{{ t('user_profile.editor.edit_slot_label', { count: inviteMessageDraft.length }) }}</label>
              <textarea
                v-model="inviteMessageDraft"
                rows="3"
                maxlength="64"
                class="w-full p-2 rounded text-sm resize-none"
                style="background: var(--theme-bg-main); border: 1px solid var(--theme-border-soft); color: var(--theme-text); outline: none;"
                :placeholder="t('user_profile.editor.invite_message_placeholder')"
              ></textarea>
              <p class="text-xs mt-1" style="color: var(--theme-text-muted);">
                {{ t('user_profile.editor.invite_message_sync_hint') }}
              </p>
            </div>
          </template>
        </div>

        <!-- Footer -->
        <div class="flex justify-end gap-2 p-4" style="border-top: 1px solid var(--theme-border-soft);">
          <button
            class="px-4 py-2 rounded text-sm"
            style="background: var(--theme-surface-hover); color: var(--theme-text-muted);"
            :disabled="editorSaving"
            @click="closeEditor"
          >{{ t('user_profile.editor.cancel') }}</button>
          <button
            class="px-4 py-2 rounded text-sm font-medium flex items-center gap-1.5"
            style="background: var(--theme-primary); color: white;"
            :disabled="editorSubmitDisabled"
            @click="submitEditor"
          >
            <Loader2 v-if="editorSaving" :size="12" class="animate-spin" />
            <Mail v-else-if="activeEditor === 'send_invite' || activeEditor === 'send_invite_request'" :size="12" />
            <UsersRound v-else-if="activeEditor === 'invite_group'" :size="12" />
            <Save v-else :size="12" />
            <template v-if="activeEditor === 'send_invite'">{{ t('user_profile.editor.send_invite') }}</template>
            <template v-else-if="activeEditor === 'send_invite_request'">{{ t('user_profile.editor.send_request') }}</template>
            <template v-else-if="activeEditor === 'invite_group'">{{ t('user_profile.editor.send_group_invite') }}</template>
            <template v-else>{{ t('user_profile.editor.save') }}</template>
          </button>
        </div>
      </div>
    </div>
  </transition>

  <!-- Fullscreen image preview -->
  <transition name="modal-fade">
    <div v-if="showImagePreview" class="fixed inset-0 z-[200] flex flex-col items-center justify-center" style="background: rgba(0,0,0,0.92);" @click="toggleImagePreview">
      <div class="relative flex-1 flex items-center justify-center w-full overflow-hidden" @click.stop>
        <img :src="previewImageUrl || avatarImageUrl" class="max-w-[80vw] max-h-[70vh] object-contain transition-all duration-300" :style="{ transform: `scale(${imageScale}) rotate(${imageRotation}deg)` }" />
      </div>
      <div class="flex gap-3 p-4 mb-4 rounded-xl shrink-0" style="background: rgba(255,255,255,0.08); border: 1px solid rgba(255,255,255,0.15);">
        <button class="preview-btn" @click.stop="handleZoomOut"><ZoomOut :size="20" /></button>
        <button class="preview-btn" @click.stop="handleZoomIn"><ZoomIn :size="20" /></button>
        <button class="preview-btn" @click.stop="handleRotateCw"><RotateCw :size="20" /></button>
        <button class="preview-btn" @click.stop="handleResetImage"><RotateCcw :size="20" /></button>
        <button class="preview-btn" @click.stop="handleDownloadImage"><Download :size="20" /></button>
        <button class="preview-btn" @click.stop="toggleImagePreview"><X :size="20" /></button>
      </div>
    </div>
  </transition>
</template>

<style scoped>
/* ── Panel ─────────────────────────────────────────────────────── */
.profile-panel { color: var(--theme-text); }

/* ── Tabs ───────────────────────────────────────────────────────── */
.tab-btn {
  padding: 10px 16px;
  font-size: 13px;
  font-weight: 500;
  white-space: nowrap;
  color: var(--theme-text-muted);
  border-bottom: 2px solid transparent;
  transition: all 0.15s;
  background: transparent;
  cursor: pointer;
}
.tab-btn:hover { color: var(--theme-text); }
.tab-btn.active { color: var(--theme-primary); border-bottom-color: var(--theme-primary); }

/* ── Icon buttons ───────────────────────────────────────────────── */
.icon-btn {
  width: 32px; height: 32px;
  display: flex; align-items: center; justify-content: center;
  border-radius: 8px;
  background: var(--theme-surface);
  border: 1px solid var(--theme-border-soft);
  color: var(--theme-text-muted);
  cursor: pointer;
  transition: all 0.15s;
}
.icon-btn:hover { background: var(--theme-surface-hover); color: var(--theme-text); }
.icon-btn.active { color: #f59e0b; }

.icon-btn-sm {
  width: 24px; height: 24px;
  display: inline-flex; align-items: center; justify-content: center;
  border-radius: 6px;
  background: var(--theme-surface);
  border: 1px solid var(--theme-border-soft);
  color: var(--theme-text-muted);
  cursor: pointer;
  transition: all 0.15s;
}
.icon-btn-sm:hover { background: var(--theme-surface-hover); color: var(--theme-text); }
.icon-btn-sm:disabled { opacity: 0.4; cursor: not-allowed; }

/* ── Badge ──────────────────────────────────────────────────────── */
.badge {
  display: inline-flex; align-items: center;
  padding: 2px 8px;
  border-radius: 6px;
  border: 1px solid;
  font-size: 11px;
  font-weight: 600;
  line-height: 1.4;
}

/* ── Dropdown ───────────────────────────────────────────────────── */
.dropdown-panel {
  position: absolute;
  z-index: 9999;
  background: var(--theme-bg-main);
  border: 1.5px solid var(--theme-border-strong);
  border-radius: 10px;
  box-shadow: 0 12px 40px rgba(0,0,0,0.28), 0 2px 8px rgba(0,0,0,0.12);
  padding: 6px;
  min-width: 160px;
  opacity: 1 !important;
  pointer-events: auto !important;
}
.dropdown-title {
  padding: 6px 10px 4px;
  font-size: 11px;
  font-weight: 700;
  color: var(--theme-text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.dropdown-section-label {
  padding: 4px 10px 2px;
  font-size: 11px;
  color: var(--theme-text-muted);
}
.dropdown-item {
  display: flex; align-items: center;
  width: 100%;
  padding: 7px 10px;
  border-radius: 6px;
  font-size: 13px;
  color: var(--theme-text);
  cursor: pointer;
  transition: background 0.1s;
  background: transparent;
  text-align: left;
  pointer-events: auto !important;
}
.dropdown-item:hover { background: var(--theme-surface-hover); color: var(--theme-text-strong); }
.dropdown-item.danger { color: #ef4444; }
.dropdown-item.danger:hover { background: rgba(239,68,68,0.1); }
.dropdown-divider { height: 1px; background: var(--theme-border-soft); margin: 4px 0; }

/* ── Info rows ──────────────────────────────────────────────────── */
.info-section { padding: 8px 0; border-bottom: 1px solid var(--theme-border-soft); }
.info-row { display: flex; gap: 16px; padding: 6px 0; border-bottom: 1px solid var(--theme-border-soft); }
.info-label { width: 120px; shrink: 0; font-size: 13px; font-weight: 600; color: var(--theme-text-muted); }
.info-value { flex: 1; font-size: 13px; color: var(--theme-text); }

/* ── Stats ──────────────────────────────────────────────────────── */
.stat-label { font-size: 11px; color: var(--theme-text-muted); margin-bottom: 2px; }
.stat-value { font-size: 13px; color: var(--theme-text); }

/* ── Friend/World/Avatar items ──────────────────────────────────── */
.friend-item {
  display: flex; align-items: center; gap: 8px;
  padding: 6px 8px;
  width: 167px;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.1s;
}
.friend-item:hover { background: var(--theme-surface-hover); border-radius: 20px 6px 6px 20px; }

/* ── Search input ───────────────────────────────────────────────── */
.search-input {
  height: 30px;
  padding: 0 10px;
  border-radius: 6px;
  font-size: 12px;
  background: var(--theme-surface);
  border: 1px solid var(--theme-border-soft);
  color: var(--theme-text);
  outline: none;
  width: 160px;
}
.search-input:focus { border-color: var(--theme-primary); }

/* ── Preview buttons ────────────────────────────────────────────── */
.preview-btn {
  width: 40px; height: 40px;
  display: flex; align-items: center; justify-content: center;
  border-radius: 10px;
  color: rgba(255,255,255,0.7);
  cursor: pointer;
  transition: all 0.15s;
  background: transparent;
}
.preview-btn:hover { background: rgba(255,255,255,0.1); color: white; }

/* ── JSON highlight ─────────────────────────────────────────────── */
:deep(.json-key) { color: var(--theme-text-muted); }
:deep(.json-str) { color: var(--theme-primary); }
:deep(.json-num) { color: #60a5fa; }
:deep(.json-bool) { color: #34d399; }
:deep(.json-null) { color: #f87171; }

/* ── Transitions ────────────────────────────────────────────────── */
.modal-fade-enter-active, .modal-fade-leave-active { transition: opacity 0.25s ease; }
.modal-fade-enter-from, .modal-fade-leave-to { opacity: 0; }
.modal-scale-enter-active, .modal-scale-leave-active { transition: all 0.3s cubic-bezier(0.16,1,0.3,1); }
.modal-scale-enter-from, .modal-scale-leave-to { opacity: 0; transform: scale(0.95) translateY(10px); }
.dropdown-enter-active, .dropdown-leave-active { transition: all 0.15s ease; }
.dropdown-enter-from, .dropdown-leave-to { opacity: 0; transform: translateY(-6px) scale(0.97); }

.no-scrollbar::-webkit-scrollbar { display: none; }
.no-scrollbar { -ms-overflow-style: none; scrollbar-width: none; }

/* ── Badge Popover (VRCX 对齐) ────────────────────────────────── */
.badge-popover {
  position: absolute;
  top: 36px;
  left: 0;
  z-index: 9999;
  width: 240px;
  padding: 12px;
  background: var(--theme-surface);
  border: 1px solid var(--theme-border-soft);
  border-radius: 12px;
  box-shadow: 0 10px 32px rgba(0, 0, 0, 0.45);
  animation: badge-pop 0.18s ease-out;
}
@keyframes badge-pop {
  from { opacity: 0; transform: translateY(-4px) scale(0.96); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}
.badge-popover img.cursor-pointer { aspect-ratio: 16 / 9; object-fit: cover; }
.grayscale { filter: grayscale(1); }
</style>
