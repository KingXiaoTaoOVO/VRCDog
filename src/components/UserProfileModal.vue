<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { 
  X, MoreHorizontal, Star, Monitor, Headset, Copy, RefreshCcw, Share2, 
  LogIn, Mail, Hand, MessageSquarePlus, Settings, PenLine, User, Users, 
  History, Ban, VolumeX, MessageSquareOff, Eye, EyeOff, ShieldBan, Flag, 
  UserMinus, Search, ChevronDown, Download, Check, ZoomIn, ZoomOut, RotateCcw, RotateCw, Globe, PencilLine, Save, Languages, Image, Heart, Map, Cuboid, LogOut,
  Shield, ExternalLink, Info, Database, Clock, Calendar, FileJson, Code, AlignLeft, MapPin
} from 'lucide-vue-next';
import { useUserProfileStore } from '../stores/userProfile';
import { useToast } from '../composables/useToast';
import { VrcApi, DbApi } from '../api';
import VrcAvatar from './VrcAvatar.vue';
import VrcResourceCard from './VrcResourceCard.vue';

const { t } = useI18n();
const profileStore = useUserProfileStore();
const toast = useToast();

const activeTab = ref<'info' | 'social' | 'mutual' | 'groups' | 'created_worlds' | 'fav_worlds' | 'created_avatars' | 'activity' | 'raw_json'>('info');
const showMoreMenu = ref(false);
const showFavoriteModal = ref(false);
const isFavorite = ref(false);
const isEditingNote = ref(false);
const localNote = ref('');
const translatedBio = ref('');
const isTranslating = ref(false);

const isSelf = computed(() => profileStore.baseInfo?.id === profileStore.myId);

// Sync local note with store
watch(() => profileStore.localNote, (newNote) => {
  localNote.value = newNote;
}, { immediate: true });

const executeAction = async (action: string) => {
  const userId = profileStore.baseInfo?.id;
  if (!userId) return;
  try {
    switch (action) {
      case 'refresh':
        await profileStore.openProfile(userId);
        toast.success(t('user_profile.actions.refresh_success'));
        break;
      case 'copy_id':
        navigator.clipboard.writeText(userId);
        toast.success(t('user_profile.actions.copy_id_success'));
        break;
      case 'copy_vrc_url':
        navigator.clipboard.writeText('https://vrchat.com/home/user/' + userId);
        toast.success(t('user_profile.actions.copy_url_success'));
        break;
      case 'view_on_vrc':
        window.open('https://vrchat.com/home/user/' + userId, '_blank');
        break;
      case 'request_invite':
        await VrcApi.requestInvite(userId);
        toast.success(t('user_profile.actions.request_invite_success'));
        break;
      case 'invite':
        await VrcApi.inviteUser(userId);
        toast.success(t('user_profile.actions.invite_success'));
        break;
      case 'unfriend':
        if (confirm(t('user_profile.actions.unfriend_confirm'))) {
          await VrcApi.unfriend({ userId });
          toast.success(t('user_profile.actions.unfriend_success'));
          profileStore.closeProfile();
        }
        break;
      case 'block':
        await VrcApi.moderateUser({ moderated: userId, type: 'block' });
        toast.success(t('user_profile.actions.block_success'));
        break;
      case 'mute':
        await VrcApi.moderateUser({ moderated: userId, type: 'mute' });
        toast.success(t('user_profile.actions.mute_success'));
        break;
      case 'showAvatar':
        await VrcApi.moderateUser({ moderated: userId, type: 'showAvatar' });
        toast.success(t('user_profile.actions.show_avatar_success'));
        break;
      case 'hideAvatar':
        await VrcApi.moderateUser({ moderated: userId, type: 'hideAvatar' });
        toast.success(t('user_profile.actions.hide_avatar_success'));
        break;
    }
  } catch (e: any) {
    toast.error(t('common.error') + ': ' + e.message);
  }
};

const handleTranslate = async () => {
  if (translatedBio.value) {
    translatedBio.value = '';
    return;
  }
  isTranslating.value = true;
  // Mock translation for now, or use an API if available
  await new Promise(r => setTimeout(r, 600));
  translatedBio.value = t('user.translate_result') + (profileStore.baseInfo?.bio || '');
  isTranslating.value = false;
};

const saveLocalNote = async () => {
  profileStore.localNote = localNote.value;
  await profileStore.saveLocalNote();
  isEditingNote.value = false;
  toast.success(t('user_profile.info.memo_saved'));
};

// Image Preview logic
const showImagePreview = ref(false);
const imageScale = ref(1);
const imageRotation = ref(0);

