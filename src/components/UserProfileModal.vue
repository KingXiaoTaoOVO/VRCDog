<script setup lang="ts">
import { useUserProfileStore } from '../stores/userProfile';
import { useToast } from '../composables/useToast';

import { VrcApi } from '../api';

const toast = useToast();

const executeAction = async (action: string) => {
  const userId = profileStore.baseInfo?.id;
  if (!userId) return;
  try {
    switch (action) {
      
      case 'refresh':
        profileStore.openProfile(userId);
        toast.success(t('auto_0d221546'));
        break;
      case 'share':
        navigator.clipboard.writeText('https://vrchat.com/home/user/' + userId);
        toast.success(t('auto_d9ddf077'));
        break;
      case 'show_avatar_info':
        toast.info(t('auto_d39a196f') + (profileStore.baseInfo?.currentAvatarImageUrl ? t('auto_b4156a8e') : t('auto_dce5379c')));
        break;
      case 'show_history':
        toast.info(t('auto_cc8945d3'));
        break;
      case 'hide_chat':
        toast.success(t('auto_856f5782'));
        break;
      case 'disable_interaction':
        toast.success(t('auto_e4954af9'));
        break;
      case 'report':
        toast.warning(t('auto_71739124'));
        break;
      case 'request_invite':
        await VrcApi.requestInvite(userId);
        toast.success(t('auto_faff6dcd'));
        break;
      case 'invite':
        await VrcApi.inviteUser(userId);
        toast.success(t('auto_d785bab0'));
        break;
      case 'unfriend':
        if (confirm(t('auto_1a0a24f7'))) {
          await VrcApi.unfriend(userId);
          toast.success(t('auto_bd5a56e1'));
          profileStore.closeProfile();
        }
        break;
      case 'block':
        await VrcApi.moderateUser({ moderated: userId, type: 'block' });
        toast.success(t('auto_442f9153'));
        break;
      case 'mute':
        await VrcApi.moderateUser({ moderated: userId, type: 'mute' });
        toast.success(t('auto_383cc43c'));
        break;
      case 'showAvatar':
        await VrcApi.moderateUser({ moderated: userId, type: 'showAvatar' });
        toast.success(t('auto_ade50481'));
        break;
      case 'hideAvatar':
        await VrcApi.moderateUser({ moderated: userId, type: 'hideAvatar' });
        toast.success(t('auto_1e539938'));
        break;
      case 'favorite':
        // Not a full impl, but works as placeholder
        if (isFavorite.value) {
           await VrcApi.request(`/favorites/${userId}`, 'DELETE');
        } else {
           await VrcApi.request('/favorites', 'POST', { type: 'friend', favoriteId: userId, tags: ['group_0'] });
        }
        toast.success(isFavorite.value ? t('auto_3a906c71') : t('auto_934ebd87'));
        isFavorite.value = !isFavorite.value;
        break;
    }
  } catch (e: any) {
    toast.error(t('auto_be4a600c') + e.message);
  }
};

const isFavorite = ref(false);
const showFavoriteModal = ref(false);

const addFavoriteToGroup = async (group: string) => {
  try {
    await VrcApi.request('/favorites', 'POST', { type: 'friend', favoriteId: profileStore.baseInfo?.id, tags: [group] });
    toast.success(t('auto_281fadcf') + group);
    isFavorite.value = true;
    showFavoriteModal.value = false;
  } catch (err: any) {
    toast.error(t('auto_93e2fd20') + err.message);
  }
};
 // TODO: fetch from favorites api


import { 
  X, MoreHorizontal, Star, Monitor, Headset, Copy, RefreshCcw, Share2, 
  LogIn, Mail, Hand, MessageSquarePlus, Settings, PenLine, User, Users, 
  History, Ban, VolumeX, MessageSquareOff, Eye, EyeOff, ShieldBan, Flag, 
  UserMinus, Search, ChevronDown, Download, Check, ZoomIn, ZoomOut, RotateCcw, RotateCw, Globe, PencilLine, Save, Languages, Image, Heart, Map, Cuboid, LogOut
} from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { useEntityModalStore } from '../stores/entityModal';
import { computed, ref, onMounted, onUnmounted } from 'vue';

const { t } = useI18n();
const profileStore = useUserProfileStore();
const entityStore = useEntityModalStore();

const activeTab = ref<'info' | 'mutual' | 'groups' | 'created_worlds' | 'fav_worlds' | 'created_avatars' | 'activity' | 'raw_json'>('info');
const showMoreMenu = ref(false);
const showSortMenu = ref(false);
const sortOption = ref(t('auto_078e09ab'));
const localNote = ref('');
const isEditingNote = ref(false);
const translatedBio = ref('');
const isTranslating = ref(false);

const handleTranslate = async () => {
  if (translatedBio.value) {
    translatedBio.value = '';
    return;
  }
  isTranslating.value = true;
  await new Promise(r => setTimeout(r, 600));
  translatedBio.value = t('auto_217b1165') + (profileStore.baseInfo?.bio || '');
  isTranslating.value = false;
};

const saveLocalNote = () => {
  isEditingNote.value = false;
  // TODO: Save note to local db
};


const showImagePreview = ref(false);
const imageScale = ref(1);
const imageRotation = ref(0);

const toggleImagePreview = () => {
  showImagePreview.value = !showImagePreview.value;
  imageScale.value = 1;
  imageRotation.value = 0;
};

const handleZoomIn = () => { imageScale.value = Math.min(imageScale.value + 0.25, 3); };
const handleZoomOut = () => { imageScale.value = Math.max(imageScale.value - 0.25, 0.5); };
const handleRotateCw = () => { imageRotation.value += 90; };
const handleRotateCcw = () => { imageRotation.value -= 90; };
const handleResetImage = () => { imageScale.value = 1; imageRotation.value = 0; };
const handleCopyImage = async () => {
  try {
     const url = profileStore.baseInfo?.profilePicOverride || profileStore.baseInfo?.currentAvatarImageUrl || profileStore.baseInfo?.currentAvatarThumbnailImageUrl;
     if (!url) return;
     const response = await fetch(url);
     const blob = await response.blob();
     await navigator.clipboard.write([new ClipboardItem({ [blob.type]: blob })]);
     toast.success(t('auto_4fb42e6e'));
  } catch(e) {
     toast.error(t('auto_e41af23d'));
  }
};
const handleDownloadImage = () => {
   const url = profileStore.baseInfo?.profilePicOverride || profileStore.baseInfo?.currentAvatarImageUrl || profileStore.baseInfo?.currentAvatarThumbnailImageUrl;
   if (!url) return;
   const a = document.createElement('a');
   a.href = url;
   a.download = `avatar_${profileStore.baseInfo?.displayName || 'unknown'}.png`;
   document.body.appendChild(a);
   a.click();
   document.body.removeChild(a);
};

const closeMenus = (e: MouseEvent) => {
  const target = e.target as HTMLElement;
  if (!target.closest('.more-menu-container')) {
    showMoreMenu.value = false;
  }
  if (!target.closest('.sort-menu-container')) {
    showSortMenu.value = false;
  }
};

