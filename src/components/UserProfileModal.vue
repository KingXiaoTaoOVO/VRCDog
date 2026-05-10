<script setup lang="ts">
import { useUserProfileStore } from '../stores/userProfile';
import { X, Loader2, Users, Globe, Activity, Heart, Shield, Hash, MapPin, Search, StickyNote, MoreVertical, MicOff, EyeOff, Ban, Mail, MessageSquare } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import VrcAvatarComp from './VrcAvatar.vue';
import VrcResourceCard from './VrcResourceCard.vue';
import { useEntityModalStore } from '../stores/entityModal';
import { computed, ref } from 'vue';

const { t } = useI18n();
const profileStore = useUserProfileStore();
const entityStore = useEntityModalStore();

const activeTab = ref<'info' | 'mutual' | 'groups' | 'worlds' | 'avatars'>('info');

const openWorldDetail = (world: any) => {
  entityStore.openWorld(world.id);
};

const openGroupDetail = (group: any) => {
  entityStore.openGroup(group);
};

const trustColor = computed(() => {
  if (!profileStore.baseInfo?.tags) return 'text-slate-400 bg-slate-100';
  const tags = profileStore.baseInfo.tags;
  if (tags.includes('system_trust_veteran')) return 'text-purple-600 bg-purple-100';
  if (tags.includes('system_trust_trusted')) return 'text-purple-400 bg-purple-50';
  if (tags.includes('system_trust_known')) return 'text-orange-500 bg-orange-100';
  if (tags.includes('system_trust_basic')) return 'text-blue-500 bg-blue-100';
  if (tags.includes('system_trust_new')) return 'text-slate-500 bg-slate-100';
  return 'text-slate-400 bg-slate-100';
});

const trustName = computed(() => {
  if (!profileStore.baseInfo?.tags) return 'Visitor';
  const tags = profileStore.baseInfo.tags;
  if (tags.includes('system_trust_veteran')) return 'Trusted User';
  if (tags.includes('system_trust_trusted')) return 'Known User';
  if (tags.includes('system_trust_known')) return 'User';
  if (tags.includes('system_trust_basic')) return 'New User';
  if (tags.includes('system_trust_new')) return 'Visitor';
  return 'Visitor';
});

const launchInstance = async (location: string) => {
  try {
    const { SysApi } = await import('../api');
    await SysApi.launchVrc({ launchArgs: `vrchat://launch?id=${location}` });
  } catch (err) {
    console.warn('Failed to launch VRC', err);
  }
};

const inviteMyselfToLocation = async (location: string) => {
  try {
    const { VrcApi } = await import('../api');
    const [worldId, instanceId] = location.split(':');
    if (worldId && instanceId) {
      await VrcApi.inviteMyself({ worldId, instanceId });
    }
  } catch (err) {
    console.warn('Failed to invite myself', err);
  }
};

const handleFriendAction = async (action: 'add' | 'remove') => {
  if (!profileStore.baseInfo?.id) return;
  const userId = profileStore.baseInfo.id;
  try {
    const { VrcApi } = await import('../api');
    if (action === 'add') {
      await VrcApi.friendRequest({ userId });
      alert(t('user_profile.friend_request_sent'));
    } else {
      if (confirm(t('user_profile.confirm_unfriend'))) {
        await VrcApi.unfriend({ userId });
        profileStore.baseInfo.isFriend = false;
        alert(t('user_profile.unfriend_success'));
      }
    }
  } catch (err: any) {
    alert(t('user_profile.action_failed') + (err.message || err));
  }
};

const handleModeration = async (type: 'block' | 'mute' | 'hideAvatar') => {
  if (!profileStore.baseInfo?.id) return;
  const userId = profileStore.baseInfo.id;
  try {
    const { VrcApi } = await import('../api');
    await VrcApi.moderateUser({ moderated: userId, type });
    alert(`${t('user_profile.action_success')} ${type}`);
  } catch (err: any) {
    alert(t('user_profile.action_failed') + (err.message || err));
  }
};