const toggleImagePreview = () => {
  showImagePreview.value = !showImagePreview.value;
  imageScale.value = 1;
  imageRotation.value = 0;
};

const handleResetImage = () => {
  imageScale.value = 1;
  imageRotation.value = 0;
};

const handleZoomIn = () => { imageScale.value = Math.min(imageScale.value + 0.25, 4); };
const handleZoomOut = () => { imageScale.value = Math.max(imageScale.value - 0.25, 0.5); };
const handleRotateCw = () => { imageRotation.value += 90; };

const handleCopyImage = async () => {
  const url = profileStore.baseInfo?.profilePicOverride || profileStore.baseInfo?.currentAvatarImageUrl || profileStore.baseInfo?.currentAvatarThumbnailImageUrl;
  if (!url) return;
  try {
    const response = await fetch(url);
    const blob = await response.blob();
    await navigator.clipboard.write([new ClipboardItem({ [blob.type]: blob })]);
    toast.success(t('user_profile.actions.copy_image_success'));
  } catch (err) {
    toast.error(t('user.copy_fail'));
  }
};

const handleDownloadImage = () => {
  const url = profileStore.baseInfo?.profilePicOverride || profileStore.baseInfo?.currentAvatarImageUrl || profileStore.baseInfo?.currentAvatarThumbnailImageUrl;
  if (!url) return;
  const a = document.createElement('a');
  a.href = url;
  a.download = `user_${profileStore.baseInfo?.id}.png`;
  a.click();
};