onMounted(() => {
  document.addEventListener('click', closeMenus);
});

onUnmounted(() => {
  document.removeEventListener('click', closeMenus);
});

const isSelf = computed(() => profileStore.baseInfo?.id === profileStore.myId);

const trustColor = computed(() => {
  if (!profileStore.baseInfo?.tags) return '#94a3b8'; // slate-400
  const tags = profileStore.baseInfo.tags;
  if (tags.includes('system_trust_veteran') || tags.includes('system_trust_legend')) return '#a855f7'; // purple-500
  if (tags.includes('system_trust_trusted')) return '#f97316'; // orange-500
  if (tags.includes('system_trust_known')) return '#22c55e'; // green-500
  if (tags.includes('system_trust_basic')) return '#3b82f6'; // blue-500
  return '#94a3b8';
});

const trustName = computed(() => {
  if (!profileStore.baseInfo?.tags) return 'Visitor';
  const tags = profileStore.baseInfo.tags;
  if (tags.includes('system_trust_veteran') || tags.includes('system_trust_legend')) return 'Trusted User';
  if (tags.includes('system_trust_trusted')) return 'Known User';
  if (tags.includes('system_trust_known')) return 'User';
  if (tags.includes('system_trust_basic')) return 'New User';
  return 'Visitor';
});

const getStatusColor = (status: string) => {
  switch (status) {
    case 'active': return '#22c55e';
    case 'join me': return '#3b82f6';
    case 'ask me': return '#f97316';
    case 'busy': return '#ef4444';
    default: return '#64748b';
  }
};

const copyToClipboard = (text: string) => {
  navigator.clipboard.writeText(text);
};