const handleInvite = async (type: 'invite' | 'request') => {
  if (!profileStore.baseInfo?.id) return;
  const userId = profileStore.baseInfo.id;
  try {
    const { VrcApi } = await import('../api');
    if (type === 'invite') {
      await VrcApi.inviteUser({ userId });
      alert(t('user_profile.invite_sent'));
    } else {
      await VrcApi.requestInvite({ userId });
      alert(t('user_profile.req_invite_sent'));
    }
  } catch (err: any) {
    alert(t('user_profile.send_failed') + (err.message || err));
  }
};

const userLanguages = computed(() => {
  if (!profileStore.baseInfo?.tags) return [];
  const langMap: Record<string, string> = {
    'language_eng': 'English',
    'language_kor': 'Korean',
    'language_rus': 'Russian',
    'language_spa': 'Spanish',
    'language_por': 'Portuguese',
    'language_zho': 'Chinese',
    'language_deu': 'German',
    'language_jpn': 'Japanese',
    'language_fra': 'French',
    'language_swe': 'Swedish',
    'language_nld': 'Dutch',
    'language_pol': 'Polish',
    'language_dan': 'Danish',
    'language_nor': 'Norwegian',
    'language_ita': 'Italian',
    'language_tha': 'Thai',
    'language_fin': 'Finnish',
    'language_hun': 'Hungarian',
    'language_ces': 'Czech',
    'language_tur': 'Turkish',
    'language_ara': 'Arabic',
    'language_ron': 'Romanian',
    'language_vie': 'Vietnamese',
    'language_ase': 'Sign Language'
  };
  return profileStore.baseInfo.tags
    .filter((t: string) => t.startsWith('language_'))
    .map((t: string) => langMap[t] || t.replace('language_', ''));
});
</script>