const trustColor = computed(() => {
  if (!profileStore.baseInfo?.tags) return 'var(--color-text-muted)';
  const tags = profileStore.baseInfo.tags;
  if (tags.includes('system_trust_veteran') || tags.includes('system_trust_legend')) return 'var(--color-primary)';
  if (tags.includes('system_trust_trusted')) return '#f97316'; // orange-500
  if (tags.includes('system_trust_known')) return '#22c55e'; // green-500
  if (tags.includes('system_trust_basic')) return '#3b82f6'; // blue-500
  return 'var(--color-text-muted)';
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
  switch (status?.toLowerCase()) {
    case 'active': return '#22c55e';
    case 'join me': return '#3b82f6';
    case 'ask me': return '#f97316';
    case 'busy': return '#f97316';
    case 'do not disturb': return '#ef4444';
    default: return '#64748b';
  }
};

const formatTime = (time: string | undefined) => {
  if (!time) return '-';
  const d = new Date(time);
  return d.toLocaleString();
};

const formatDate = (time: string | undefined) => {
  if (!time) return '-';
  const d = new Date(time);
  return d.toLocaleDateString();
};

const highlightJson = (obj: any) => {
  if (!obj) return '{}';
  let json = JSON.stringify(obj, null, 2);
  json = json.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  return json.replace(/("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g, function (match) {
    let cls = 'text-primary';
    if (/^"/.test(match)) {
      if (/:$/.test(match)) cls = 'text-text-muted';
      else cls = 'text-emerald-400';
    } else if (/true|false/.test(match)) cls = 'text-blue-400';
    else if (/null/.test(match)) cls = 'text-red-400';
    return '<span class="' + cls + '">' + match + '</span>';
  });
};

const closeMenus = (e: MouseEvent) => {
  const target = e.target as HTMLElement;
  if (!target.closest('.more-menu-container')) showMoreMenu.value = false;
  if (!target.closest('.fav-menu-container')) showFavoriteModal.value = false;
};

onMounted(() => document.addEventListener('click', closeMenus));
onUnmounted(() => document.removeEventListener('click', closeMenus));

</script>

<template>
  <transition name="fade">
    <div
      v-if="profileStore.isOpen"
      class="fixed inset-0 bg-black/60 z-[100] flex items-center justify-center p-4 backdrop-blur-xl"
      @click.self="profileStore.closeProfile"
    >
      <!-- Main Panel -->
      <transition name="scale">
        <div
          class="glass-panel border-white/10 shadow-[0_32px_64px_-12px_rgba(0,0,0,0.5)] rounded-[24px] w-full max-w-[1100px] max-h-[90vh] flex flex-col overflow-hidden text-text relative"
        >
          <!-- Background Decorative Blobs -->
          <div class="absolute top-[-10%] left-[-10%] w-[40%] h-[40%] bg-primary/20 rounded-full blur-[100px] pointer-events-none"></div>
          <div class="absolute bottom-[-10%] right-[-10%] w-[40%] h-[40%] bg-primary/10 rounded-full blur-[100px] pointer-events-none"></div>

          <!-- Header Section -->
          <div class="relative shrink-0 border-b border-border-soft bg-transparent z-20 overflow-hidden">
            <!-- Banner Background -->
            <div class="absolute inset-0 z-0 h-48 overflow-hidden">
              <div class="absolute inset-0 bg-gradient-to-b from-black/20 via-[var(--theme-surface)]/80 to-[var(--theme-surface)] z-10"></div>
              <img 
                v-if="profileStore.baseInfo?.profilePicOverride || profileStore.baseInfo?.currentAvatarThumbnailImageUrl"
                :src="profileStore.baseInfo?.profilePicOverride || profileStore.baseInfo?.currentAvatarThumbnailImageUrl"
                class="w-full h-full object-cover filter blur-[60px] opacity-30 scale-110"
              />
            </div>

            <div class="relative z-30 px-8 pt-12 pb-6">
              <div class="flex gap-8 items-end">
                <!-- Avatar -->
                <div 
                  class="w-40 h-40 shrink-0 bg-[var(--theme-surface)]/40 backdrop-blur-md rounded-3xl flex items-center justify-center overflow-hidden border border-border-soft cursor-pointer group relative shadow-2xl z-20"
                  @click="toggleImagePreview"
                >
                  <div class="relative group shrink-0">
                    <div class="w-28 h-28 rounded-[2rem] overflow-hidden border-4 border-[var(--theme-surface)] shadow-2xl transition-transform hover:scale-105 hover:-rotate-3" @click.stop="toggleImagePreview">
                      <img 
                        :src="profileStore.baseInfo?.currentAvatarThumbnailImageUrl || profileStore.baseInfo?.currentAvatarImageUrl || profileStore.baseInfo?.profilePicOverride || 'https://via.placeholder.com/150'" 
                        class="w-full h-full object-cover bg-[var(--theme-surface)]"
                      />
                    </div>
                    <!-- Status Indicator -->
                    <div 
                      class="absolute bottom-1 right-1 w-6 h-6 rounded-full border-4 border-[var(--theme-surface)] shadow-lg"
                      :style="{ backgroundColor: getStatusColor(profileStore.baseInfo?.status || 'offline') }"
                    ></div>
                  </div>
                </div>

                <!-- Basic Info -->
                <div class="flex-1 min-w-0 pb-1">
                  <div class="flex items-center gap-3">
                    <h2 class="text-3xl font-black text-[var(--theme-text-strong)] truncate tracking-tight">{{ profileStore.baseInfo?.displayName || 'Loading...' }}</h2>
                    <span 
                      class="px-2.5 py-1 rounded-lg text-[11px] font-bold tracking-wide uppercase border border-border-soft"
                      :style="{ color: trustColor, backgroundColor: `${trustColor}15`, borderColor: `${trustColor}30` }"
                    >
                      {{ trustName }}
                    </span>
                  </div>
                  
                  <div class="flex items-center gap-2 mt-2 text-sm font-medium text-[var(--theme-text-muted)] truncate max-w-lg">
                    <span class="truncate">{{ profileStore.baseInfo?.statusDescription || t('auto_37e93fbb') }}</span>
                  </div>
                  
                  <div v-if="profileStore.baseInfo?.location && profileStore.baseInfo.location !== 'offline'" class="flex items-center gap-2 mt-2.5">
                    <div class="px-3 py-1.5 rounded-xl bg-[var(--theme-surface)]/60 backdrop-blur-md border border-border-soft flex items-center gap-2 text-[12px] text-[var(--theme-text-strong)] cursor-pointer hover:bg-[var(--theme-surface)]-hover transition-colors shadow-sm">
                      <MapPin class="w-3.5 h-3.5 text-blue-400" />
                      <span class="truncate max-w-[200px]">{{ profileStore.baseInfo.location.split('~')[0] }}</span>
                    </div>
                  </div>
                </div>

                <!-- Quick Actions -->
                <div class="flex items-center gap-2 pb-1 relative">
                  <button 
                    class="w-10 h-10 rounded-2xl flex items-center justify-center bg-[var(--theme-surface)] border border-border-soft text-[var(--theme-text-strong)] hover:bg-[var(--theme-surface)]-hover transition-all shadow-lg hover:-translate-y-0.5 fav-menu-container"
                    @click.stop="showFavoriteModal = !showFavoriteModal"
                  >
                    <Star :size="18" :class="{ 'fill-yellow-400 text-yellow-400': profileStore.baseInfo?.isFriend, 'text-[var(--theme-text-muted)]': !profileStore.baseInfo?.isFriend }" />
                  </button>
                  
                  <transition name="fade">
                    <div v-if="showFavoriteModal" class="absolute top-12 right-12 w-64 glass-panel border-border-soft rounded-2xl shadow-2xl p-2 z-50 overflow-hidden text-[13px] fav-menu-container">
                      <div class="px-3 py-2 text-[11px] font-bold text-[var(--theme-text-muted)] uppercase tracking-wider mb-1">{{ t('user_profile.groups.favorite') }}</div>
                      <div class="max-h-48 overflow-y-auto custom-scrollbar">
                        <div 
                          class="px-3 py-2 rounded-xl hover:bg-[var(--theme-surface)]-hover cursor-pointer flex items-center justify-between text-[var(--theme-text-strong)] transition-colors mb-1"
                          @click="executeAction('favorite_group_1')"
                        >
                          <div class="flex items-center gap-2">
                            <div class="w-2.5 h-2.5 rounded-full bg-blue-500"></div> {{ t('user_profile.groups.group_1') }}
                          </div>
                        </div>
                        <div 
                          class="px-3 py-2 rounded-xl hover:bg-[var(--theme-surface)]-hover cursor-pointer flex items-center justify-between text-[var(--theme-text-strong)] transition-colors mb-1"
                          @click="executeAction('favorite_group_2')"
                        >
                          <div class="flex items-center gap-2">
                            <div class="w-2.5 h-2.5 rounded-full bg-green-500"></div> {{ t('user_profile.groups.group_2') }}
                          </div>
                        </div>
                      </div>
                    </div>
                  </transition>

                  <button 
                    class="w-10 h-10 rounded-2xl flex items-center justify-center bg-[var(--theme-surface)] border border-border-soft text-[var(--theme-text-strong)] hover:bg-[var(--theme-surface)]-hover transition-all shadow-lg hover:-translate-y-0.5 more-menu-container"
                    @click.stop="showMoreMenu = !showMoreMenu"
                  >
                    <MoreHorizontal :size="18" />
                  </button>
                  
                  <transition name="fade">
                    <div v-if="showMoreMenu" class="absolute top-12 right-0 w-56 glass-panel border-border-soft rounded-2xl shadow-2xl py-2 z-50 overflow-hidden text-[13px] more-menu-container">
                      <div class="px-4 py-2 hover:bg-[var(--theme-surface)]-hover cursor-pointer flex items-center gap-3 text-[var(--theme-text-strong)] transition-all" @click="executeAction('copy_id')">
                        <Copy :size="16" /> {{ t('user_profile.actions.copy_id') }}
                      </div>
                      <div class="px-4 py-2 hover:bg-[var(--theme-surface)]-hover cursor-pointer flex items-center gap-3 text-[var(--theme-text-strong)] transition-all" @click="executeAction('copy_vrc_url')">
                        <Share2 :size="16" /> {{ t('user_profile.actions.copy_vrc_url') }}
                      </div>
                      <div class="px-4 py-2 hover:bg-[var(--theme-surface)]-hover cursor-pointer flex items-center gap-3 text-[var(--theme-text-strong)] transition-all" @click="executeAction('view_on_vrc')">
                        <ExternalLink :size="16" /> {{ t('user_profile.actions.view_on_vrc') }}
                      </div>
                      <div class="h-[1px] bg-border-soft my-1"></div>
                      <div class="px-4 py-2 hover:bg-red-500/10 cursor-pointer flex items-center gap-3 text-red-500 transition-all" @click="executeAction('block')">
                        <ShieldBan :size="16" /> {{ t('user_profile.actions.block') }}
                      </div>
                    </div>
                  </transition>
                </div>
              </div>

              <!-- Navigation Tabs -->
              <div class="flex gap-2 mt-8 overflow-x-auto no-scrollbar relative z-30">
                <button 
                  v-for="tab in [
                    { id: 'info', label: t('user_profile.tabs.info'), icon: Info },
                    { id: 'social', label: t('user_profile.tabs.social'), icon: Users, hide: !profileStore.baseInfo?.isFriend && !isSelf },
                    { id: 'groups', label: t('user_profile.tabs.groups'), icon: Globe, count: profileStore.groups.length },
                    { id: 'mutual', label: t('user_profile.tabs.mutual'), icon: Users, hide: isSelf },
                    { id: 'created_worlds', label: t('user_profile.tabs.worlds'), icon: Map },
                    { id: 'created_avatars', label: t('user_profile.tabs.avatars'), icon: Cuboid },
                    { id: 'activity', label: t('user_profile.tabs.activity'), icon: History },
                    { id: 'raw_json', label: t('user_profile.tabs.raw'), icon: Code }
                  ]"
                  :key="tab.id"
                  v-show="!tab.hide"
                  class="px-5 py-2.5 text-[12px] font-black rounded-xl transition-all flex items-center gap-2 border whitespace-nowrap active:scale-95"
                  :class="activeTab === tab.id 
                    ? 'bg-primary text-white border-primary shadow-xl shadow-primary/30' 
                    : 'bg-[var(--theme-surface)] text-[var(--theme-text-muted)] border-border-soft hover:bg-[var(--theme-surface)]-hover hover:text-[var(--theme-text-strong)]'"
                  @click="activeTab = tab.id as any"
                >
                  <component :is="tab.icon" :size="16" />
                  {{ tab.label }}
                  <span v-if="tab.count !== undefined" class="ml-1 opacity-50">{{ tab.count }}</span>
                </button>
              </div>
            </div>
          </div>

          <!-- Content Body -->
          <div class="flex-1 overflow-y-auto custom-scrollbar p-8 bg-[var(--theme-surface)]/40 relative">
            
            <!-- Info Tab -->
            <template v-if="activeTab === 'info'">
              <div class="grid grid-cols-1 md:grid-cols-3 gap-8">
                <!-- Main Info Column -->
                <div class="md:col-span-2 space-y-8">
                  <!-- Biography -->
                  <div class="glass-panel border-border-soft p-6 rounded-3xl shadow-xl hover:shadow-2xl transition-shadow relative overflow-hidden group">
                    <div class="absolute top-0 right-0 w-32 h-32 bg-primary/5 rounded-bl-full transition-transform group-hover:scale-110"></div>
                    <h3 class="text-[13px] font-black text-[var(--theme-text-muted)] uppercase tracking-widest mb-4 flex items-center gap-2">
                      <AlignLeft class="w-4 h-4 text-primary" /> {{ t('user_profile.sections.biography') }}
                    </h3>
                    <p class="text-[14px] leading-relaxed text-[var(--theme-text-strong)] whitespace-pre-wrap font-medium">{{ profileStore.baseInfo?.bio || t('auto_37e93fbb') }}</p>
                  </div>
                </div>

                <!-- Right Column: Details & JSON -->
                <div class="space-y-6">
                  <!-- Details Card -->
                  <div class="glass-panel border-border-soft rounded-3xl p-6 shadow-xl relative overflow-hidden">
                    <h3 class="text-[13px] font-black text-[var(--theme-text-muted)] uppercase tracking-widest mb-4 flex items-center gap-2">
                      <Info class="w-4 h-4 text-primary" /> {{ t('user_profile.sections.details') }}
                    </h3>
                    <div class="space-y-4">
                      <div class="flex flex-col gap-1 border-b border-border-soft pb-3 last:border-0 last:pb-0">
                        <span class="text-[11px] font-bold text-[var(--theme-text-muted)] uppercase tracking-wider">{{ t('user_profile.details.id') }}</span>
                        <div class="flex items-center justify-between">
                          <span class="text-[13px] font-mono text-[var(--theme-text-strong)] truncate mr-2">{{ profileStore.baseInfo?.id }}</span>
                        </div>
                      </div>
                      
                      <div class="flex flex-col gap-1 border-b border-border-soft pb-3 last:border-0 last:pb-0">
                        <span class="text-[11px] font-bold text-[var(--theme-text-muted)] uppercase tracking-wider">{{ t('user_profile.details.joined') }}</span>
                        <span class="text-[13px] text-[var(--theme-text-strong)] font-medium">{{ formatDate(profileStore.baseInfo?.date_joined) }}</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </template>

            <!-- Social / Mutual Tab -->
            <template v-else-if="activeTab === 'social' || activeTab === 'mutual'">
              <div class="space-y-6">
                <div class="flex items-center justify-between">
                  <h3 class="text-[13px] font-black text-[var(--theme-text-muted)] uppercase tracking-widest flex items-center gap-2">
                    <Users class="w-4 h-4 text-primary" /> {{ activeTab === 'social' ? t('user_profile.tabs.social') : t('user_profile.tabs.mutual') }}
                  </h3>
                </div>
                
                <div v-if="profileStore.isLoadingMutual" class="flex justify-center p-8 text-primary">
                  <RefreshCcw class="animate-spin w-8 h-8" />
                </div>
                <div v-else-if="profileStore.mutualFriends.length === 0" class="flex flex-col items-center justify-center p-12 opacity-50">
                  <Users class="w-16 h-16 mb-4" />
                  <p class="font-bold">{{ t('global.empty', 'Empty') }}</p>
                </div>
                <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                  <div 
                    v-for="friend in profileStore.mutualFriends" 
                    :key="friend.id"
                    class="bg-[var(--theme-surface)] hover:bg-[var(--theme-surface)]-hover border border-border-soft p-3 rounded-2xl flex items-center gap-3 cursor-pointer transition-colors shadow-sm"
                    @click="profileStore.openProfile(friend.id, friend)"
                  >
                    <div class="w-12 h-12 rounded-xl overflow-hidden shrink-0 bg-black/20">
                      <VrcAvatar :user="friend" :url="friend.currentAvatarThumbnailImageUrl" custom-class="w-full h-full object-cover" />
                    </div>
                    <div class="min-w-0 flex-1">
                      <div class="font-bold text-sm text-[var(--theme-text-strong)] truncate">{{ friend.displayName }}</div>
                      <div class="text-xs text-[var(--theme-text-muted)] truncate">{{ friend.statusDescription || friend.status }}</div>
                    </div>
                  </div>
                </div>
              </div>
            </template>

            <!-- Groups Tab -->
            <template v-else-if="activeTab === 'groups'">
              <div class="space-y-6">
                <div class="flex items-center justify-between">
                  <h3 class="text-[13px] font-black text-[var(--theme-text-muted)] uppercase tracking-widest flex items-center gap-2">
                    <Globe class="w-4 h-4 text-primary" /> {{ t('user_profile.tabs.groups') }}
                  </h3>
                </div>
                
                <div v-if="profileStore.isLoadingGroups" class="flex justify-center p-8 text-primary">
                  <RefreshCcw class="animate-spin w-8 h-8" />
                </div>
                <div v-else-if="profileStore.groups.length === 0" class="flex flex-col items-center justify-center p-12 opacity-50">
                  <Globe class="w-16 h-16 mb-4" />
                  <p class="font-bold">{{ t('global.empty', 'Empty') }}</p>
                </div>
                <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                  <VrcResourceCard 
                    v-for="group in profileStore.groups" 
                    :key="group.id" 
                    type="group" 
                    :data="group" 
                  />
                </div>
              </div>
            </template>

            <!-- Created Worlds Tab -->
            <template v-else-if="activeTab === 'created_worlds'">
              <div class="space-y-6">
                <div class="flex items-center justify-between">
                  <h3 class="text-[13px] font-black text-[var(--theme-text-muted)] uppercase tracking-widest flex items-center gap-2">
                    <Map class="w-4 h-4 text-primary" /> {{ t('user_profile.tabs.worlds') }}
                  </h3>
                </div>
                
                <div v-if="profileStore.isLoadingWorlds" class="flex justify-center p-8 text-primary">
                  <RefreshCcw class="animate-spin w-8 h-8" />
                </div>
                <div v-else-if="profileStore.createdWorlds.length === 0" class="flex flex-col items-center justify-center p-12 opacity-50">
                  <Map class="w-16 h-16 mb-4" />
                  <p class="font-bold">{{ t('global.empty', 'Empty') }}</p>
                </div>
                <div v-else class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
                  <VrcResourceCard 
                    v-for="world in profileStore.createdWorlds" 
                    :key="world.id" 
                    type="world" 
                    :data="world" 
                  />
                </div>
              </div>
            </template>

            <!-- Created Avatars Tab -->
            <template v-else-if="activeTab === 'created_avatars'">
              <div class="space-y-6">
                <div class="flex items-center justify-between">
                  <h3 class="text-[13px] font-black text-[var(--theme-text-muted)] uppercase tracking-widest flex items-center gap-2">
                    <Cuboid class="w-4 h-4 text-primary" /> {{ t('user_profile.tabs.avatars') }}
                  </h3>
                </div>
                
                <div v-if="profileStore.isLoadingAvatars" class="flex justify-center p-8 text-primary">
                  <RefreshCcw class="animate-spin w-8 h-8" />
                </div>
                <div v-else-if="profileStore.createdAvatars.length === 0" class="flex flex-col items-center justify-center p-12 opacity-50">
                  <Cuboid class="w-16 h-16 mb-4" />
                  <p class="font-bold">{{ t('global.empty', 'Empty') }}</p>
                </div>
                <div v-else class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
                  <VrcResourceCard 
                    v-for="avatar in profileStore.createdAvatars" 
                    :key="avatar.id" 
                    type="avatar" 
                    :data="avatar" 
                  />
                </div>
              </div>
            </template>

            <!-- Activity / Notes Tab -->
            <template v-else-if="activeTab === 'activity'">
              <div class="space-y-6">
                <div class="flex items-center justify-between">
                  <h3 class="text-[13px] font-black text-[var(--theme-text-muted)] uppercase tracking-widest flex items-center gap-2">
                    <History class="w-4 h-4 text-primary" /> {{ t('user_profile.tabs.activity') }}
                  </h3>
                </div>
                
                <div class="glass-panel border-border-soft rounded-3xl p-6 shadow-xl relative overflow-hidden group">
                  <div class="flex items-center justify-between mb-4">
                    <h3 class="text-[13px] font-black text-[var(--theme-text-muted)] uppercase tracking-widest flex items-center gap-2">
                      <PencilLine class="w-4 h-4 text-primary" /> {{ t('user_profile.sections.personal_memo') }}
                    </h3>
                    <button 
                      v-if="!isEditingNote" 
                      @click="isEditingNote = true" 
                      class="text-xs font-bold px-3 py-1.5 bg-primary/20 text-primary rounded-lg hover:bg-primary/30 transition-colors"
                    >
                      {{ t('user_profile.actions.edit_memo') }}
                    </button>
                    <div v-else class="flex gap-2">
                      <button @click="isEditingNote = false" class="text-xs font-bold px-3 py-1.5 bg-border-soft text-text-muted rounded-lg hover:bg-border-strong transition-colors">{{ t('common.cancel') }}</button>
                      <button @click="saveLocalNote" class="text-xs font-bold px-3 py-1.5 bg-primary text-white rounded-lg hover:bg-primary/80 transition-colors flex items-center gap-1">
                        <Save class="w-3 h-3" /> {{ t('common.save') }}
                      </button>
                    </div>
                  </div>
                  
                  <div v-if="!isEditingNote" class="min-h-[60px] p-4 bg-[var(--theme-surface)]/60 rounded-xl border border-[var(--theme-border-soft)] text-sm whitespace-pre-wrap font-medium">
                    {{ localNote || t('user_profile.info.no_memo') }}
                  </div>
                  <textarea 
                    v-else 
                    v-model="localNote" 
                    rows="4" 
                    class="w-full p-4 bg-[var(--theme-surface)] border border-primary/50 focus:border-primary rounded-xl text-sm text-[var(--theme-text-strong)] focus:outline-none transition-colors"
                    :placeholder="t('user_profile.info.write_memo')"
                  ></textarea>
                </div>

                <div class="glass-panel border-border-soft rounded-3xl p-6 shadow-xl relative overflow-hidden">
                  <h3 class="text-[13px] font-black text-[var(--theme-text-muted)] uppercase tracking-widest mb-4 flex items-center gap-2">
                    <Clock class="w-4 h-4 text-primary" /> {{ t('user_profile.sections.recent_activity') }}
                  </h3>
                  <div v-if="profileStore.isLoadingActivity" class="flex justify-center p-4 text-primary">
                    <RefreshCcw class="animate-spin w-6 h-6" />
                  </div>
                  <div v-else-if="profileStore.activityLogs.length === 0" class="text-sm text-text-muted italic">
                    {{ t('user_profile.info.no_activity') }}
                  </div>
                  <div v-else class="space-y-3 max-h-[300px] overflow-y-auto custom-scrollbar pr-2">
                    <div v-for="(log, index) in profileStore.activityLogs" :key="index" class="flex items-start gap-3 p-3 bg-[var(--theme-surface)]/60 rounded-xl border border-border-soft">
                      <div class="w-8 h-8 rounded-full bg-primary/20 flex items-center justify-center shrink-0">
                        <History class="w-4 h-4 text-primary" />
                      </div>
                      <div>
                        <div class="text-sm font-bold text-[var(--theme-text-strong)]">{{ log.type }}</div>
                        <div class="text-xs text-[var(--theme-text-muted)]">{{ formatTime(log.created_at) }}</div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </template>

            <!-- Raw JSON Tab -->
            <template v-else-if="activeTab === 'raw_json'">
              <div class="glass-panel border-border-soft rounded-3xl p-6 shadow-xl relative overflow-hidden flex flex-col max-h-[400px]">
                <div class="flex items-center justify-between mb-4">
                  <h3 class="text-[13px] font-black text-[var(--theme-text-muted)] uppercase tracking-widest flex items-center gap-2">
                    <Code class="w-4 h-4 text-primary" /> {{ t('user_profile.sections.json_data') }}
                  </h3>
                </div>
                <div class="bg-[var(--theme-surface)]/60 rounded-xl p-3 overflow-auto custom-scrollbar flex-1 border border-border-soft shadow-inner">
                  <pre class="text-[11px] font-mono leading-relaxed" v-html="highlightJson(profileStore.baseInfo)"></pre>
                </div>
              </div>
            </template>

          </div>

          <!-- Close Button -->
          <button 
            @click="profileStore.closeProfile"
            class="absolute top-6 right-6 w-12 h-12 bg-[var(--theme-surface)]/50 hover:bg-red-500/20 text-[var(--theme-text-muted)] hover:text-red-500 rounded-full flex items-center justify-center transition-all border border-[var(--theme-border-soft)] z-[110] active:scale-95"
          >
            <X :size="24" />
          </button>
        </div>
      </transition>
    </div>
  </transition>

  <!-- Fullscreen Image Preview -->
  <transition name="fade">
    <div v-if="showImagePreview" class="fixed inset-0 bg-black/95 z-[1000] flex items-center justify-center backdrop-blur-3xl" @click="toggleImagePreview">
      <div class="relative w-full h-full flex flex-col items-center justify-center p-10" @click.stop>
        <div class="absolute top-10 right-10 flex gap-4">
          <button @click="handleResetImage" class="p-4 bg-[var(--theme-surface)]/50 hover:bg-[var(--theme-surface)]/80 rounded-2xl text-[var(--theme-text-muted)] transition-all border border-[var(--theme-border-soft)]"><RotateCcw :size="20" /></button>
          <button @click="toggleImagePreview" class="p-4 bg-[var(--theme-surface)]/50 hover:bg-red-500/20 rounded-2xl text-[var(--theme-text-muted)] hover:text-red-500 transition-all border border-[var(--theme-border-soft)]"><X :size="24" /></button>
        </div>
        
        <div class="flex-1 flex items-center justify-center overflow-hidden w-full h-full">
           <img 
            :src="profileStore.baseInfo?.profilePicOverride || profileStore.baseInfo?.currentAvatarImageUrl || profileStore.baseInfo?.currentAvatarThumbnailImageUrl" 
            class="max-w-[85vw] max-h-[75vh] object-contain shadow-[0_0_100px_rgba(0,0,0,0.8)] transition-all duration-300"
            :style="{ transform: `scale(${imageScale}) rotate(${imageRotation}deg)` }"
          />
        </div>

        <div class="mt-10 flex gap-4 p-5 bg-[var(--theme-surface)]/50 rounded-3xl border border-[var(--theme-border-soft)] backdrop-blur-xl">
          <button @click="handleZoomOut" class="p-4 hover:bg-[var(--theme-surface)]/80 rounded-2xl text-[var(--theme-text-muted)] transition-all"><ZoomOut :size="24" /></button>
          <div class="w-[1px] bg-[var(--theme-border-soft)]"></div>
          <button @click="handleZoomIn" class="p-4 hover:bg-[var(--theme-surface)]/80 rounded-2xl text-[var(--theme-text-muted)] transition-all"><ZoomIn :size="24" /></button>
          <div class="w-[1px] bg-[var(--theme-border-soft)]"></div>
          <button @click="handleRotateCw" class="p-4 hover:bg-[var(--theme-surface)]/80 rounded-2xl text-[var(--theme-text-muted)] transition-all"><RotateCw :size="24" /></button>
          <div class="w-[1px] bg-[var(--theme-border-soft)]"></div>
          <button @click="handleCopyImage" class="p-4 hover:bg-[var(--theme-surface)]/80 rounded-2xl text-[var(--theme-text-muted)] transition-all"><Copy :size="24" /></button>
          <div class="w-[1px] bg-[var(--theme-border-soft)]"></div>
          <button @click="handleDownloadImage" class="p-4 hover:bg-[var(--theme-surface)]/80 rounded-2xl text-[var(--theme-text-muted)] transition-all"><Download :size="24" /></button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.glass-panel {
  background: rgba(23, 25, 30, 0.75);
  backdrop-filter: blur(40px) saturate(180%);
}








.fade-enter-active, .fade-leave-active { transition: opacity 0.4s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }

.scale-enter-active, .scale-leave-active { transition: all 0.5s cubic-bezier(0.16, 1, 0.3, 1); }
.scale-enter-from, .scale-leave-to { opacity: 0; transform: scale(0.9) translateY(20px); }

.dropdown-enter-active, .dropdown-leave-active { transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1); }
.dropdown-enter-from, .dropdown-leave-to { opacity: 0; transform: translateY(-10px) scale(0.95); }

pre span.text-primary { color: var(--color-primary); }
pre span.text-text-muted { color: rgba(255,255,255,0.4); }
pre span.text-emerald-400 { color: #34d399; }
pre span.text-blue-400 { color: #60a5fa; }
pre span.text-red-400 { color: #f87171; }
</style>
