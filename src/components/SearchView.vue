<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { VrcApi, DbApi, SysApi } from "../api";
import { Search, Users, Globe, Loader2, ExternalLink, UserPlus, Image as ImageIcon, UsersRound, Eye, Shield, Check } from 'lucide-vue-next';
import VrcAvatar from './VrcAvatar.vue';
import BaseModal from './BaseModal.vue';
import VrcResourceCard from './VrcResourceCard.vue';
import type { VrcUser, VrcWorld, VrcAvatar as VrcAvatarType } from '../types/vrc';
import { useI18n } from 'vue-i18n';
import { useUserProfileStore } from '../stores/userProfile';

const { t } = useI18n();
const profileStore = useUserProfileStore();

const searchQuery = ref('');
const searchType = ref<'users' | 'worlds' | 'avatars' | 'groups'>('worlds');
const results = ref<Array<any>>([]);
const loading = ref(false);
const errorMsg = ref('');
const hasSearched = ref(false);

// 用户详情弹窗交由 UserProfileStore 管理
import { useEntityModalStore } from '../stores/entityModal';
const entityStore = useEntityModalStore();

const doSearch = async () => {
  const q = searchQuery.value.trim();
  if (!q) return;
  loading.value = true;
  errorMsg.value = '';
  hasSearched.value = true;
  results.value = [];
  try {
    let res: any;
    if (searchType.value === 'users') {
      res = await VrcApi.searchUsers({ query: q, n: 30, offset: 0 });
    } else if (searchType.value === 'worlds') {
      res = await VrcApi.searchWorlds({ query: q, n: 30, offset: 0 });
    } else if (searchType.value === 'avatars') {
      res = await VrcApi.getAvatars({ search: q, n: 30, offset: 0 });
    } else if (searchType.value === 'groups') {
      res = await VrcApi.searchGroups({ query: q, n: 30, offset: 0 });
    }
    results.value = Array.isArray(res) ? res : [];
  } catch (err: any) {
    errorMsg.value = err.message || err;
  } finally {
    loading.value = false;
  }
};

const openUserDetail = (user: any) => {
  profileStore.openProfile(user.id, user);
};

const openWorldDetail = async (worldId: string) => {
  await entityStore.openWorld(worldId);
};

const openAvatarDetail = async (avatar: any) => {
  await entityStore.openAvatar(avatar);
};

const openGroupDetail = async (group: any) => {
  await entityStore.openGroup(group);
};

const getStatusColor = (status: string) => {
  switch (status) {
    case 'active': return 'bg-green-500';
    case 'join me': return 'bg-blue-500';
    case 'ask me': return 'bg-orange-500';
    case 'busy': return 'bg-red-500';
    default: return 'bg-surface';
  }
};

const getStatusLabel = (status: string) => {
  switch (status) {
    case 'active': return t('status.active');
    case 'join me': return t('status.join_me');
    case 'ask me': return t('status.ask_me');
    case 'busy': return t('status.busy');
    default: return t('status.offline');
  }
};


const handleDirectOpen = (e: any) => {
  const { type, data } = e.detail;
  if (type === 'user') {
    profileStore.openProfile(data.id, data);
  } else if (type === 'world') {
    entityStore.openWorld(data.id || data.world_id);
  } else if (type === 'avatar') {
    entityStore.openAvatar(data);
  } else if (type === 'group') {
    entityStore.openGroup(data);
  }
};

const handleGlobalSearch = (e: any) => {
  const { type, query } = e.detail;
  searchQuery.value = query || '';
  if (type === 'user') searchType.value = 'users';
  else if (type === 'world') searchType.value = 'worlds';
  else if (type === 'avatar') searchType.value = 'avatars';
  else if (type === 'group') searchType.value = 'groups';
  
  if (searchQuery.value.trim()) {
    doSearch();
  }
};

onMounted(() => {
  window.addEventListener('vrc-open-detail', handleDirectOpen);
  window.addEventListener('vrc-global-search', handleGlobalSearch);
});

onUnmounted(() => {
  window.removeEventListener('vrc-open-detail', handleDirectOpen);
  window.removeEventListener('vrc-global-search', handleGlobalSearch);
});
</script>

