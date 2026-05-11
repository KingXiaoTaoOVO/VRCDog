<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { 
  X, MoreHorizontal, Star, Monitor, Headset, Copy, RefreshCcw, Share2, 
  LogIn, Mail, Hand, MessageSquarePlus, Settings, PenLine, User, Users, 
  History, Ban, VolumeX, MessageSquareOff, Eye, EyeOff, ShieldBan, Flag, 
  UserMinus, Search, ChevronDown, Download, Check, ZoomIn, ZoomOut, RotateCcw, RotateCw, Globe, PencilLine, Save, Languages, Image, Heart, Map, Cuboid, LogOut,
  Shield, ExternalLink, Info, Database, Clock, Calendar, FileJson
} from 'lucide-vue-next';
import { useUserProfileStore } from '../stores/userProfile';
import { useToast } from '../composables/useToast';
import { VrcApi, DbApi } from '../api';

const { t } = useI18n();
const profileStore = useUserProfileStore();
const toast = useToast();

const activeTab = ref<'info' | 'mutual' | 'groups' | 'created_worlds' | 'fav_worlds' | 'created_avatars' | 'activity' | 'raw_json'>('info');
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
        toast.success(t('user_profile.actions.refresh_success') || '刷新成功');
        break;
      case 'copy_id':
        navigator.clipboard.writeText(userId);
        toast.success(t('user_profile.actions.copy_id_success') || 'ID 已复制');
        break;
      case 'copy_vrc_url':
        navigator.clipboard.writeText('https://vrchat.com/home/user/' + userId);
        toast.success(t('user_profile.actions.copy_url_success') || '链接已复制');
        break;
      case 'view_on_vrc':
        window.open('https://vrchat.com/home/user/' + userId, '_blank');
        break;
      case 'request_invite':
        await VrcApi.requestInvite(userId);
        toast.success(t('user_profile.actions.request_invite_success') || '请求已发送');
        break;
      case 'invite':
        await VrcApi.inviteUser(userId);
        toast.success(t('user_profile.actions.invite_success') || '邀请已发送');
        break;
      case 'unfriend':
        if (confirm(t('user_profile.actions.unfriend_confirm') || '确定要解除好友关系吗？')) {
          await VrcApi.unfriend({ userId });
          toast.success(t('user_profile.actions.unfriend_success') || '好友关系已解除');
          profileStore.closeProfile();
        }
        break;
      case 'block':
        await VrcApi.moderateUser({ moderated: userId, type: 'block' });
        toast.success(t('user_profile.actions.block_success') || '已封禁');
        break;
      case 'mute':
        await VrcApi.moderateUser({ moderated: userId, type: 'mute' });
        toast.success(t('user_profile.actions.mute_success') || '已静音');
        break;
      case 'showAvatar':
        await VrcApi.moderateUser({ moderated: userId, type: 'showAvatar' });
        toast.success(t('user_profile.actions.show_avatar_success') || '已显示头像');
        break;
      case 'hideAvatar':
        await VrcApi.moderateUser({ moderated: userId, type: 'hideAvatar' });
        toast.success(t('user_profile.actions.hide_avatar_success') || '已隐藏头像');
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
  translatedBio.value = "【翻译结果】" + (profileStore.baseInfo?.bio || '');
  isTranslating.value = false;
};