const formatTime = (time: string | undefined) => {
  if (!time) return '-';
  const d = new Date(time);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}:${String(d.getSeconds()).padStart(2, '0')}`;
};

const formatDate = (time: string | undefined) => {
  if (!time) return '-';
  const d = new Date(time);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
};

const resolvedWorldName = ref('');
const resolvedWorldThumbnail = ref('');

// Watch location changes to fetch world data asynchronously
import { watch } from 'vue';
watch(() => profileStore.baseInfo?.location, (loc) => {
  if (!loc || loc === 'offline' || loc === 'private') return;
  const displayLoc = loc.split('~')[0];
  if (displayLoc.startsWith('wrld_')) {
    const worldId = displayLoc.split(':')[0];
    VrcApi.request('/api/1/worlds/' + worldId).then((res) => {
       if (res && res.name) {
          resolvedWorldName.value = res.name;
          if (res.thumbnailImageUrl) resolvedWorldThumbnail.value = res.thumbnailImageUrl;
       }
    }).catch((e) => console.error(e));
  }
}, { immediate: true });

const locationParsed = computed(() => {
  const loc = profileStore.baseInfo?.location;
  if (!loc || loc === 'offline') return null;
  if (loc === 'private') return { name: t('auto_4ad3ada9'), raw: loc, flag: '🔒' };
  
  let flag = '🌐';
  if (loc.includes('JP')) flag = '🇯🇵';
  else if (loc.includes('US')) flag = '🇺🇸';
  else if (loc.includes('CN')) flag = '🇨🇳';
  else if (loc.includes('EU')) flag = '🇪🇺';
  else if (loc.includes('KR')) flag = '🇰🇷';

  const displayLoc = loc.split('~')[0];
  
  return {
    name: displayLoc,
    flag,
    raw: loc
  };
});

// Calculate mock active duration
const getMockDuration = () => {
  if (profileStore.baseInfo?.status === 'offline') return t('auto_cb4049ba');
  return t('auto_2e8f1c81');
};

// Syntax Highlighting for JSON
const highlightJson = (obj: any) => {
  if (!obj) return '{}';
  let json = JSON.stringify(obj, null, 2);
  json = json.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  return json.replace(/("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g, function (match) {
    let cls = 'text-blue-400'; // number
    if (/^"/.test(match)) {
      if (/:$/.test(match)) {
        cls = 'text-text-muted'; // key
      } else {
        cls = 'text-green-400'; // string
      }
    } else if (/true|false/.test(match)) {
      cls = 'text-blue-500'; // boolean
    } else if (/null/.test(match)) {
      cls = 'text-pink-500'; // null
    }
    return '<span class="' + cls + '">' + match + '</span>';
  });
};

</script>

<template>
  <transition name="fade">
    <div
      v-if="profileStore.isOpen"
      class="fixed inset-0 bg-surface0/40 z-[100] flex items-center justify-center p-4 backdrop-blur-md"
      @click.self="profileStore.closeProfile"
    >
      <!-- 主弹窗 - VRCX风格毛玻璃效果 -->
      <transition name="scale">
        <div
          v-if="profileStore.isOpen"
          class="bg-surface backdrop-blur-3xl border border-white/60 shadow-[0_0_50px_rgba(0,0,0,0.15)] rounded-3xl w-full max-w-[950px] max-h-[90vh] flex flex-col overflow-hidden text-text"
        >
          <!-- 头部信息 Banner与基础信息区域合并 -->
          <div class="relative shrink-0 border-b border-border-soft0/50 bg-surface z-20">
            <!-- Banner Background -->
            <div class="absolute inset-0 z-0 h-40 overflow-hidden">
              <div class="absolute inset-0 bg-gradient-to-b from-white/10 via-white/50 to-white/95 z-10"></div>
              <img 
                v-if="profileStore.baseInfo?.profilePicOverride || profileStore.baseInfo?.currentAvatarThumbnailImageUrl"
                :src="profileStore.baseInfo?.profilePicOverride || profileStore.baseInfo?.currentAvatarThumbnailImageUrl"
                class="w-full h-full object-cover filter blur-[2px] opacity-70"
              />
            </div>

            <div class="relative z-10 px-8 pt-16 pb-4">
              <div class="flex gap-6 items-end">
                <!-- 头像 -->
                <div 
                  class="w-40 h-40 shrink-0 bg-background/10 rounded-2xl flex items-center justify-center overflow-hidden border-[4px] border-border-strong cursor-pointer group relative shadow-xl z-20"
                  @click="toggleImagePreview"
                >
                  <img 
                    v-if="profileStore.baseInfo?.profilePicOverride || profileStore.baseInfo?.currentAvatarThumbnailImageUrl || profileStore.baseInfo?.currentAvatarImageUrl"
                    :src="profileStore.baseInfo?.profilePicOverride || profileStore.baseInfo?.currentAvatarThumbnailImageUrl || profileStore.baseInfo?.currentAvatarImageUrl"
                    class="w-full h-full object-cover transition-transform duration-500 group-hover:scale-110"
                  />
                  <div class="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center backdrop-blur-sm">
                    <Search class="w-8 h-8 text-text-muted0/80" />
                  </div>
                  <!-- Status Indicator on Avatar -->
                  <div class="absolute bottom-1 right-1 w-6 h-6 rounded-full border-[3px] border-border-strong shadow-sm" :style="{ backgroundColor: getStatusColor(profileStore.baseInfo?.status) }"></div>
                </div>

                <!-- 基础信息与动作按钮 -->
                <div class="flex-1 min-w-0 pb-2">
                  <div class="flex items-start justify-between">
                    <div>
                      <div class="flex items-center gap-3 mb-1.5">
                        <h1 class="text-3xl font-extrabold text-text tracking-tight truncate drop-shadow-sm">
                          {{ profileStore.baseInfo?.displayName || 'Loading...' }}
                        </h1>
                        <span v-if="profileStore.baseInfo?.location?.includes('CN')" class="text-xl" title="China">🇨🇳</span>
                        <span v-else-if="profileStore.baseInfo?.location?.includes('JP')" class="text-xl" title="Japan">🇯🇵</span>
                        <span v-else-if="profileStore.baseInfo?.location?.includes('US')" class="text-xl" title="USA">🇺🇸</span>
                        
                        <!-- Trust Level Badge -->
                        <span 
                          class="px-2.5 py-0.5 rounded-md text-[11px] font-bold flex items-center gap-1.5 border border-border-soft shadow-sm ml-2 bg-surface backdrop-blur-md"
                          :style="{ color: trustColor }"
                        >
                          {{ trustName }}
                        </span>
                      </div>
                      <div class="text-[14px] text-text-muted font-medium font-mono tracking-wide drop-shadow-sm flex items-center gap-4">
                        <span class="flex items-center gap-1">@{{ profileStore.baseInfo?.username || profileStore.baseInfo?.displayName || '' }}</span>
                        <span class="flex items-center gap-1 text-text-muted hover:text-primary cursor-pointer transition-colors px-2 py-0.5 rounded-md bg-background/10 border border-border-soft text-[11px]" @click="copyToClipboard(profileStore.baseInfo?.id || '')" :title="profileStore.baseInfo?.id">
                          <Copy class="w-3.5 h-3.5" /> {{ profileStore.baseInfo?.id?.substring(0, 13) }}...
                        </span>
                      </div>
                    </div>

                    <!-- 按钮操作区 -->
                    <div class="flex gap-2 more-menu-container relative z-30">
                      <button v-if="!isSelf" @click="showFavoriteModal = !showFavoriteModal" class="w-10 h-10 rounded-xl bg-surface hover:bg-surface backdrop-blur-md flex items-center justify-center text-text-muted transition-all border border-border-soft shadow-sm hover:shadow-lg hover:text-primary">
                        <Star class="w-4.5 h-4.5" :class="{'fill-primary text-primary': isFavorite}" />
                      </button>
                      <button 
                        class="w-10 h-10 rounded-xl bg-surface hover:bg-surface backdrop-blur-md flex items-center justify-center text-text-muted transition-all border border-border-soft shadow-sm hover:shadow-lg hover:text-primary"
                        @click="showMoreMenu = !showMoreMenu"
                      >
                        <MoreHorizontal class="w-4.5 h-4.5" />
                      </button>

                      
                      <!-- Favorite Group Modal -->
                      <transition name="scale">
                        <div v-if="showFavoriteModal" class="absolute top-12 right-12 w-72 bg-surface rounded-lg border border-border-soft shadow-xl z-[100] text-[13px] overflow-hidden flex flex-col">
                          <div class="px-4 py-3 border-b border-border-soft flex justify-between items-center bg-surface-hover">
                            <span class="font-bold text-text tracking-wide">{{ $t('auto_eb9092cd') }}</span>
                            <button @click="showFavoriteModal = false" class="text-border-strong hover:text-text-muted transition-colors"><X class="w-4 h-4" /></button>
                          </div>
                          <div class="p-4 flex flex-col gap-4">
                            <div>
                              <div class="text-xs text-text-muted mb-2">{{ $t('auto_7fc3a5f2') }}</div>
                              <div class="flex flex-col gap-1.5">
                                <button @click="addFavoriteToGroup('group_0')" class="w-full py-2 px-3 text-left rounded-md hover:bg-surface-hover border border-transparent hover:border-border-soft transition-all text-text-muted hover:text-indigo-600">group_0</button>
                                <button @click="addFavoriteToGroup('group_1')" class="w-full py-2 px-3 text-left rounded-md hover:bg-surface-hover border border-transparent hover:border-border-soft transition-all text-text-muted hover:text-indigo-600">Group 2</button>
                                <button @click="addFavoriteToGroup('group_2')" class="w-full py-2 px-3 text-left rounded-md hover:bg-surface-hover border border-transparent hover:border-border-soft transition-all text-text-muted hover:text-indigo-600">Group 3</button>
                                <button @click="addFavoriteToGroup('group_3')" class="w-full py-2 px-3 text-left rounded-md hover:bg-surface-hover border border-transparent hover:border-border-soft transition-all text-text-muted hover:text-indigo-600">Group 4</button>
                              </div>
                            </div>
                          </div>
                        </div>
                      </transition>

                      <!-- 下拉菜单 -->
                      <transition name="dropdown">
                        <div v-if="showMoreMenu" class="absolute top-12 right-0 w-64 bg-surface rounded-lg py-1.5 border border-border-soft shadow-xl z-[100] text-[13px] text-text-muted max-h-[50vh] overflow-y-auto custom-scrollbar">
                          <div class="px-4 py-2 hover:bg-surface-hover hover:text-indigo-600 cursor-pointer flex items-center gap-3 transition-colors" @click="executeAction('refresh'); showMoreMenu = false"><RefreshCcw class="w-4 h-4" />{{ $t('auto_694fc5ef') }}</div>
                          <div class="px-4 py-2 hover:bg-surface-hover hover:text-indigo-600 cursor-pointer flex items-center gap-3 transition-colors" @click="executeAction('share'); showMoreMenu = false"><Share2 class="w-4 h-4" />{{ $t('auto_c31f48f8') }}</div>
                          <template v-if="!isSelf">
                            <div class="h-[1px] bg-background/10 my-1.5 mx-2"></div>
                            <div class="px-4 py-2 hover:bg-surface-hover hover:text-indigo-600 cursor-pointer flex items-center gap-3 transition-colors" @click="executeAction('request_invite'); showMoreMenu = false"><LogIn class="w-4 h-4" />{{ $t('auto_98b2d831') }}</div>
                            <div class="px-4 py-2 hover:bg-surface-hover hover:text-indigo-600 cursor-pointer flex items-center gap-3 transition-colors" @click="executeAction('invite'); showMoreMenu = false"><Hand class="w-4 h-4" />{{ $t('auto_be904d90') }}</div>
                            <div class="px-4 py-2 hover:bg-surface-hover hover:text-indigo-600 cursor-pointer flex items-center gap-3 transition-colors" @click="isEditingNote = true; showMoreMenu = false"><PenLine class="w-4 h-4" />{{ $t('auto_9f06dd67') }}</div>
                          </template>
                          <div class="h-[1px] bg-background/10 my-1.5 mx-2"></div>
                          <div class="px-4 py-2 hover:bg-surface-hover hover:text-indigo-600 cursor-pointer flex items-center gap-3 transition-colors" @click="executeAction('show_avatar_info'); showMoreMenu = false"><User class="w-4 h-4" />{{ $t('auto_ad7cf636') }}</div>
                          <template v-if="!isSelf">
                            <div class="px-4 py-2 hover:bg-surface-hover hover:text-indigo-600 cursor-pointer flex items-center gap-3 transition-colors" @click="executeAction('show_history'); showMoreMenu = false"><History class="w-4 h-4" />{{ $t('auto_be88db2a') }}</div>
                            <div class="h-[1px] bg-background/10 my-1.5 mx-2"></div>
                            <div class="px-4 py-2 hover:bg-surface-hover hover:text-indigo-600 cursor-pointer flex items-center gap-3 transition-colors" @click="executeAction('block'); showMoreMenu = false"><Ban class="w-4 h-4" />{{ $t('auto_dd4e0b57') }}</div>
                            <div class="px-4 py-2 hover:bg-surface-hover hover:text-indigo-600 cursor-pointer flex items-center gap-3 transition-colors" @click="executeAction('mute'); showMoreMenu = false"><VolumeX class="w-4 h-4" />{{ $t('auto_351cb1f8') }}</div>
                            <div class="px-4 py-2 hover:bg-surface-hover hover:text-indigo-600 cursor-pointer flex items-center gap-3 transition-colors" @click="executeAction('hide_chat'); showMoreMenu = false"><MessageSquareOff class="w-4 h-4" />{{ $t('auto_b599c124') }}</div>
                            <div class="px-4 py-2 hover:bg-surface-hover hover:text-indigo-600 cursor-pointer flex items-center gap-3 transition-colors" @click="executeAction('showAvatar'); showMoreMenu = false"><Eye class="w-4 h-4" />{{ $t('auto_f0deb532') }}</div>
                            <div class="px-4 py-2 hover:bg-surface-hover hover:text-indigo-600 cursor-pointer flex items-center gap-3 transition-colors" @click="executeAction('hideAvatar'); showMoreMenu = false"><EyeOff class="w-4 h-4" />{{ $t('auto_05c28106') }}</div>
                            <div class="px-4 py-2 hover:bg-surface-hover hover:text-indigo-600 cursor-pointer flex items-center gap-3 transition-colors" @click="executeAction('disable_interaction'); showMoreMenu = false"><ShieldBan class="w-4 h-4" />{{ $t('auto_db1a1d62') }}</div>
                            <div class="px-4 py-2 hover:bg-red-50 text-red-500 cursor-pointer flex items-center gap-3 transition-colors" @click="executeAction('report'); showMoreMenu = false"><Flag class="w-4 h-4" />{{ $t('auto_bff74409') }}</div>
                            <div class="px-4 py-2 hover:bg-red-50 text-red-600 cursor-pointer flex items-center gap-3 transition-colors mt-1" @click="executeAction('unfriend'); showMoreMenu = false"><UserMinus class="w-4 h-4" />{{ $t('auto_5a35d2b6') }}</div>
                          </template>
                        </div>
                      </transition>
                    </div>
                  </div>
                  
                  <!-- 在线状态信息栏 -->
                  <div class="flex items-center gap-3 mt-3 flex-wrap">
                    <span class="px-3 py-1 rounded-md bg-surface border border-border-soft text-text-muted text-[12px] font-bold shadow-sm flex items-center gap-1.5 backdrop-blur-sm">
                      <Monitor class="w-3.5 h-3.5 text-text-muted" /> Desktop
                    </span>
                    <span v-if="profileStore.baseInfo?.status_description" class="px-3 py-1 rounded-md bg-surface border border-border-soft text-text-muted text-[12px] font-bold shadow-sm flex items-center gap-1.5 backdrop-blur-sm max-w-[300px] truncate">
                      <MessageSquarePlus class="w-3.5 h-3.5 text-text-muted shrink-0" /> {{ profileStore.baseInfo.status_description }}
                    </span>
                    <span class="px-3 py-1 rounded-md bg-surface border border-border-soft text-text-muted text-[12px] font-bold shadow-sm flex items-center gap-1.5 backdrop-blur-sm">
                      <Languages class="w-3.5 h-3.5 text-text-muted" /> zh-CN, en
                    </span>
                  </div>
                </div>
              </div>

              <!-- Tabs -->
              <div class="flex gap-2 mt-6 overflow-x-auto custom-scrollbar">
                <button 
                  class="px-5 py-2 text-[13px] font-bold rounded-lg transition-all"
                  :class="activeTab === 'info' ? 'bg-indigo-500 text-text shadow-md' : 'text-text-muted hover:bg-surface hover:text-text'"
                  @click="activeTab = 'info'"
                >
                  个人资料
                </button>
                <button 
                  v-if="!isSelf"
                  class="px-5 py-2 text-[13px] font-bold rounded-lg transition-all"
                  :class="activeTab === 'mutual' ? 'bg-indigo-500 text-text shadow-md' : 'text-text-muted hover:bg-surface hover:text-text'"
                  @click="activeTab = 'mutual'"
                >
                  共同好友
                </button>
                <button 
                  class="px-5 py-2 text-[13px] font-bold rounded-lg transition-all flex items-center gap-1.5"
                  :class="activeTab === 'groups' ? 'bg-indigo-500 text-text shadow-md' : 'text-text-muted hover:bg-surface hover:text-text'"
                  @click="activeTab = 'groups'"
                >
                  群组 <span class="bg-black/30 px-1.5 py-0.5 rounded text-[10px]">{{ profileStore.groups.length || 0 }}</span>
                </button>
                <button 
                  class="px-5 py-2 text-[13px] font-bold rounded-lg transition-all"
                  :class="activeTab === 'created_worlds' ? 'bg-indigo-500 text-text shadow-md' : 'text-text-muted hover:bg-surface hover:text-text'"
                  @click="activeTab = 'created_worlds'"
                >
                  创造的世界
                </button>
                <button 
                  class="px-5 py-2 text-[13px] font-bold rounded-lg transition-all"
                  :class="activeTab === 'fav_worlds' ? 'bg-indigo-500 text-text shadow-md' : 'text-text-muted hover:bg-surface hover:text-text'"
                  @click="activeTab = 'fav_worlds'"
                >
                  收藏的世界
                </button>
                <button 
                  class="px-5 py-2 text-[13px] font-bold rounded-lg transition-all"
                  :class="activeTab === 'created_avatars' ? 'bg-indigo-500 text-text shadow-md' : 'text-text-muted hover:bg-surface hover:text-text'"
                  @click="activeTab = 'created_avatars'"
                >
                  创造的模型
                </button>
                <button 
                  class="px-5 py-2 text-[13px] font-bold rounded-lg transition-all"
                  :class="activeTab === 'activity' ? 'bg-indigo-500 text-text shadow-md' : 'text-text-muted hover:bg-surface hover:text-text'"
                  @click="activeTab = 'activity'"
                >
                  活动记录
                </button>
                <button 
                  class="px-5 py-2 text-[13px] font-bold rounded-lg transition-all"
                  :class="activeTab === 'raw_json' ? 'bg-indigo-500 text-text shadow-md' : 'text-text-muted hover:bg-surface hover:text-text'"
                  @click="activeTab = 'raw_json'"
                >
                  JSON
                </button>
              </div>
            </div>
          </div>

          <!-- Content Body -->
          <div class="flex-1 overflow-y-auto custom-scrollbar p-8 bg-transparent relative z-10">
            
            <!-- 个人资料 Tab -->
            <template v-if="activeTab === 'info'">
              <!-- 当前位置 Banner -->
              <div v-if="locationParsed && profileStore.baseInfo?.status !== 'offline'" class="mb-8 p-4 bg-surface border border-border-soft rounded-xl flex items-center justify-between shadow-sm">
                <div class="flex items-center gap-4">
                  <div class="w-14 h-14 rounded-lg bg-background/10 flex items-center justify-center border border-border-soft shrink-0 overflow-hidden relative">
                    <img v-if="locationParsed.flag === '🔒'" src="https://images.unsplash.com/photo-1550684848-fac1c5b4e853?auto=format&fit=crop&w=120&q=80" class="w-full h-full object-cover opacity-50" />
                    <img v-else :src="resolvedWorldThumbnail || 'https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?auto=format&fit=crop&w=120&q=80'" class="w-full h-full object-cover" />
                    <div class="absolute inset-0 bg-background/10"></div>
                  </div>
                  <div>
                    <div class="flex items-center gap-2 mb-1">
                      <span class="text-[14px] font-extrabold text-text truncate max-w-[200px]" :title="locationParsed.name">{{ resolvedWorldName || locationParsed.name }}</span>
                      <span class="px-1.5 py-0.5 rounded bg-background/10 text-[10px] font-bold text-text-muted border border-border-soft shadow-sm">#{{ locationParsed.raw.split('~')[0].split(':')[1] || 'Public' }}</span>
                    </div>
                    <div class="text-[12px] text-text-muted flex items-center gap-3 font-medium">
                      <span class="flex items-center gap-1"><Users class="w-3.5 h-3.5" /> ? / ?</span>
                      <span class="flex items-center gap-1">{{ locationParsed.flag }} {{ locationParsed.flag === '🔒' ? 'Private' : 'Public' }}</span>
                    </div>
                  </div>
                </div>
                <div v-if="!isSelf" class="flex gap-2">

                  <button class="px-4 py-2 bg-indigo-500 hover:bg-indigo-600 text-text text-[13px] font-bold rounded-lg transition-colors flex items-center gap-2 shadow-sm" @click="executeAction('request_invite')">
                    <LogIn class="w-4 h-4" /> 申请加入
                  </button>
                  <button class="px-3 py-2 bg-surface hover:bg-surface-hover text-text text-[13px] font-bold rounded-lg transition-colors border border-border-soft shadow-sm">
                    <RefreshCcw class="w-4 h-4" />
                  </button>
                </div>
              </div>

              <!-- 主信息网格 -->
              <div class="grid grid-cols-1 md:grid-cols-3 gap-8">
                <!-- 左列：备注与简介 -->
                <div class="md:col-span-2 space-y-6">
                  
                  <!-- 备注卡片 -->
                  <div class="bg-surface border border-border-soft rounded-xl p-5 shadow-sm">
                    <div class="flex items-center justify-between mb-3">
                      <h3 class="text-[13px] font-bold text-text flex items-center gap-2">
                        <PencilLine class="w-4 h-4 text-indigo-400" /> 本地备注 (VRCX)
                      </h3>
                      <button v-if="!isEditingNote" @click="isEditingNote = true" class="text-[12px] text-indigo-400 hover:text-indigo-300 font-bold transition-colors">
                        编辑
                      </button>
                    </div>
                    
                    <div v-if="isEditingNote" class="space-y-3">
                      <textarea 
                        v-model="localNote" 
                        class="w-full h-24 bg-transparent border border-indigo-500/50 rounded-lg p-3 text-[13px] text-text outline-none focus:ring-2 focus:ring-indigo-500/20 resize-none"
                        :placeholder="$t('auto_8d9a2358')"
                      ></textarea>
                      <div class="flex justify-end gap-2">
                        <button @click="isEditingNote = false" class="px-3 py-1.5 bg-background/10 hover:bg-background/20 text-text-muted text-[12px] font-bold rounded-md transition-colors">{{ $t('auto_625fb26b') }}</button>
                        <button @click="saveLocalNote" class="px-3 py-1.5 bg-indigo-500 hover:bg-indigo-600 text-text text-[12px] font-bold rounded-md transition-colors flex items-center gap-1.5"><Save class="w-3.5 h-3.5" />{{ $t('auto_be5fbbe3') }}</button>
                      </div>
                    </div>
                    <div v-else class="text-[13px] text-text-muted whitespace-pre-wrap leading-relaxed min-h-[40px]">
                      {{ localNote || '还没有添加本地备注。' }}
                    </div>
                  </div>

                  <!-- 个人简介卡片 -->
                  <div class="bg-surface border border-border-soft rounded-xl p-5 shadow-sm">
                    <div class="flex items-center justify-between mb-3">
                      <h3 class="text-[13px] font-bold text-text flex items-center gap-2">
                        <Languages class="w-4 h-4 text-emerald-400" /> 个人简介 (Bio)
                      </h3>
                      <button @click="handleTranslate" class="text-[12px] text-indigo-400 hover:text-indigo-300 font-bold transition-colors flex items-center gap-1" :disabled="isTranslating">
                        <Globe class="w-3.5 h-3.5" /> {{ translatedBio ? '显示原文' : '翻译' }}
                      </button>
                    </div>
                    <div class="text-[13px] text-text-muted whitespace-pre-wrap leading-relaxed" :class="{'opacity-50': isTranslating}">
                      {{ isTranslating ? '正在翻译...' : (translatedBio || profileStore.baseInfo?.bio || '这家伙很懒，什么都没写。') }}
                    </div>
                  </div>

                  <!-- 正在使用的模型 -->
                  <div class="bg-surface border border-border-soft rounded-xl p-5 shadow-sm">
                    <h3 class="text-[13px] font-bold text-text flex items-center gap-2 mb-4">
                      <Image class="w-4 h-4 text-sky-400" /> 正在使用的模型
                    </h3>
                    <div class="flex items-start gap-4">
                      <img 
                        :src="profileStore.baseInfo?.currentAvatarThumbnailImageUrl" 
                        class="w-20 h-20 rounded-lg object-cover border border-white/10 bg-transparent shrink-0 cursor-pointer hover:opacity-80 transition-opacity"
                        :title="$t('auto_b7bd9769')"
                      />
                      <div>
                        <div class="text-[14px] font-bold text-text mb-1">{{ $t('auto_3c76301b') }}</div>
                        <div class="text-[12px] text-text-muted mb-2">{{ $t('auto_745fa496') }}</div>
                        <div class="flex items-center gap-2 text-[11px]">
                          <span class="px-2 py-1 bg-surface border border-border-soft rounded text-text-muted">PC / Quest</span>
                          <span class="px-2 py-1 bg-surface border border-border-soft rounded text-text-muted">{{ $t('auto_dc9459d2') }}</span>
                        </div>
                      </div>
                    </div>
                  </div>

                </div>

                <!-- 右列：详细数据网格 -->
                <div class="space-y-4">
                  <div class="bg-surface border border-border-soft rounded-xl p-4 shadow-sm">
                    <div class="space-y-4">
                      <div v-if="!isSelf">
                        <div class="text-[11px] font-bold text-text-muted mb-0.5 uppercase tracking-wider">{{ $t('auto_0f6808b7') }}</div>
                        <div class="text-[13px] font-bold text-text">{{ getMockDuration() }}</div>
                      </div>
                      <div v-else>
                        <div class="text-[11px] font-bold text-text-muted mb-0.5 uppercase tracking-wider">{{ $t('auto_b6f1db7a') }}</div>
                        <div class="text-[13px] font-bold text-text">{{ getMockDuration() }}</div>
                      </div>

                      <div>
                        <div class="text-[11px] font-bold text-text-muted mb-0.5 uppercase tracking-wider">{{ $t('auto_24f572f3') }}</div>
                        <div class="text-[13px] font-bold text-text">{{ formatTime(profileStore.baseInfo?.last_activity) }}</div>
                      </div>

                      <div v-if="!isSelf">
                        <div class="text-[11px] font-bold text-text-muted mb-0.5 uppercase tracking-wider">{{ $t('auto_e64d38cb') }}</div>
                        <div class="text-[13px] font-bold text-text">2026-05-10 14:30:22</div>
                      </div>

                      <div v-if="!isSelf">
                        <div class="text-[11px] font-bold text-text-muted mb-0.5 uppercase tracking-wider">{{ $t('auto_f8169e31') }}</div>
                        <div class="text-[13px] font-bold text-text">{{ $t('auto_634e7f56') }}</div>
                      </div>

                      <div>
                        <div class="text-[11px] font-bold text-text-muted mb-0.5 uppercase tracking-wider">{{ $t('auto_08e67093') }}</div>
                        <div class="text-[13px] font-bold text-text">{{ formatDate(profileStore.baseInfo?.date_joined) }}</div>
                      </div>
                      
                      <div v-if="isSelf">
                        <div class="text-[11px] font-bold text-text-muted mb-0.5 uppercase tracking-wider">{{ $t('auto_e75888f6') }}</div>
                        <div class="text-[13px] font-bold text-yellow-400 flex items-center gap-1"><Star class="w-3 h-3 fill-current" /> 1,200</div>
                      </div>
                    </div>
                  </div>

                  <div class="bg-surface border border-border-soft rounded-xl p-4 shadow-sm">
                    <h3 class="text-[12px] font-bold text-text-muted mb-3 uppercase tracking-wider">{{ $t('auto_e69c654e') }}</h3>
                    <div class="space-y-3">
                      <div class="flex justify-between items-center text-[13px]">
                        <span class="text-text-muted">{{ $t('auto_7f8a6a38') }}</span>
                        <span class="font-bold" :class="isSelf ? 'text-red-400' : 'text-green-400'">{{ isSelf ? '否' : '是' }}</span>
                      </div>
                      <div class="flex justify-between items-center text-[13px]">
                        <span class="text-text-muted">{{ $t('auto_e18c47b3') }}</span>
                        <span class="font-bold text-green-400">{{ $t('auto_0a60ac8f') }}</span>
                      </div>
                      <div class="flex justify-between items-center text-[13px]">
                        <span class="text-text-muted">{{ $t('auto_b231a88a') }}</span>
                        <span class="font-bold text-green-400">{{ $t('auto_0a60ac8f') }}</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </template>

            <!-- 共同好友 Tab (VRCX Grid) -->
            <template v-else-if="activeTab === 'mutual'">
              <div class="flex items-center justify-between mb-6 pb-4 border-b border-border-soft">
                <div class="flex items-center gap-4 text-text-muted">
                  <RefreshCcw class="w-4 h-4 cursor-pointer hover:text-text transition-colors" />
                  <div class="flex items-center gap-1.5 text-[13px] font-bold">
                    <Users class="w-4 h-4" /> 共同好友 ({{ profileStore.mutualFriends.length }})
                  </div>
                </div>
                <div class="flex items-center gap-3">
                  <div class="relative group">
                    <Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-text-muted group-focus-within:text-indigo-400" />
                    <input type="text" :placeholder="$t('auto_64e327ed')" class="bg-surface border border-border-soft rounded-md py-1.5 pl-9 pr-3 text-[13px] text-text placeholder-slate-500 focus:outline-none focus:border-indigo-500 focus:bg-transparent transition-all w-48">
                  </div>
                </div>
              </div>
              
              <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-4">
                <div v-if="profileStore.isLoadingMutual" class="col-span-full py-10 flex justify-center"><RefreshCcw class="w-6 h-6 animate-spin text-text-muted" /></div>
                <div v-else-if="profileStore.mutualFriends.length === 0" class="col-span-full py-10 flex justify-center text-text-muted text-sm font-bold">{{ $t('auto_ec1dc1d7') }}</div>
                <div v-for="friend in profileStore.mutualFriends" :key="friend.id" class="bg-surface border border-border-soft rounded-xl p-3 flex items-center gap-3 hover:bg-surface-hover hover:border-indigo-500/50 cursor-pointer transition-all group">
                  <div class="relative shrink-0">
                    <img :src="friend.currentAvatarThumbnailImageUrl || friend.profilePicOverride || 'https://via.placeholder.com/64'" class="w-12 h-12 rounded-lg object-cover border border-white/5">
                    <div class="absolute -bottom-1 -right-1 w-3.5 h-3.5 border-2 border-[#1e1f22] rounded-full" :style="{ backgroundColor: getStatusColor(friend.status) }"></div>
                  </div>
                  <div class="flex-1 min-w-0">
                    <div class="font-bold text-[13px] text-text truncate group-hover:text-indigo-300 transition-colors">{{ friend.displayName }}</div>
                    <div class="text-[11px] text-text-muted truncate">{{ friend.status_description || 'Online' }}</div>
                  </div>
                </div>
              </div>
            </template>

            <!-- 群组 Tab -->
            <template v-else-if="activeTab === 'groups'">
              <div class="flex items-center justify-between mb-6 pb-4 border-b border-border-soft">
                <div class="flex items-center gap-4 text-text-muted">
                  <RefreshCcw class="w-4 h-4 cursor-pointer hover:text-text transition-colors" />
                  <div class="flex items-center gap-1.5 text-[13px] font-bold">
                    展示的群组 ({{ profileStore.groups.length }})
                  </div>
                </div>
              </div>
              
              <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
                <div v-if="profileStore.isLoadingGroups" class="col-span-full py-10 flex justify-center"><RefreshCcw class="w-6 h-6 animate-spin text-text-muted" /></div>
                <div v-else-if="profileStore.groups.length === 0" class="col-span-full py-10 flex justify-center text-text-muted text-sm font-bold">{{ $t('auto_ac16db0d') }}</div>
                <div v-for="group in profileStore.groups" :key="group.id" class="bg-surface border border-border-soft rounded-xl p-4 flex flex-col gap-3 hover:bg-surface-hover hover:border-indigo-500/50 cursor-pointer transition-all group relative overflow-hidden">
                  <div class="absolute top-0 left-0 w-full h-12 bg-gradient-to-b from-indigo-500/10 to-transparent"></div>
                  <div class="flex items-center gap-3 relative z-10">
                    <img :src="group.iconUrl || group.bannerUrl || 'https://via.placeholder.com/64'" class="w-12 h-12 rounded-lg object-cover border border-white/10 shadow-sm bg-transparent">
                    <div class="flex-1 min-w-0">
                      <div class="font-bold text-[14px] text-text truncate group-hover:text-indigo-300 transition-colors">{{ group.name }}</div>
                      <div class="text-[11px] text-text-muted mt-0.5 flex items-center gap-1"><Users class="w-3 h-3" /> {{ group.memberCount || 0 }} 成员</div>
                    </div>
                  </div>
                </div>
              </div>
            </template>

            <!-- 创造的世界 Tab -->
            <template v-else-if="activeTab === 'created_worlds'">
              <div class="flex items-center justify-between mb-6 pb-4 border-b border-border-soft">
                <div class="flex items-center gap-1.5 text-[13px] font-bold text-text-muted">
                  <Map class="w-4 h-4" /> 该用户创建的世界
                </div>
              </div>
              <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-5">
                <div v-if="profileStore.isLoadingWorlds" class="col-span-full py-10 flex justify-center"><RefreshCcw class="w-6 h-6 animate-spin text-text-muted" /></div>
                <div v-else-if="profileStore.createdWorlds.length === 0" class="col-span-full py-10 flex justify-center text-text-muted text-sm font-bold">{{ $t('auto_136f38fc') }}</div>
                <div v-for="world in profileStore.createdWorlds" :key="world.id" class="bg-surface border border-border-soft rounded-xl overflow-hidden hover:border-indigo-500/50 cursor-pointer transition-all group shadow-sm hover:shadow-lg">
                  <div class="relative h-36">
                    <img :src="world.imageUrl || world.thumbnailImageUrl || 'https://via.placeholder.com/400x200'" class="w-full h-full object-cover bg-transparent">
                    <div class="absolute bottom-2 right-2 px-2 py-1 bg-black/60 backdrop-blur-md rounded text-[11px] font-bold text-text flex items-center gap-1">
                      <Users class="w-3 h-3" /> {{ world.occupants || 0 }}
                    </div>
                  </div>
                  <div class="p-4">
                    <div class="font-bold text-[14px] text-text mb-1 group-hover:text-indigo-400 transition-colors truncate">{{ world.name }}</div>
                    <div class="flex items-center gap-3 text-[11px] text-text-muted">
                      <span class="flex items-center gap-1"><Heart class="w-3 h-3" /> {{ world.favorites || 0 }}</span>
                      <span class="flex items-center gap-1"><Eye class="w-3 h-3" /> {{ world.visits || 0 }}</span>
                      <span>{{ formatDate(world.updated_at) }}</span>
                    </div>
                  </div>
                </div>
              </div>
            </template>

            <!-- 收藏的世界 Tab -->
            <template v-else-if="activeTab === 'fav_worlds'">
              <div class="flex items-center justify-between mb-6 pb-4 border-b border-border-soft">
                <div class="flex items-center gap-1.5 text-[13px] font-bold text-text-muted">
                  <Heart class="w-4 h-4 text-pink-400" /> 收藏的世界
                </div>
              </div>
              <div class="flex items-center justify-center h-48 text-text-muted font-bold text-[13px]">
                该用户的收藏列表是私密的。
              </div>
            </template>

            <!-- 创造的模型 Tab -->
            <template v-else-if="activeTab === 'created_avatars'">
              <div class="flex items-center justify-between mb-6 pb-4 border-b border-border-soft">
                <div class="flex items-center gap-1.5 text-[13px] font-bold text-text-muted">
                  <Cuboid class="w-4 h-4" /> 该用户公开的模型
                </div>
              </div>
              <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-4">
                <div v-if="profileStore.isLoadingAvatars" class="col-span-full py-10 flex justify-center"><RefreshCcw class="w-6 h-6 animate-spin text-text-muted" /></div>
                <div v-else-if="profileStore.createdAvatars.length === 0" class="col-span-full py-10 flex justify-center text-text-muted text-sm font-bold">{{ $t('auto_4bae05bc') }}</div>
                <div v-for="avatar in profileStore.createdAvatars" :key="avatar.id" class="bg-surface border border-border-soft rounded-xl overflow-hidden hover:border-indigo-500/50 cursor-pointer transition-all group shadow-sm">
                  <div class="relative aspect-square bg-transparent">
                    <img :src="avatar.thumbnailImageUrl || avatar.imageUrl || 'https://via.placeholder.com/200'" class="w-full h-full object-cover">
                    <div class="absolute top-2 left-2 px-1.5 py-0.5 bg-black/60 backdrop-blur-md rounded text-[10px] font-bold text-text border border-white/10">
                      {{ (avatar.supportedPlatforms || []).join(' / ') || 'PC/Quest' }}
                    </div>
                  </div>
                  <div class="p-3">
                    <div class="font-bold text-[13px] text-text truncate group-hover:text-indigo-400 transition-colors">{{ avatar.name }}</div>
                    <div class="text-[11px] text-text-muted mt-1">{{ avatar.releaseStatus === 'public' ? '公开模型' : '私有模型' }}</div>
                  </div>
                </div>
              </div>
            </template>

            <!-- 活动记录 Tab -->
            <template v-else-if="activeTab === 'activity'">
              <div class="flex items-center justify-between mb-6 pb-4 border-b border-border-soft">
                <div class="flex items-center gap-1.5 text-[13px] font-bold text-text-muted">
                  <History class="w-4 h-4" /> 本地记录的活动日志 (VRCX)
                </div>
              </div>
              <div v-if="profileStore.isLoadingActivity" class="col-span-full py-10 flex justify-center"><RefreshCcw class="w-6 h-6 animate-spin text-text-muted" /></div>
              <div v-else-if="profileStore.activityLogs.length === 0" class="col-span-full py-10 flex justify-center text-text-muted text-sm font-bold">{{ $t('auto_745c321a') }}</div>
              <div v-else class="space-y-0 relative before:absolute before:inset-0 before:ml-5 before:-translate-x-px md:before:mx-auto md:before:translate-x-0 before:h-full before:w-0.5 before:bg-gradient-to-b before:from-transparent before:via-[#3f4147] before:to-transparent">
                <div v-for="log in profileStore.activityLogs" :key="log.id || log.created_at" class="relative flex items-center justify-between md:justify-normal md:odd:flex-row-reverse group is-active mb-6">
                  <div class="flex items-center justify-center w-10 h-10 rounded-full border-4 border-[#111214] text-text shadow shrink-0 md:order-1 md:group-odd:-translate-x-1/2 md:group-even:translate-x-1/2 z-10" :class="log.type === 'online' ? 'bg-green-500' : log.type === 'offline' ? 'bg-surface-hover0' : 'bg-indigo-500'">
                    <LogIn v-if="log.type === 'online'" class="w-4 h-4" />
                    <LogOut v-else-if="log.type === 'offline'" class="w-4 h-4" />
                    <Map v-else class="w-4 h-4" />
                  </div>
                  <div class="w-[calc(100%-4rem)] md:w-[calc(50%-2.5rem)] bg-surface p-4 rounded-xl border border-border-soft shadow-sm">
                    <div class="flex items-center justify-between space-x-2 mb-1">
                      <div class="font-bold text-text text-[13px]">{{ log.type === 'online' ? '上线' : log.type === 'offline' ? '下线' : '位置变更' }}</div>
                      <time class="text-[11px] font-mono text-text-muted">{{ formatTime(log.created_at) }}</time>
                    </div>
                    <div class="text-text-muted text-[12px] break-words">{{ log.content || log.location || (log.type === 'online' ? '用户已上线' : '用户已下线') }}</div>
                  </div>
                </div>
              </div>
            </template>

            <!-- 原始 JSON 信息 Tab -->
            <template v-else-if="activeTab === 'raw_json'">
              <!-- Toolbar -->
              <div class="flex items-center gap-4 text-text-muted mb-4 pb-4 border-b border-border-soft">
                <RefreshCcw class="w-4 h-4 cursor-pointer hover:text-text transition-colors" :title="$t('auto_694fc5ef')" />
                <Download class="w-4 h-4 cursor-pointer hover:text-text transition-colors" :title="$t('auto_f26ef914')" />
              </div>
              
              <!-- JSON Viewer -->
              <div class="bg-[#0a0a0c] rounded-xl p-5 font-mono text-[13px] leading-relaxed overflow-x-auto border border-border-soft shadow-inner custom-scrollbar">
                <pre v-html="highlightJson(profileStore.baseInfo)"></pre>
              </div>
            </template>

          </div>
        </div>
      </transition>
    </div>
  </transition>

  <!-- Image Preview Modal -->
    <div 
      v-if="showImagePreview" 
      class="fixed inset-0 z-[10000] flex items-center justify-center bg-black/80 backdrop-blur-xl transition-all"
    >
      <!-- Toolbar -->
      <div class="absolute top-4 right-4 bg-surface backdrop-blur-md border border-white/10 rounded-xl p-1.5 flex items-center gap-1 shadow-2xl">
        <button class="p-2 hover:bg-surface rounded-lg text-text-muted hover:text-text transition-colors" :title="$t('auto_79d3abe9')" @click="handleCopyImage">
          <Copy class="w-4 h-4" />
        </button>
        <button class="p-2 hover:bg-surface rounded-lg text-text-muted hover:text-text transition-colors" :title="$t('auto_f26ef914')" @click="handleDownloadImage">
          <Download class="w-4 h-4" />
        </button>
        <div class="w-[1px] h-4 bg-surface mx-1"></div>
        <button class="p-2 hover:bg-surface rounded-lg text-text-muted hover:text-text transition-colors" :title="$t('auto_4f9b192c')" @click="handleZoomIn">
          <ZoomIn class="w-4 h-4" />
        </button>
        <button class="p-2 hover:bg-surface rounded-lg text-text-muted hover:text-text transition-colors" :title="$t('auto_b21ac253')" @click="handleZoomOut">
          <ZoomOut class="w-4 h-4" />
        </button>
        <div class="w-[1px] h-4 bg-surface mx-1"></div>
        <button class="p-2 hover:bg-surface rounded-lg text-text-muted hover:text-text transition-colors" :title="$t('auto_9b95ba68')" @click="handleRotateCcw">
          <RotateCcw class="w-4 h-4" />
        </button>
        <button class="p-2 hover:bg-surface rounded-lg text-text-muted hover:text-text transition-colors" :title="$t('auto_0d403239')" @click="handleRotateCw">
          <RotateCw class="w-4 h-4" />
        </button>
        <div class="w-[1px] h-4 bg-surface mx-1"></div>
        <button class="p-2 hover:bg-surface rounded-lg text-text-muted hover:text-text transition-colors" :title="$t('auto_4b9c3271')" @click="handleResetImage">
          <RefreshCcw class="w-4 h-4" />
        </button>
        <button class="p-2 hover:bg-red-500/20 rounded-lg text-text-muted hover:text-red-400 transition-colors ml-1" :title="$t('auto_b15d9127')" @click="toggleImagePreview">
          <X class="w-4 h-4" />
        </button>
      </div>

      <!-- Image Display -->
      <div 
        class="relative max-w-[90vw] max-h-[90vh] transition-transform duration-300 ease-out"
        :style="{ transform: `scale(${imageScale}) rotate(${imageRotation}deg)` }"
      >
        <img 
          :src="profileStore.baseInfo?.profilePicOverride || profileStore.baseInfo?.currentAvatarImageUrl || profileStore.baseInfo?.currentAvatarThumbnailImageUrl"
          class="max-w-full max-h-[90vh] object-contain rounded-lg shadow-2xl"
          @click.stop
        />
      </div>
      
      <!-- Click background to close -->
      <div class="absolute inset-0 z-[-1]" @click="toggleImagePreview"></div>
    </div>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.scale-enter-active,
.scale-leave-active {
  transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
}
.scale-enter-from,
.scale-leave-to {
  opacity: 0;
  transform: scale(0.97);
}

.dropdown-enter-active,
.dropdown-leave-active {
  transition: all 0.15s ease;
}
.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-5px);
}

.custom-scrollbar::-webkit-scrollbar { width: 8px; height: 8px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.1); border-radius: 4px; border: 2px solid transparent; background-clip: padding-box; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.2); border: 2px solid transparent; background-clip: padding-box; }

/* JSON Viewer base styles */
:deep(.text-blue-400) { color: #60a5fa; }
:deep(.text-text-muted) { color: #cbd5e1; font-weight: 600; }
:deep(.text-green-400) { color: #4ade80; }
:deep(.text-blue-500) { color: #3b82f6; font-weight: bold; }
:deep(.text-pink-500) { color: #ec4899; font-weight: bold; }
</style>