<template>
  <div class="h-full flex flex-col bg-surface-hover p-2">
    <!-- 顶部导航 Tab -->
    <div class="flex items-center gap-2 border-border-soft mb-6 px-4 pt-2">
      <button
        :class="searchType === 'users' ? 'border-b-2 border-primary text-primary font-extrabold' : 'text-text-muted hover:text-text-muted font-bold hover:bg-surface rounded-t-lg border-b-2 border-transparent'"
        class="py-3 px-4 transition-all text-sm"
        @click="searchType = 'users'"
      >
        {{ t('search.type_users') }}
      </button>
      <button
        :class="searchType === 'worlds' ? 'border-b-2 border-primary text-primary font-extrabold' : 'text-text-muted hover:text-text-muted font-bold hover:bg-surface rounded-t-lg border-b-2 border-transparent'"
        class="py-3 px-4 transition-all text-sm"
        @click="searchType = 'worlds'"
      >
        {{ t('search.type_worlds') }}
      </button>
      <button
        :class="searchType === 'avatars' ? 'border-b-2 border-primary text-primary font-extrabold' : 'text-text-muted hover:text-text-muted font-bold hover:bg-surface rounded-t-lg border-b-2 border-transparent'"
        class="py-3 px-4 transition-all text-sm"
        @click="searchType = 'avatars'"
      >
        {{ t('search.type_avatars') }}
      </button>
      <button
        :class="searchType === 'groups' ? 'border-b-2 border-primary text-primary font-extrabold' : 'text-text-muted hover:text-text-muted font-bold hover:bg-surface rounded-t-lg border-b-2 border-transparent'"
        class="py-3 px-4 transition-all text-sm"
        @click="searchType = 'groups'"
      >
        {{ t('search.type_groups') }}
      </button>
    </div>

    <!-- {{ t('search.placeholder') || 'Search' }}栏 -->
    <div class="flex gap-3 mb-6 px-2">
      <div class="flex-1 relative">
        <div class="absolute inset-y-0 left-0 pl-4 flex items-center pointer-events-none">
          <Search class="h-5 w-5 text-border-strong" />
        </div>
        <input
          v-model="searchQuery"
          type="text"
          class="block w-full pl-12 pr-4 py-3.5 bg-surface border-border-soft shadow-sm rounded-xl text-text placeholder-slate-400 focus:outline-none  focus:ring-4 focus:ring-indigo-500/10 text-sm font-bold transition-all"
          :placeholder="t('search.placeholder')"
          @keyup.enter="doSearch"
        >
      </div>
      <button
        :disabled="loading || !searchQuery.trim()"
        class="px-8 py-3 bg-surface hover:bg-background/80 backdrop-blur-md text-white font-bold rounded-xl shadow-sm transition-colors disabled:opacity-50 text-sm flex items-center gap-2"
        @click="doSearch"
      >
        <Loader2
          v-if="loading"
          class="animate-spin"
          :size="16"
        />
        <Search
          v-else
          class="w-4 h-4"
        />
        {{ t('search.placeholder') || 'Search' }}
      </button>
    </div>

    <!-- 错误 -->
    <div
      v-if="errorMsg"
      class="bg-red-50 text-red-600 p-4 rounded-xl border-red-200 text-sm font-bold mb-4 mx-2"
    >
      {{ errorMsg }}
    </div>

    <!-- 结果列表 -->
    <div class="flex-1 overflow-y-auto px-4 custom-scrollbar relative">
      <div
        v-if="loading"
        class="absolute inset-0 flex flex-col items-center justify-center text-primary bg-surface-hover backdrop-blur-sm z-10"
      >
        <Loader2
          class="animate-spin mb-4"
          :size="48"
        /> 
        <span class="font-extrabold text-lg tracking-wide">{{ t('search.searching') }}</span>
      </div>

      <div
        v-else-if="hasSearched && results.length === 0"
        class="h-full flex flex-col items-center justify-center text-border-strong"
      >
        <Search
          class="mb-4 opacity-30"
          :size="64"
        />
        <p class="font-bold text-xl text-text-muted">
          {{ t('search.no_data') }}
        </p>
        <p class="text-sm mt-2 font-medium">
          {{ t('search.no_match') }}
        </p>
      </div>

      <div
        v-else-if="!hasSearched"
        class="h-full flex flex-col items-center justify-center text-text-muted"
      >
        <Search
          class="mb-4 opacity-20"
          :size="80"
        />
        <p class="font-bold text-xl text-border-strong tracking-wide">
          {{ t('search.start_explore') }}
        </p>
      </div>

      <div
        v-else-if="searchType === 'users'"
        class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 pb-10"
      >
        <VrcResourceCard
          v-for="user in results"
          :key="user.id"
          type="user"
          :data="user"
          :is-user="true"
          @click="openUserDetail(user)"
        />
      </div>

      <!-- 世界{{ t('search.placeholder') || 'Search' }}结果 -->
      <div
        v-else-if="searchType === 'worlds'"
        class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-5 pb-10"
      >
        <VrcResourceCard
          v-for="world in results"
          :key="world.id"
          type="world"
          :data="world"
          @click="openWorldDetail(world.id)"
        />
      </div>

      <!-- 模型{{ t('search.placeholder') || 'Search' }}结果 -->
      <div
        v-else-if="searchType === 'avatars'"
        class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6 gap-5 pb-10"
      >
        <VrcResourceCard
          v-for="avatar in results"
          :key="avatar.id"
          type="avatar"
          :data="avatar"
          @click="openAvatarDetail(avatar)"
        />
      </div>

      <!-- {{ t('search.type_groups') }}{{ t('search.placeholder') || 'Search' }}结果 -->
      <div
        v-else-if="searchType === 'groups'"
        class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-5 pb-10"
      >
        <VrcResourceCard
          v-for="group in results"
          :key="group.id"
          type="group"
          :data="group"
          @click="openGroupDetail(group)"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.fade-enter-active, .fade-leave-active { transition: opacity 0.3s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }




</style>