<template>
  <!-- 背景遮罩 -->
  <transition name="fade">
    <div
      v-if="profileStore.isOpen"
      class="fixed inset-0 bg-black/60 backdrop-blur-sm z-[100] flex items-center justify-center p-4"
      @click.self="profileStore.closeProfile"
    >
      <!-- 主弹窗 -->
      <transition name="slide-up">
        <div
          v-if="profileStore.isOpen"
          class="bg-white/90 backdrop-blur-xl rounded-2xl shadow-2xl w-full max-w-4xl max-h-[90vh] flex flex-col overflow-hidden border border-white/20"
        >
          <!-- 头部区域 -->
          <div class="relative h-48 bg-gradient-to-br from-indigo-900 to-purple-900 flex-shrink-0">
            <!-- 头像背景虚化 -->
            <img 
              v-if="profileStore.baseInfo?.profilePicOverride || profileStore.baseInfo?.currentAvatarImageUrl"
              :src="profileStore.baseInfo.profilePicOverride || profileStore.baseInfo.currentAvatarImageUrl"
              class="absolute inset-0 w-full h-full object-cover opacity-30 mix-blend-overlay"
            >
            
            <button
              class="absolute top-4 right-4 p-2 bg-black/20 hover:bg-black/40 text-white rounded-full transition-colors z-10"
              @click="profileStore.closeProfile"
            >
              <X :size="20" />
            </button>

            <!-- 核心信息区 -->
            <div class="absolute bottom-0 left-0 right-0 p-6 flex items-end gap-6 bg-gradient-to-t from-black/80 to-transparent">
              <div class="w-28 h-28 rounded-2xl overflow-hidden border-4 border-white/10 shadow-2xl bg-slate-900 flex-shrink-0">
                <VrcAvatarComp
                  v-if="profileStore.baseInfo"
                  :user="profileStore.baseInfo"
                  custom-class="w-full h-full object-cover"
                />
                <div
                  v-else
                  class="w-full h-full flex items-center justify-center text-white/50"
                >
                  <Loader2
                    class="animate-spin"
                    :size="32"
                  />
                </div>
              </div>
              
              <div class="flex-1 pb-2">
                <div class="flex items-center gap-3">
                  <h2 class="text-3xl font-extrabold text-white tracking-tight">
                    {{ profileStore.baseInfo?.displayName || 'Loading...' }}
                  </h2>
                  <!-- 状态圆点 -->
                  <div 
                    class="w-3.5 h-3.5 rounded-full border-2 border-black/50"
                    :class="{
                      'bg-green-500': profileStore.baseInfo?.status === 'active' || profileStore.baseInfo?.status === 'join me',
                      'bg-orange-500': profileStore.baseInfo?.status === 'ask me',
                      'bg-red-500': profileStore.baseInfo?.status === 'busy',
                      'bg-slate-500': profileStore.baseInfo?.status === 'offline'
                    }"
                  />
                </div>
                
                <div class="flex items-center gap-2 mt-2">
                  <span 
                    class="px-2.5 py-1 rounded-md text-xs font-bold font-mono"
                    :class="trustColor"
                  >
                    <Shield
                      :size="12"
                      class="inline mr-1"
                    />{{ trustName }}
                  </span>
                  
                  <span
                    v-if="profileStore.baseInfo?.isFriend"
                    class="px-2.5 py-1 bg-yellow-400/20 text-yellow-300 rounded-md text-xs font-bold"
                  >
                    <Heart
                      :size="12"
                      class="inline mr-1"
                    /> {{ t('user_profile.friend_badge') }}
                  </span>
                </div>
              </div>

              <!-- 操作区 (Action Buttons) -->
              <div
                v-if="profileStore.baseInfo && profileStore.myId && profileStore.baseInfo.id !== profileStore.myId"
                class="flex-shrink-0 pb-2 flex gap-2 items-end z-10"
              >
                <!-- 更多操作下拉菜单 -->
                <div class="relative group">
                  <button class="p-2.5 bg-black/40 hover:bg-black/60 text-white rounded-xl shadow-lg transition-all backdrop-blur-md">
                    <MoreVertical :size="20" />
                  </button>
                  <div class="absolute right-0 bottom-full pb-2 hidden group-hover:block z-50">
                    <div class="w-40 bg-slate-900 border border-slate-700/50 rounded-xl shadow-2xl py-1.5 transition-all opacity-0 group-hover:opacity-100 backdrop-blur-xl">
                      <button
                        class="w-full px-4 py-2 text-left text-[13px] text-slate-200 hover:bg-white/10 hover:text-white transition-colors flex items-center gap-2"
                        @click="handleInvite('request')"
                      >
                        <Mail :size="14" /> {{ t('user_profile.req_invite') }}
                      </button>
                      <button
                        class="w-full px-4 py-2 text-left text-[13px] text-slate-200 hover:bg-white/10 hover:text-white transition-colors flex items-center gap-2"
                        @click="handleInvite('invite')"
                      >
                        <MessageSquare :size="14" /> {{ t('user_profile.invite') }}
                      </button>
                      <div class="h-px bg-slate-700/50 my-1.5" />
                      <button
                        class="w-full px-4 py-2 text-left text-[13px] text-slate-200 hover:bg-white/10 hover:text-white transition-colors flex items-center gap-2"
                        @click="handleModeration('hideAvatar')"
                      >
                        <EyeOff :size="14" /> {{ t('user_profile.hide_avatar') }}
                      </button>
                      <button
                        class="w-full px-4 py-2 text-left text-[13px] text-slate-200 hover:bg-white/10 hover:text-white transition-colors flex items-center gap-2"
                        @click="handleModeration('mute')"
                      >
                        <MicOff :size="14" /> {{ t('user_profile.mute') }}
                      </button>
                      <div class="h-px bg-slate-700/50 my-1.5" />
                      <button
                        class="w-full px-4 py-2 text-left text-[13px] text-red-400 hover:bg-red-500/10 hover:text-red-300 transition-colors flex items-center gap-2"
                        @click="handleModeration('block')"
                      >
                        <Ban :size="14" /> {{ t('user_profile.block') }}
                      </button>
                    </div>
                  </div>
                </div>

                <button 
                  v-if="!profileStore.baseInfo.isFriend" 
                  class="px-5 py-2.5 bg-indigo-500 hover:bg-indigo-600 text-white rounded-xl text-sm font-extrabold shadow-lg transition-all" 
                  @click="handleFriendAction('add')"
                >
                  {{ t('user_profile.add_friend') }}
                </button>
                <button 
                  v-else 
                  class="px-5 py-2.5 bg-red-500/80 hover:bg-red-600 text-white border border-red-400/50 rounded-xl text-sm font-extrabold shadow-lg transition-all" 
                  @click="handleFriendAction('remove')"
                >
                  {{ t('user_profile.remove_friend') }}
                </button>
              </div>
            </div>
          </div>

          <!-- Tab 导航 -->
          <div class="flex items-center gap-6 px-6 border-b border-slate-100 bg-white/50 flex-shrink-0 overflow-x-auto hide-scrollbar">
            <button 
              class="py-4 text-sm font-bold border-b-2 transition-colors whitespace-nowrap"
              :class="activeTab === 'info' ? 'border-indigo-500 text-indigo-600' : 'border-transparent text-slate-500 hover:text-slate-700'"
              @click="activeTab = 'info'"
            >
              {{ t('user_profile.tab_info') }}
            </button>
            <button 
              class="py-4 text-sm font-bold border-b-2 transition-colors whitespace-nowrap flex items-center gap-1.5"
              :class="activeTab === 'mutual' ? 'border-indigo-500 text-indigo-600' : 'border-transparent text-slate-500 hover:text-slate-700'"
              @click="activeTab = 'mutual'"
            >
              {{ t('user_profile.tab_mutual') }}
            </button>
            <button 
              class="py-4 text-sm font-bold border-b-2 transition-colors whitespace-nowrap flex items-center gap-1.5"
              :class="activeTab === 'groups' ? 'border-indigo-500 text-indigo-600' : 'border-transparent text-slate-500 hover:text-slate-700'"
              @click="activeTab = 'groups'"
            >
              {{ t('user_profile.tab_groups') }} <span class="bg-slate-100 text-slate-500 px-1.5 py-0.5 rounded-full text-[10px]">{{ profileStore.groups.length }}</span>
            </button>
            <button 
              class="py-4 text-sm font-bold border-b-2 transition-colors whitespace-nowrap flex items-center gap-1.5"
              :class="activeTab === 'worlds' ? 'border-indigo-500 text-indigo-600' : 'border-transparent text-slate-500 hover:text-slate-700'"
              @click="activeTab = 'worlds'"
            >
              {{ t('user_profile.tab_worlds') }}
            </button>
            <button 
              class="py-4 text-sm font-bold border-b-2 transition-colors whitespace-nowrap flex items-center gap-1.5"
              :class="activeTab === 'avatars' ? 'border-indigo-500 text-indigo-600' : 'border-transparent text-slate-500 hover:text-slate-700'"
              @click="activeTab = 'avatars'"
            >
              {{ t('user_profile.tab_avatars') }}
            </button>
          </div>

          <!-- 内容区 -->
          <div class="flex-1 overflow-y-auto p-6 bg-slate-50/50">
            <!-- 基础信息 Tab -->
            <div
              v-if="activeTab === 'info'"
              class="space-y-6"
            >
              <!-- 本地备注 (Local Note) -->
              <div class="bg-white rounded-xl p-4 border border-slate-100 shadow-sm relative">
                <div class="flex items-center justify-between mb-2">
                  <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider flex items-center gap-1.5">
                    <StickyNote :size="14" /> {{ t('user_profile.local_note') }}
                  </h3>
                  <Loader2
                    v-if="profileStore.isSavingNote"
                    class="animate-spin text-indigo-400"
                    :size="14"
                  />
                </div>
                <textarea 
                  v-model="profileStore.localNote"
                  class="w-full bg-slate-50 border-0 rounded-lg p-3 text-sm text-slate-700 focus:ring-2 focus:ring-indigo-500/50 transition-shadow resize-none h-20"
                  :placeholder="t('user_profile.local_note_placeholder')"
                  @blur="profileStore.saveLocalNote()"
                />
              </div>

              <!-- Bio -->
              <div
                v-if="profileStore.baseInfo?.bio"
                class="bg-white rounded-xl p-4 border border-slate-100 shadow-sm"
              >
                <h3 class="text-xs font-bold text-slate-400 mb-2 uppercase tracking-wider">
                  {{ t('user_profile.bio') }}
                </h3>
                <p class="text-sm text-slate-700 whitespace-pre-wrap leading-relaxed">
                  {{ profileStore.baseInfo.bio }}
                </p>
              </div>
              
              <!-- Other details -->
              <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
                <div class="bg-white rounded-xl p-4 border border-slate-100 shadow-sm">
                  <h3 class="text-xs font-bold text-slate-400 mb-1 uppercase tracking-wider">
                    {{ t('user_profile.status') }}
                  </h3>
                  <p class="text-sm text-slate-700 font-medium">
                    {{ profileStore.baseInfo?.statusDescription || t('user_profile.none') }}
                  </p>
                </div>
                <div class="bg-white rounded-xl p-4 border border-slate-100 shadow-sm">
                  <h3 class="text-xs font-bold text-slate-400 mb-1 uppercase tracking-wider">
                    {{ t('user_profile.last_login') }}
                  </h3>
                  <p class="text-sm text-slate-700 font-medium">
                    {{ profileStore.baseInfo?.last_login || profileStore.baseInfo?.last_activity || t('user_profile.unknown') }}
                  </p>
                </div>
                <div class="bg-white rounded-xl p-4 border border-slate-100 shadow-sm">
                  <h3 class="text-xs font-bold text-slate-400 mb-1 uppercase tracking-wider">
                    {{ t('user_profile.date_joined') }}
                  </h3>
                  <p class="text-sm text-slate-700 font-medium">
                    {{ profileStore.baseInfo?.date_joined || t('user_profile.unknown') }}
                  </p>
                </div>
                <div class="bg-white rounded-xl p-4 border border-slate-100 shadow-sm">
                  <h3 class="text-xs font-bold text-slate-400 mb-1 uppercase tracking-wider">
                    {{ t('user_profile.dev_type') }}
                  </h3>
                  <p class="text-sm text-slate-700 font-medium uppercase">
                    {{ profileStore.baseInfo?.developerType === 'none' ? t('user_profile.normal_player') : (profileStore.baseInfo?.developerType || t('user_profile.unknown')) }}
                  </p>
                </div>
              </div>

              <!-- 个人资料拓展: 语言与代词 -->
              <div
                v-if="userLanguages.length > 0 || profileStore.baseInfo?.pronouns"
                class="grid grid-cols-1 md:grid-cols-2 gap-4"
              >
                <div
                  v-if="userLanguages.length > 0"
                  class="bg-white rounded-xl p-4 border border-slate-100 shadow-sm flex items-start gap-4"
                >
                  <div class="p-2 bg-indigo-50 text-indigo-500 rounded-lg">
                    <Globe :size="18" />
                  </div>
                  <div>
                    <h3 class="text-xs font-bold text-slate-400 mb-1 uppercase tracking-wider">
                      {{ t('user_profile.languages') }}
                    </h3>
                    <div class="flex flex-wrap gap-1.5 mt-1">
                      <span
                        v-for="lang in userLanguages"
                        :key="lang"
                        class="bg-slate-100 text-slate-600 px-2 py-0.5 rounded text-xs font-bold"
                      >
                        {{ lang }}
                      </span>
                    </div>
                  </div>
                </div>
                <div
                  v-if="profileStore.baseInfo?.pronouns"
                  class="bg-white rounded-xl p-4 border border-slate-100 shadow-sm flex items-start gap-4"
                >
                  <div class="p-2 bg-purple-50 text-purple-500 rounded-lg">
                    <Users :size="18" />
                  </div>
                  <div>
                    <h3 class="text-xs font-bold text-slate-400 mb-1 uppercase tracking-wider">
                      {{ t('user_profile.pronouns') }}
                    </h3>
                    <p class="text-sm text-slate-700 font-medium">
                      {{ profileStore.baseInfo.pronouns }}
                    </p>
                  </div>
                </div>
              </div>

              <!-- Bio Links -->
              <div
                v-if="profileStore.baseInfo?.bioLinks && profileStore.baseInfo.bioLinks.length > 0"
                class="bg-white rounded-xl p-4 border border-slate-100 shadow-sm"
              >
                <h3 class="text-xs font-bold text-slate-400 mb-2 uppercase tracking-wider">
                  {{ t('user_profile.social_links') }}
                </h3>
                <div class="flex flex-wrap gap-2 mt-1">
                  <a 
                    v-for="link in profileStore.baseInfo.bioLinks" 
                    :key="link" 
                    :href="link" 
                    target="_blank" 
                    class="flex items-center gap-2 bg-slate-50 border border-slate-200 hover:border-indigo-300 text-slate-700 hover:text-indigo-600 px-3 py-1.5 rounded-lg text-sm transition-colors font-medium break-all"
                  >
                    {{ link }}
                  </a>
                </div>
              </div>

              <!-- 当前位置 (Current Location) -->
              <div
                v-if="profileStore.baseInfo?.location && profileStore.baseInfo.location !== 'offline'"
                class="bg-indigo-50/50 rounded-xl p-4 border border-indigo-100 shadow-sm"
              >
                <h3 class="text-xs font-bold text-indigo-400 mb-2 uppercase tracking-wider flex items-center gap-1.5">
                  <MapPin :size="14" /> {{ t('user_profile.current_location') }}
                </h3>
                <div class="flex items-start justify-between gap-4">
                  <p class="text-sm text-indigo-900 font-bold break-all flex-1 min-w-0 leading-relaxed">
                    {{ profileStore.baseInfo.location === 'private' ? t('user_profile.private_room') : profileStore.baseInfo.location }}
                  </p>
                  
                  <div
                    v-if="profileStore.baseInfo.location !== 'private'"
                    class="flex gap-2 flex-shrink-0"
                  >
                    <button 
                      class="px-3 py-1.5 bg-white border border-indigo-200 hover:border-indigo-400 text-indigo-700 rounded-lg text-xs font-bold shadow-sm transition-all whitespace-nowrap" 
                      @click="launchInstance(profileStore.baseInfo.location)"
                    >
                      {{ t('user_profile.join') }}
                    </button>
                    <button 
                      class="px-3 py-1.5 bg-indigo-500 hover:bg-indigo-600 text-white rounded-lg text-xs font-bold shadow-sm transition-all whitespace-nowrap" 
                      @click="inviteMyselfToLocation(profileStore.baseInfo.location)"
                    >
                      {{ t('user_profile.drop_portal') }}
                    </button>
                  </div>
                </div>
              </div>
            </div>

            <!-- 群组 Tab -->
            <div v-if="activeTab === 'groups'">
              <div
                v-if="profileStore.isLoadingGroups"
                class="flex justify-center py-12 text-indigo-500"
              >
                <Loader2
                  class="animate-spin"
                  :size="32"
                />
              </div>
              <div
                v-else-if="profileStore.groups.length === 0"
                class="text-center py-12 text-slate-400"
              >
                <Hash
                  class="mx-auto mb-2 opacity-50"
                  :size="48"
                />
                <p>{{ t('user_profile.no_groups') }}</p>
              </div>
              <div
                v-else
                class="grid grid-cols-1 md:grid-cols-2 gap-4"
              >
                <div 
                  v-for="group in profileStore.groups" 
                  :key="group.id"
                  class="bg-white rounded-xl p-3 border border-slate-100 shadow-sm flex items-center gap-4 hover:border-indigo-200 transition-colors cursor-pointer"
                  @click="openGroupDetail(group)"
                >
                  <img
                    :src="group.iconUrl || group.icon_url"
                    class="w-12 h-12 rounded-lg bg-slate-100 object-cover"
                  >
                  <div class="flex-1 min-w-0">
                    <h4 class="font-bold text-slate-900 text-sm truncate">
                      {{ group.name }}
                    </h4>
                    <p class="text-xs text-slate-500">
                      {{ group.shortCode }} • {{ group.memberCount }} members
                    </p>
                  </div>
                </div>
              </div>
            </div>

            <!-- 创建的世界 Tab -->
            <div v-if="activeTab === 'worlds'">
              <div
                v-if="profileStore.isLoadingWorlds"
                class="flex justify-center py-12 text-indigo-500"
              >
                <Loader2
                  class="animate-spin"
                  :size="32"
                />
              </div>
              <div
                v-else-if="profileStore.createdWorlds.length === 0"
                class="text-center py-12 text-slate-400"
              >
                <Globe
                  class="mx-auto mb-2 opacity-50"
                  :size="48"
                />
                <p>{{ t('user_profile.no_worlds') }}</p>
              </div>
              <div
                v-else
                class="grid grid-cols-2 lg:grid-cols-3 gap-4"
              >
                <VrcResourceCard 
                  v-for="world in profileStore.createdWorlds" 
                  :key="world.id"
                  type="world"
                  :data="world"
                  @click="openWorldDetail"
                />
              </div>
            </div>
              
            <!-- 创建的模型 Tab -->
            <div v-if="activeTab === 'avatars'">
              <div
                v-if="profileStore.isLoadingAvatars"
                class="flex justify-center py-12 text-indigo-500"
              >
                <Loader2
                  class="animate-spin"
                  :size="32"
                />
              </div>
              <div
                v-else-if="profileStore.createdAvatars.length === 0"
                class="text-center py-12 text-slate-400"
              >
                <Users
                  class="mx-auto mb-2 opacity-50"
                  :size="48"
                />
                <p>{{ t('user_profile.no_avatars') }}</p>
              </div>
              <div
                v-else
                class="grid grid-cols-3 lg:grid-cols-4 gap-4"
              >
                <VrcResourceCard 
                  v-for="avatar in profileStore.createdAvatars" 
                  :key="avatar.id"
                  type="avatar"
                  :data="avatar"
                  @click="entityStore.openAvatar(avatar)"
                />
              </div>
            </div>
              
            <!-- 共同好友 Tab -->
            <div v-if="activeTab === 'mutual'">
              <div
                v-if="profileStore.isLoadingMutual"
                class="flex justify-center items-center py-12 text-indigo-500"
              >
                <Loader2
                  class="animate-spin"
                  :size="32"
                />
              </div>
              <div
                v-else-if="profileStore.mutualFriends.length === 0"
                class="text-center py-12 text-slate-400"
              >
                <Users
                  class="mx-auto mb-2 opacity-50"
                  :size="48"
                />
                <p>{{ t('user_profile.no_mutual') }}</p>
              </div>
              <div
                v-else
                class="grid grid-cols-3 lg:grid-cols-4 gap-4"
              >
                <VrcResourceCard 
                  v-for="friend in profileStore.mutualFriends" 
                  :key="friend.id || friend"
                  type="avatar"
                  :data="typeof friend === 'string' ? { id: friend, displayName: 'Mutual Friend: ' + friend } : friend"
                  :is-user="true"
                  @click="profileStore.openProfile(typeof friend === 'string' ? friend : friend.id)"
                />
              </div>
            </div>
          </div>
        </div>
      </transition>
    </div>
  </transition>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.slide-up-enter-active,
.slide-up-leave-active {
  transition: all 0.4s cubic-bezier(0.16, 1, 0.3, 1);
}
.slide-up-enter-from {
  opacity: 0;
  transform: translateY(20px) scale(0.95);
}
.slide-up-leave-to {
  opacity: 0;
  transform: translateY(20px) scale(0.95);
}

.hide-scrollbar::-webkit-scrollbar {
  display: none;
}
.hide-scrollbar {
  -ms-overflow-style: none;
  scrollbar-width: none;
}
</style>