const saveLocalNote = async () => {
  profileStore.localNote = localNote.value;
  await profileStore.saveLocalNote();
  isEditingNote.value = false;
  toast.success(t('user_profile.info.memo_saved') || '备注已保存');
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
    toast.success(t('user_profile.actions.copy_image_success') || '图片已复制');
  } catch (err) {
    toast.error('无法复制图片');
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
  switch (status) {
    case 'active': return '#22c55e';
    case 'join me': return '#3b82f6';
    case 'ask me': return '#f97316';
    case 'busy': return '#ef4444';
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
          <div class="relative shrink-0 border-b border-white/5 bg-transparent z-20 overflow-hidden">
            <!-- Banner Background -->
            <div class="absolute inset-0 z-0 h-48 overflow-hidden">
              <div class="absolute inset-0 bg-gradient-to-b from-black/20 via-background/60 to-background z-10"></div>
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
                  class="w-40 h-40 shrink-0 bg-background/40 backdrop-blur-md rounded-3xl flex items-center justify-center overflow-hidden border border-white/10 cursor-pointer group relative shadow-2xl z-20"
                  @click="toggleImagePreview"
                >
                  <img 
                    v-if="profileStore.baseInfo?.profilePicOverride || profileStore.baseInfo?.currentAvatarThumbnailImageUrl || profileStore.baseInfo?.currentAvatarImageUrl"
                    :src="profileStore.baseInfo?.profilePicOverride || profileStore.baseInfo?.currentAvatarThumbnailImageUrl || profileStore.baseInfo?.currentAvatarImageUrl"
                    class="w-full h-full object-cover transition-transform duration-700 group-hover:scale-110"
                  />
                  <div class="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center backdrop-blur-sm">
                    <Search class="w-8 h-8 text-white/80" />
                  </div>
                  <!-- Status Circle -->
                  <div class="absolute bottom-3 right-3 w-6 h-6 rounded-full border-4 border-background shadow-lg" :style="{ backgroundColor: getStatusColor(profileStore.baseInfo?.status || '') }"></div>
                </div>

                <!-- Basic Info -->
                <div class="flex-1 min-w-0 pb-1">
                  <div class="flex items-start justify-between">
                    <div>
                      <div class="flex items-center gap-3 mb-2">
                        <h1 class="text-4xl font-black text-white tracking-tight truncate drop-shadow-lg">
                          {{ profileStore.baseInfo?.displayName || t('user_profile.loading') }}
                        </h1>
                        
                        <!-- Trust Level Badge -->
                        <span 
                          class="px-3 py-1 rounded-full text-[10px] font-black uppercase tracking-widest flex items-center gap-2 border shadow-lg ml-2 bg-background/40 backdrop-blur-md"
                          :style="{ color: trustColor, borderColor: trustColor + '40' }"
                        >
                          <Shield :size="12" />
                          {{ trustName }}
                        </span>
                        
                        <!-- Friend Icon -->
                        <Users v-if="profileStore.baseInfo?.isFriend" :size="18" class="text-emerald-400 drop-shadow-lg" />
                      </div>
                      
                      <div class="flex items-center gap-4">
                        <div class="text-[14px] text-white/60 font-bold font-mono tracking-wide flex items-center gap-2">
                          <span class="opacity-80">@{{ profileStore.baseInfo?.username || profileStore.baseInfo?.displayName || '' }}</span>
                          <span class="px-2 py-0.5 rounded-lg bg-white/5 border border-white/5 text-[11px] cursor-pointer hover:bg-white/10 transition-all flex items-center gap-1" @click="executeAction('copy_id')">
                            <Copy :size="12" /> {{ profileStore.baseInfo?.id?.substring(0, 12) }}...
                          </span>
                        </div>
                        <div v-if="profileStore.baseInfo?.pronouns" class="text-[12px] text-primary font-black uppercase tracking-widest px-3 py-1 bg-primary/10 rounded-lg border border-primary/20">
                          {{ profileStore.baseInfo.pronouns }}
                        </div>
                      </div>
                    </div>

                    <!-- Action Buttons -->
                    <div class="flex gap-3 more-menu-container relative z-30">
                      <button @click="executeAction('refresh')" class="w-11 h-11 rounded-2xl bg-white/5 hover:bg-white/10 backdrop-blur-md flex items-center justify-center text-white/70 transition-all border border-white/10 shadow-lg hover:text-primary active:scale-95">
                        <RefreshCcw :size="20" :class="{'animate-spin': profileStore.isLoadingBase}" />
                      </button>
                      <button 
                        v-if="!isSelf"
                        class="w-11 h-11 rounded-2xl bg-white/5 hover:bg-white/10 backdrop-blur-md flex items-center justify-center transition-all border border-white/10 shadow-lg active:scale-95"
                        :class="profileStore.isFavorite ? 'text-pink-500' : 'text-white/70 hover:text-pink-500'"
                        @click="profileStore.toggleFavorite"
                      >
                        <Heart :size="20" :fill="profileStore.isFavorite ? 'currentColor' : 'none'" />
                      </button>
                      <button 
                        v-if="!isSelf"
                        class="w-11 h-11 rounded-2xl bg-white/5 hover:bg-white/10 backdrop-blur-md flex items-center justify-center text-white/70 transition-all border border-white/10 shadow-lg hover:text-primary active:scale-95"
                        @click="showMoreMenu = !showMoreMenu"
                      >
                        <MoreHorizontal :size="20" />
                      </button>

                      <!-- Action Dropdown -->
                      <transition name="dropdown">
                        <div v-if="showMoreMenu" class="absolute top-14 right-0 w-60 bg-surface/90 backdrop-blur-3xl rounded-2xl py-3 border border-white/10 shadow-2xl z-[100] overflow-hidden text-[13px] font-bold">
                          <div class="px-4 py-2 hover:bg-white/5 cursor-pointer flex items-center gap-3 text-white/80 hover:text-primary transition-all" @click="executeAction('copy_id')">
                            <Copy :size="16" /> {{ t('user_profile.actions.copy_id') }}
                          </div>
                          <div class="px-4 py-2 hover:bg-white/5 cursor-pointer flex items-center gap-3 text-white/80 hover:text-primary transition-all" @click="executeAction('copy_vrc_url')">
                            <Share2 :size="16" /> {{ t('user_profile.actions.copy_vrc_url') }}
                          </div>
                          <div class="px-4 py-2 hover:bg-white/5 cursor-pointer flex items-center gap-3 text-white/80 hover:text-primary transition-all" @click="executeAction('view_on_vrc')">
                            <ExternalLink :size="16" /> {{ t('user_profile.actions.view_on_vrc') }}
                          </div>
                          <div class="h-[1px] bg-white/5 my-2"></div>
                          <div v-if="profileStore.baseInfo?.isFriend" class="px-4 py-2 hover:bg-red-500/10 cursor-pointer flex items-center gap-3 text-red-400 transition-all" @click="executeAction('unfriend')">
                            <UserMinus :size="16" /> {{ t('user_profile.actions.unfriend') }}
                          </div>
                          <div v-else class="px-4 py-2 hover:bg-primary/10 cursor-pointer flex items-center gap-3 text-primary transition-all">
                            <Users :size="16" /> {{ t('user_profile.actions.add_friend') }}
                          </div>
                          <div class="h-[1px] bg-white/5 my-2"></div>
                          <div class="px-4 py-2 hover:bg-red-500/10 cursor-pointer flex items-center gap-3 text-red-500 transition-all" @click="executeAction('block')">
                            <ShieldBan :size="16" /> {{ t('user_profile.actions.block') }}
                          </div>
                          <div class="px-4 py-2 hover:bg-white/5 cursor-pointer flex items-center gap-3 text-white/80 transition-all" @click="executeAction('mute')">
                            <VolumeX :size="16" /> {{ t('user_profile.actions.mute') }}
                          </div>
                        </div>
                      </transition>
                    </div>
                  </div>
                  
                  <!-- Platform & Status Indicators -->
                  <div class="flex items-center gap-3 mt-5 flex-wrap">
                    <span v-if="profileStore.baseInfo?.last_platform" class="px-4 py-1.5 rounded-xl bg-white/5 border border-white/10 text-white/80 text-[11px] font-black uppercase tracking-widest shadow-lg flex items-center gap-2 backdrop-blur-md">
                      <Monitor class="w-3.5 h-3.5 text-primary" /> {{ profileStore.baseInfo.last_platform }}
                    </span>
                    <span v-if="profileStore.baseInfo?.statusDescription" class="px-4 py-1.5 rounded-xl bg-white/5 border border-white/10 text-white/80 text-[12px] font-bold shadow-lg flex items-center gap-2 backdrop-blur-md max-w-[450px] truncate italic opacity-90">
                       "{{ profileStore.baseInfo.statusDescription }}"
                    </span>
                    <div v-if="profileStore.baseInfo?.languages?.length" class="flex gap-1">
                      <span v-for="lang in profileStore.baseInfo.languages" :key="lang" class="px-3 py-1 rounded-lg bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 text-[10px] font-black uppercase tracking-widest">
                        {{ lang }}
                      </span>
                    </div>
                  </div>
                </div>
              </div>

              <!-- Navigation Tabs -->
              <div class="flex gap-2 mt-10 overflow-x-auto no-scrollbar relative z-30">
                <button 
                  v-for="tab in [
                    { id: 'info', label: t('user_profile.tabs.info'), icon: Info },
                    { id: 'social', label: t('user_profile.tabs.social'), icon: Users, hide: !profileStore.baseInfo?.isFriend && !isSelf },
                    { id: 'groups', label: t('user_profile.tabs.groups'), icon: Globe, count: profileStore.groups.length },
                    { id: 'mutual', label: t('user_profile.tabs.mutual'), icon: Users, hide: isSelf },
                    { id: 'created_worlds', label: t('user_profile.tabs.worlds'), icon: Map },
                    { id: 'created_avatars', label: t('user_profile.tabs.avatars'), icon: Cuboid },
                    { id: 'activity', label: t('user_profile.tabs.activity'), icon: History },
                    { id: 'raw_json', label: t('user_profile.tabs.raw'), icon: FileJson }
                  ]"
                  :key="tab.id"
                  v-show="!tab.hide"
                  class="px-5 py-2.5 text-[12px] font-black rounded-xl transition-all flex items-center gap-2 border whitespace-nowrap active:scale-95"
                  :class="activeTab === tab.id 
                    ? 'bg-primary text-white border-primary shadow-xl shadow-primary/30' 
                    : 'bg-white/5 text-white/40 border-white/5 hover:bg-white/10 hover:text-white/80'"
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
          <div class="flex-1 overflow-y-auto custom-scrollbar p-8 bg-transparent relative z-10">
            
            <!-- Info Tab -->
            <template v-if="activeTab === 'info'">
              <!-- Current Location Banner -->
              <div v-if="profileStore.baseInfo?.location && profileStore.baseInfo?.status !== 'offline'" class="mb-10 p-6 bg-white/5 border border-white/10 rounded-3xl flex items-center justify-between shadow-2xl backdrop-blur-xl group hover:border-primary/40 transition-all">
                <div class="flex items-center gap-6">
                  <div class="w-20 h-20 rounded-2xl bg-black/40 flex items-center justify-center border border-white/10 shrink-0 overflow-hidden relative shadow-inner">
                    <img v-if="profileStore.baseInfo.location === 'private'" src="https://images.unsplash.com/photo-1550684848-fac1c5b4e853?auto=format&fit=crop&w=200&q=80" class="w-full h-full object-cover opacity-20" />
                    <img v-else :src="profileStore.baseInfo.currentAvatarThumbnailImageUrl || 'https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?auto=format&fit=crop&w=200&q=80'" class="w-full h-full object-cover group-hover:scale-110 transition-transform duration-1000" />
                    <div class="absolute inset-0 bg-gradient-to-t from-black/60 to-transparent"></div>
                  </div>
                  <div>
                    <div class="flex items-center gap-3 mb-2">
                      <span class="text-lg font-black text-white tracking-tight">{{ profileStore.baseInfo.location === 'private' ? t('status_card.private') : profileStore.baseInfo.location }}</span>
                    </div>
                    <div class="text-[12px] text-white/50 flex items-center gap-4 font-bold tracking-wide">
                      <span class="flex items-center gap-2"><Globe :size="14" class="text-emerald-400" /> Active Instance</span>
                    </div>
                  </div>
                </div>
                <div v-if="!isSelf && profileStore.baseInfo.location !== 'private'" class="flex gap-3">
                  <button class="px-6 py-2.5 bg-primary hover:bg-primary-hover text-white border-none text-[13px] font-black rounded-xl transition-all flex items-center gap-2 shadow-xl shadow-primary/20 active:scale-95" @click="executeAction('request_invite')">
                    <LogIn :size="18" /> {{ t('user_profile.actions.request_invite') }}
                  </button>
                </div>
              </div>

              <div class="grid grid-cols-1 lg:grid-cols-3 gap-8">
                <!-- Main Info Column -->
                <div class="lg:col-span-2 space-y-8">
                  <!-- Memo / Local Note -->
                  <div class="bg-white/5 border border-white/10 rounded-3xl p-6 shadow-xl backdrop-blur-xl relative overflow-hidden group">
                    <div class="absolute top-0 left-0 w-1.5 h-full bg-primary opacity-40 group-hover:opacity-100 transition-opacity"></div>
                    <div class="flex items-center justify-between mb-5">
                      <h3 class="text-[11px] font-black text-white/80 uppercase tracking-[0.2em] flex items-center gap-3">
                        <PencilLine :size="16" class="text-primary" /> {{ t('user_profile.info.memo') }}
                      </h3>
                      <button v-if="!isEditingNote" @click="isEditingNote = true" class="px-4 py-1.5 bg-primary/10 text-primary hover:bg-primary hover:text-white text-[11px] font-black rounded-lg transition-all border border-primary/20 active:scale-95">
                        {{ t('search.save_note') }}
                      </button>
                    </div>
                    
                    <div v-if="isEditingNote" class="space-y-4">
                      <textarea 
                        v-model="localNote" 
                        class="w-full h-32 bg-black/20 border border-white/10 rounded-2xl p-5 text-[13px] text-white outline-none focus:border-primary/50 transition-all resize-none font-medium leading-relaxed"
                        :placeholder="t('search.local_note_placeholder')"
                      ></textarea>
                      <div class="flex justify-end gap-3">
                        <button @click="isEditingNote = false" class="px-4 py-2 bg-white/5 hover:bg-white/10 text-white/50 text-[12px] font-bold rounded-xl transition-all">{{ t('common.cancel') || '取消' }}</button>
                        <button @click="saveLocalNote" class="px-6 py-2 bg-primary text-white text-[12px] font-black rounded-xl transition-all flex items-center gap-2 shadow-lg shadow-primary/30"><Save :size="14" />{{ t('common.save') || '保存' }}</button>
                      </div>
                    </div>
                    <div v-else class="text-[14px] text-white/70 font-medium whitespace-pre-wrap leading-relaxed min-h-[40px] px-2 italic">
                      {{ localNote || t('user_profile.no_data') }}
                    </div>
                  </div>

                  <!-- Bio Card -->
                  <div class="bg-white/5 border border-white/10 rounded-3xl p-6 shadow-xl backdrop-blur-xl">
                    <div class="flex items-center justify-between mb-5">
                      <h3 class="text-[11px] font-black text-white/80 uppercase tracking-[0.2em] flex items-center gap-3">
                        <Languages :size="16" class="text-emerald-400" /> {{ t('user_profile.info.bio') }}
                      </h3>
                      <button @click="handleTranslate" class="px-4 py-1.5 bg-emerald-500/10 text-emerald-400 hover:bg-emerald-500 hover:text-white text-[11px] font-black rounded-lg transition-all border border-emerald-500/20 flex items-center gap-2 active:scale-95" :disabled="isTranslating">
                        <Globe :size="14" /> {{ translatedBio ? t('user_profile.info.bio') : t('user_profile.info.bio_translated') }}
                      </button>
                    </div>
                    <div class="text-[14px] text-white/80 font-medium whitespace-pre-wrap leading-relaxed px-2" :class="{'opacity-40 animate-pulse': isTranslating}">
                      {{ isTranslating ? t('user_profile.loading') : (translatedBio || profileStore.baseInfo?.bio || t('user_profile.no_data')) }}
                    </div>
                  </div>

                  <!-- Badges Section -->
                  <div v-if="profileStore.baseInfo?.badges?.length" class="bg-white/5 border border-white/10 rounded-3xl p-6 shadow-xl backdrop-blur-xl">
                    <h3 class="text-[11px] font-black text-white/80 uppercase tracking-[0.2em] flex items-center gap-3 mb-5">
                      <Shield :size="16" class="text-yellow-400" /> {{ t('user_profile.info.badges') }}
                    </h3>
                    <div class="flex flex-wrap gap-3">
                      <div v-for="badge in profileStore.baseInfo.badges" :key="badge.badgeId" class="group/badge relative">
                        <img :src="badge.badgeImageUrl" :alt="badge.badgeName" class="w-10 h-10 rounded-lg border border-white/10 hover:border-primary/50 transition-all cursor-help" />
                        <div class="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 px-3 py-1.5 bg-black/80 backdrop-blur-md rounded-lg text-[10px] text-white opacity-0 group-hover/badge:opacity-100 transition-opacity pointer-events-none whitespace-nowrap z-50 border border-white/10">
                          {{ badge.badgeName }}
                        </div>
                      </div>
                    </div>
                  </div>

                  <!-- Social Links -->
                  <div v-if="profileStore.socialLinks.length" class="bg-white/5 border border-white/10 rounded-3xl p-6 shadow-xl backdrop-blur-xl">
                    <h3 class="text-[11px] font-black text-white/80 uppercase tracking-[0.2em] flex items-center gap-3 mb-6">
                      <Share2 :size="16" class="text-blue-400" /> {{ t('user_profile.tabs.social_links') }}
                    </h3>
                    <div class="flex flex-wrap gap-4">
                      <a 
                        v-for="link in profileStore.socialLinks" 
                        :key="link.url" 
                        :href="link.url" 
                        target="_blank"
                        class="flex items-center gap-3 bg-white/5 px-4 py-2.5 rounded-2xl border border-white/5 hover:border-primary/40 hover:bg-primary/5 transition-all group/social"
                      >
                        <component 
                          :is="link.type === 'twitter' || link.type === 'x' ? Globe : link.type === 'youtube' ? Globe : link.type === 'twitch' ? Globe : link.type === 'github' ? Globe : link.type === 'discord' ? MessageSquarePlus : Globe" 
                          :size="16" 
                          class="text-white/40 group-hover/social:text-primary transition-colors" 
                        />
                        <span class="text-[13px] font-bold text-white/80 group-hover/social:text-white capitalize">{{ link.type }}</span>
                        <ExternalLink :size="12" class="text-white/20 group-hover/social:text-primary/60" />
                      </a>
                    </div>
                  </div>
                </div>


              <!-- Stats Sidebar -->
                <div class="space-y-6">
                  <div class="bg-white/5 border border-white/10 rounded-3xl p-6 shadow-xl backdrop-blur-xl relative overflow-hidden">
                    <div class="absolute top-0 right-0 w-24 h-24 bg-primary/10 rounded-full blur-3xl -mr-12 -mt-12"></div>
                    <h3 class="text-[10px] font-black text-white/30 mb-6 uppercase tracking-[0.3em]">{{ t('charts.overview') }}</h3>
                    <div class="space-y-6">
                      <div class="flex flex-col gap-1.5">
                        <div class="text-[9px] font-black text-white/30 uppercase tracking-[0.2em]">{{ t('user_profile.info.last_login') }}</div>
                        <div class="text-[13px] font-bold text-white/90 flex items-center gap-2"><Clock :size="14" class="text-primary" /> {{ formatTime(profileStore.baseInfo?.last_login) }}</div>
                      </div>

                      <div class="flex flex-col gap-1.5">
                        <div class="text-[9px] font-black text-white/30 uppercase tracking-[0.2em]">{{ t('user_profile.info.date_joined') }}</div>
                        <div class="text-[13px] font-bold text-white/90 flex items-center gap-2"><Calendar :size="14" class="text-emerald-400" /> {{ formatDate(profileStore.baseInfo?.date_joined) }}</div>
                      </div>

                      <div class="flex flex-col gap-1.5">
                        <div class="text-[9px] font-black text-white/30 uppercase tracking-[0.2em]">{{ t('user_profile.info.friend_key') }}</div>
                        <div class="text-[13px] font-mono text-primary font-bold tracking-wider">{{ profileStore.baseInfo?.friendKey || 'N/A' }}</div>
                      </div>
                    </div>
                  </div>

                  <div class="bg-white/5 border border-white/10 rounded-3xl p-6 shadow-xl backdrop-blur-xl">
                    <h3 class="text-[10px] font-black text-white/30 mb-5 uppercase tracking-[0.3em]">{{ t('user_profile.info.tags') }}</h3>
                    <div class="flex flex-wrap gap-2">
                      <span v-for="tag in profileStore.baseInfo?.tags?.slice(0, 10)" :key="tag" class="px-3 py-1 rounded-lg bg-white/5 border border-white/5 text-[9px] font-black text-white/40 uppercase tracking-widest hover:text-white/80 transition-colors">
                        {{ tag.replace('system_', '') }}
                      </span>
                    </div>
                  </div>
                </div>
              </div>
            </template>

            <!-- Mutual Friends Tab -->
            <!-- Mutual Tab -->
            <template v-else-if="activeTab === 'mutual'">
              <div class="space-y-10">
                <!-- Mutual Friends -->
                <section>
                  <div class="flex items-center gap-3 mb-6">
                    <Users :size="18" class="text-primary" />
                    <h3 class="text-[14px] font-black text-white uppercase tracking-wider">{{ t('user_profile.tabs.mutual') }} ({{ profileStore.mutualFriends.length }})</h3>
                  </div>
                  <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-5">
                    <div v-if="profileStore.isLoadingMutual" class="col-span-full py-10 flex flex-col items-center gap-4">
                      <RefreshCcw :size="32" class="animate-spin text-primary/40" />
                    </div>
                    <div v-else-if="profileStore.mutualFriends.length === 0" class="col-span-full py-10 text-center text-white/40 font-bold">
                      {{ t('user_profile.no_data') }}
                    </div>
                    <div v-for="friend in profileStore.mutualFriends" :key="friend.id" class="group/friend bg-white/5 border border-white/10 rounded-2xl p-4 flex flex-col items-center gap-4 hover:bg-white/10 hover:border-primary/40 cursor-pointer transition-all shadow-lg active:scale-95" @click="profileStore.openProfile(friend.id, friend)">
                      <div class="relative">
                        <img :src="friend.currentAvatarThumbnailImageUrl || friend.profilePicOverride || 'https://via.placeholder.com/128'" class="w-20 h-20 rounded-2xl object-cover border-2 border-white/5 group-hover/friend:border-primary/40 transition-all">
                        <div class="absolute -bottom-1 -right-1 w-5 h-5 border-4 border-background rounded-full" :style="{ backgroundColor: getStatusColor(friend.status) }"></div>
                      </div>
                      <div class="text-center w-full">
                        <div class="font-black text-[13px] text-white truncate group-hover/friend:text-primary transition-colors">{{ friend.displayName }}</div>
                        <div class="text-[10px] text-white/40 mt-1 font-bold uppercase tracking-widest">{{ friend.status }}</div>
                      </div>
                    </div>
                  </div>
                </section>

                <!-- Mutual Groups -->
                <section>
                  <div class="flex items-center gap-3 mb-6">
                    <Globe :size="18" class="text-emerald-400" />
                    <h3 class="text-[14px] font-black text-white uppercase tracking-wider">{{ t('user_profile.tabs.groups') }} ({{ profileStore.mutualGroups.length }})</h3>
                  </div>
                  <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                    <div v-if="profileStore.isLoadingMutualGroups" class="col-span-full py-10 flex justify-center">
                      <RefreshCcw :size="32" class="animate-spin text-emerald-500/40" />
                    </div>
                    <div v-else-if="profileStore.mutualGroups.length === 0" class="col-span-full py-10 text-center text-white/40 font-bold">
                      {{ t('user_profile.no_data') }}
                    </div>
                    <div v-for="group in profileStore.mutualGroups" :key="group.id" class="group bg-white/5 border border-white/10 rounded-3xl p-5 flex items-center gap-5 hover:bg-white/10 hover:border-emerald-500/40 cursor-pointer transition-all shadow-xl">
                      <img :src="group.iconUrl || group.bannerUrl || 'https://via.placeholder.com/128'" class="w-16 h-16 rounded-2xl object-cover border border-white/10 shadow-lg">
                      <div class="flex-1 min-w-0">
                        <div class="font-black text-[15px] text-white truncate group-hover:text-emerald-400 transition-colors">{{ group.name }}</div>
                        <div class="text-[11px] font-black text-white/40 uppercase tracking-widest mt-1">{{ group.memberCount || 0 }} Members</div>
                      </div>
                    </div>
                  </div>
                </section>
              </div>
            </template>

            <!-- Groups Tab -->
            <template v-else-if="activeTab === 'groups'">
              <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                <div v-if="profileStore.isLoadingGroups" class="col-span-full py-20 flex justify-center"><RefreshCcw :size="40" class="animate-spin text-primary/40" /></div>
                <div v-else-if="profileStore.groups.length === 0" class="col-span-full py-20 text-center text-white/40 font-bold">{{ t('user_profile.no_data') }}</div>
                <div v-for="group in profileStore.groups" :key="group.id" class="group bg-white/5 border border-white/10 rounded-3xl p-5 flex items-center gap-5 hover:bg-white/10 hover:border-emerald-500/40 cursor-pointer transition-all shadow-xl relative overflow-hidden">
                  <div class="absolute top-0 right-0 w-32 h-32 bg-emerald-500/5 rounded-full blur-3xl -mr-16 -mt-16"></div>
                  <img :src="group.iconUrl || group.bannerUrl || 'https://via.placeholder.com/128'" class="w-16 h-16 rounded-2xl object-cover border border-white/10 shadow-lg relative z-10">
                  <div class="flex-1 min-w-0 relative z-10">
                    <div class="font-black text-[15px] text-white truncate group-hover:text-emerald-400 transition-colors">{{ group.name }}</div>
                    <div class="flex items-center gap-4 mt-1.5">
                      <span class="text-[11px] font-black text-white/40 uppercase tracking-widest flex items-center gap-1.5"><Users :size="12" /> {{ group.memberCount || 0 }}</span>
                      <span class="px-2 py-0.5 rounded-lg bg-white/5 border border-white/5 text-[9px] font-black text-white/40 uppercase tracking-tighter">{{ group.shortCode }}</span>
                    </div>
                  </div>
                </div>
              </div>
            </template>

            <!-- Worlds Tab -->
            <template v-else-if="activeTab === 'created_worlds'">
              <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-8">
                <div v-if="profileStore.isLoadingWorlds" class="col-span-full py-20 flex justify-center"><RefreshCcw :size="40" class="animate-spin text-primary/40" /></div>
                <div v-else-if="profileStore.createdWorlds.length === 0" class="col-span-full py-20 text-center text-white/40 font-bold">{{ t('user_profile.no_data') }}</div>
                <div v-for="world in profileStore.createdWorlds" :key="world.id" class="group bg-white/5 border border-white/10 rounded-3xl overflow-hidden hover:border-primary/40 cursor-pointer transition-all shadow-2xl">
                  <div class="relative h-44">
                    <img :src="world.imageUrl || world.thumbnailImageUrl || 'https://via.placeholder.com/600x300'" class="w-full h-full object-cover transition-transform duration-1000 group-hover:scale-110">
                    <div class="absolute inset-0 bg-gradient-to-t from-black/80 via-transparent to-transparent"></div>
                    <div class="absolute bottom-4 left-4 right-4 flex items-center justify-between">
                      <div class="flex items-center gap-3">
                        <span class="px-2.5 py-1 bg-primary/80 backdrop-blur-md rounded-lg text-[10px] font-black text-white flex items-center gap-1.5 shadow-lg"><Users :size="12" /> {{ world.occupants || 0 }}</span>
                      </div>
                      <span class="text-[10px] font-black text-white/60 uppercase tracking-[0.2em] shadow-lg">{{ formatDate(world.updated_at) }}</span>
                    </div>
                  </div>
                  <div class="p-6">
                    <div class="font-black text-[16px] text-white mb-2 group-hover:text-primary transition-colors truncate tracking-tight">{{ world.name }}</div>
                    <div class="flex items-center gap-4 text-[11px] font-bold text-white/40">
                      <span class="flex items-center gap-1.5"><Heart :size="12" class="text-pink-500" /> {{ world.favorites || 0 }}</span>
                      <span class="flex items-center gap-1.5"><Eye :size="12" class="text-sky-500" /> {{ world.visits || 0 }}</span>
                    </div>
                  </div>
                </div>
              </div>
            </template>

            <!-- Avatars Tab -->
            <template v-else-if="activeTab === 'created_avatars'">
              <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
                <div v-if="profileStore.isLoadingAvatars" class="col-span-full py-20 flex justify-center"><RefreshCcw :size="40" class="animate-spin text-primary/40" /></div>
                <div v-else-if="profileStore.createdAvatars.length === 0" class="col-span-full py-20 text-center text-white/40 font-bold">{{ t('user_profile.no_data') }}</div>
                <div v-for="avatar in profileStore.createdAvatars" :key="avatar.id" class="group bg-white/5 border border-white/10 rounded-3xl p-4 hover:bg-white/10 hover:border-primary/40 transition-all shadow-xl">
                  <div class="relative aspect-square rounded-2xl overflow-hidden mb-4">
                    <img :src="avatar.imageUrl || avatar.thumbnailImageUrl" class="w-full h-full object-cover transition-transform group-hover:scale-110">
                    <div class="absolute inset-0 bg-gradient-to-t from-black/60 to-transparent opacity-0 group-hover:opacity-100 transition-opacity flex items-end p-4">
                      <span class="text-[10px] font-black text-white/90 uppercase tracking-widest">{{ avatar.releaseStatus }}</span>
                    </div>
                  </div>
                  <div class="px-2">
                    <div class="font-black text-[14px] text-white truncate group-hover:text-primary transition-colors mb-1">{{ avatar.name }}</div>
                    <div class="text-[10px] font-bold text-white/30 uppercase tracking-widest">{{ formatDate(avatar.updated_at) }}</div>
                  </div>
                </div>
              </div>
            </template>
            <template v-else-if="activeTab === 'activity'">
              <div class="space-y-4 max-w-3xl mx-auto">
                <div v-if="profileStore.isLoadingActivity" class="py-20 flex justify-center"><RefreshCcw :size="40" class="animate-spin text-primary/40" /></div>
                <div v-else-if="profileStore.activityLogs.length === 0" class="py-20 text-center text-white/40 font-bold">{{ t('user_profile.no_data') }}</div>
                <div v-for="log in profileStore.activityLogs" :key="log.id" class="flex items-start gap-5 p-5 bg-white/5 border border-white/5 rounded-2xl hover:bg-white/10 transition-all group">
                   <div class="w-10 h-10 rounded-full bg-primary/10 flex items-center justify-center shrink-0 border border-primary/20 group-hover:scale-110 transition-transform">
                      <History :size="18" class="text-primary" />
                   </div>
                   <div class="flex-1 min-w-0">
                      <div class="flex items-center justify-between mb-1">
                        <span class="text-[14px] font-bold text-white/90">{{ log.type || 'Activity' }}</span>
                        <span class="text-[11px] font-medium text-white/30">{{ formatTime(log.created_at) }}</span>
                      </div>
                      <div class="text-[13px] text-white/60 leading-relaxed">{{ log.message || 'Details...' }}</div>
                   </div>
                </div>
              </div>
            </template>

            <!-- Raw JSON Tab -->
            <template v-else-if="activeTab === 'raw_json'">
              <div class="bg-black/40 rounded-3xl p-8 border border-white/10 relative shadow-2xl">
                <div class="absolute top-4 right-4">
                  <button @click="executeAction('copy_id')" class="p-3 bg-white/5 hover:bg-white/10 text-white/60 rounded-xl transition-all border border-white/10 active:scale-95">
                    <Copy :size="18" />
                  </button>
                </div>
                <pre class="font-mono text-[13px] leading-relaxed overflow-x-auto custom-scrollbar whitespace-pre-wrap select-text" v-html="highlightJson(profileStore.baseInfo)"></pre>
              </div>
            </template>

          </div>

          <!-- Close Button Top Right -->
          <button 
            @click="profileStore.closeProfile"
            class="absolute top-6 right-6 w-12 h-12 bg-white/5 hover:bg-red-500/20 text-white/40 hover:text-red-500 rounded-full flex items-center justify-center transition-all border border-white/10 z-[110] active:scale-95"
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
          <button @click="handleResetImage" class="p-4 bg-white/5 hover:bg-white/10 rounded-2xl text-white/70 transition-all border border-white/10"><RotateCcw :size="20" /></button>
          <button @click="toggleImagePreview" class="p-4 bg-white/5 hover:bg-red-500/20 rounded-2xl text-white/70 hover:text-red-500 transition-all border border-white/10"><X :size="24" /></button>
        </div>
        
        <div class="flex-1 flex items-center justify-center overflow-hidden w-full h-full">
           <img 
            :src="profileStore.baseInfo?.profilePicOverride || profileStore.baseInfo?.currentAvatarImageUrl || profileStore.baseInfo?.currentAvatarThumbnailImageUrl" 
            class="max-w-[85vw] max-h-[75vh] object-contain shadow-[0_0_100px_rgba(0,0,0,0.8)] transition-all duration-300"
            :style="{ transform: `scale(${imageScale}) rotate(${imageRotation}deg)` }"
          />
        </div>

        <div class="mt-10 flex gap-4 p-5 bg-white/5 rounded-3xl border border-white/10 backdrop-blur-xl">
          <button @click="handleZoomOut" class="p-4 hover:bg-white/10 rounded-2xl text-white/70 transition-all"><ZoomOut :size="24" /></button>
          <div class="w-[1px] bg-white/10"></div>
          <button @click="handleZoomIn" class="p-4 hover:bg-white/10 rounded-2xl text-white/70 transition-all"><ZoomIn :size="24" /></button>
          <div class="w-[1px] bg-white/10"></div>
          <button @click="handleRotateCw" class="p-4 hover:bg-white/10 rounded-2xl text-white/70 transition-all"><RotateCw :size="24" /></button>
          <div class="w-[1px] bg-white/10"></div>
          <button @click="handleCopyImage" class="p-4 hover:bg-white/10 rounded-2xl text-white/70 transition-all"><Copy :size="24" /></button>
          <button @click="handleDownloadImage" class="p-4 hover:bg-emerald-500/20 rounded-2xl text-emerald-400 transition-all"><Download :size="24" /></button>
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

.custom-scrollbar::-webkit-scrollbar {
  width: 5px;
  height: 5px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.05);
  border-radius: 10px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.1);
}

.no-scrollbar::-webkit-scrollbar {
  display: none;
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
