<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { VrcApi, DbApi } from "../api";
import { Search, Users, Globe, Loader2, ExternalLink, UserPlus, Image as ImageIcon, UsersRound, Eye } from 'lucide-vue-next';
import VrcAvatar from './VrcAvatar.vue';
import BaseModal from './BaseModal.vue';
import type { VrcUser, VrcWorld, VrcAvatar as VrcAvatarType } from '../types/vrc';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const searchQuery = ref('');
const searchType = ref<'users' | 'worlds' | 'avatars' | 'groups'>('worlds');
const results = ref<Array<any>>([]);
const loading = ref(false);
const errorMsg = ref('');
const hasSearched = ref(false);

// 用户详情弹窗
const selectedUser = ref<any>(null);
const selectedWorld = ref<any>(null);
const selectedAvatar = ref<any>(null);
const selectedGroup = ref<any>(null);
const userNote = ref('');
const isWorldFavorited = ref(false);
const isAvatarFavorited = ref(false);
const isGroupFavorited = ref(false);
const loadingUser = ref(false);
const loadingWorld = ref(false);
const loadingAvatar = ref(false);
const loadingGroup = ref(false);

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

const openUserDetail = async (userId: string) => {
  loadingUser.value = true;
  selectedUser.value = null;
  userNote.value = '';
  try {
    const user = await VrcApi.getUser({ userId });
    selectedUser.value = user as VrcUser;
    const dbNote = await DbApi.getNote({ userId });
    if (dbNote) {
      userNote.value = dbNote.note;
    }
  } catch (err: any) {
    errorMsg.value = err.message || err;
  } finally {
    loadingUser.value = false;
  }
};

const saveUserNote = async () => {
  if (selectedUser.value) {
    try {
      await DbApi.saveNote({ 
        userId: selectedUser.value.id, 
        displayName: selectedUser.value.displayName, 
        note: userNote.value 
      });
    } catch (e) {
      console.error("Failed to save note", e);
    }
  }
};

const openWorldDetail = async (worldId: string) => {
  loadingWorld.value = true;
  selectedWorld.value = null;
  isWorldFavorited.value = false;
  try {
    const world = await VrcApi.getWorld({ worldId: worldId });
    selectedWorld.value = world as VrcWorld;
    
    // Check if it's in local favorites
    const favs = await DbApi.getFavoriteWorlds();
    isWorldFavorited.value = favs.some((f: any) => f.world_id === worldId);
  } catch (err: any) {
    errorMsg.value = err.message || err;
  } finally {
    loadingWorld.value = false;
  }
};

const toggleFavoriteWorld = async () => {
  if (!selectedWorld.value) return;
  try {
    if (isWorldFavorited.value) {
      await DbApi.removeFavoriteWorld({ worldId: selectedWorld.value.id });
      isWorldFavorited.value = false;
    } else {
      await DbApi.addFavoriteWorld({
        worldId: selectedWorld.value.id,
        name: selectedWorld.value.name,
        imageUrl: selectedWorld.value.imageUrl || selectedWorld.value.thumbnailImageUrl || null
      });
      isWorldFavorited.value = true;
    }
  } catch (e) {
    console.error("Favorite toggle failed:", e);
  }
};

const openAvatarDetail = async (avatar: any) => {
  selectedAvatar.value = avatar;
  isAvatarFavorited.value = false;
  try {
    const favs: any = await DbApi.getFavoriteAvatars();
    isAvatarFavorited.value = favs.some((f: any) => f.avatar_id === avatar.id);
  } catch (e) {}
};

const openGroupDetail = async (group: any) => {
  selectedGroup.value = group;
};

const toggleFavoriteAvatar = async () => {
  if (!selectedAvatar.value) return;
  try {
    if (isAvatarFavorited.value) {
      await DbApi.removeFavoriteAvatar({ avatarId: selectedAvatar.value.id });
      isAvatarFavorited.value = false;
    } else {
      await DbApi.addFavoriteAvatar({
        avatarId: selectedAvatar.value.id,
        name: selectedAvatar.value.name,
        imageUrl: selectedAvatar.value.imageUrl || selectedAvatar.value.thumbnailImageUrl || null,
        authorId: selectedAvatar.value.authorId,
        authorName: selectedAvatar.value.authorName
      });
      isAvatarFavorited.value = true;
    }
  } catch (e) {
    console.error("Favorite toggle failed:", e);
  }
};

const getStatusColor = (status: string) => {
  switch (status) {
    case 'active': return 'bg-green-500';
    case 'join me': return 'bg-blue-500';
    case 'ask me': return 'bg-orange-500';
    case 'busy': return 'bg-red-500';
    default: return 'bg-gray-400';
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

const isSendingFriendRequest = ref(false);
const friendRequestSuccess = ref(false);
const friendRequestError = ref('');

const sendFriendRequest = async () => {
  if (!selectedUser.value) return;
  isSendingFriendRequest.value = true;
  friendRequestSuccess.value = false;
  friendRequestError.value = '';
  try {
    await VrcApi.friendRequest({ userId: selectedUser.value.id });
    friendRequestSuccess.value = true;
    selectedUser.value.isFriend = true; // 简单设置状态避免重复发送
    setTimeout(() => { friendRequestSuccess.value = false; }, 3000);
  } catch (err: any) {
    friendRequestError.value = `发送失败: ${err.message || err}`;
    setTimeout(() => { friendRequestError.value = ''; }, 3000);
  } finally {
    isSendingFriendRequest.value = false;
  }
};
const handleDirectOpen = (e: any) => {
  const { type, data } = e.detail;
  if (type === 'user') {
    selectedUser.value = data;
    // Optionally fetch note
    DbApi.getNote({ userId: data.id }).then(note => {
      if (note) userNote.value = note.note;
    });
  } else if (type === 'world') {
    selectedWorld.value = data;
    DbApi.getFavoriteWorlds().then((favs: any) => {
      isWorldFavorited.value = favs.some((f: any) => f.world_id === data.id);
    });
  } else if (type === 'avatar') {
    selectedAvatar.value = data;
    DbApi.getFavoriteAvatars().then((favs: any) => {
      isAvatarFavorited.value = favs.some((f: any) => f.avatar_id === data.id);
    });
  } else if (type === 'group') {
    selectedGroup.value = data;
  }
};

onMounted(() => {
  window.addEventListener('vrc-open-detail', handleDirectOpen);
});

onUnmounted(() => {
  window.removeEventListener('vrc-open-detail', handleDirectOpen);
});

</script>

<template>
  <div class="h-full flex flex-col bg-gray-50/50 p-2">
    <!-- 顶部导航 Tab -->
    <div class="flex items-center gap-6 border-b border-gray-200 mb-4 px-2">
      <button
        :class="searchType === 'users' ? 'border-b-2 border-amber-600 text-amber-800 font-bold' : 'text-gray-500 hover:text-gray-700 font-medium'"
        class="py-3 px-1 transition-colors text-sm"
        @click="searchType = 'users'"
      >
        {{ t('search.type_users') || '玩家' }}
      </button>
      <button
        :class="searchType === 'worlds' ? 'border-b-2 border-amber-600 text-amber-800 font-bold' : 'text-gray-500 hover:text-gray-700 font-medium'"
        class="py-3 px-1 transition-colors text-sm"
        @click="searchType = 'worlds'"
      >
        {{ t('search.type_worlds') || '世界' }}
      </button>
      <button
        :class="searchType === 'avatars' ? 'border-b-2 border-amber-600 text-amber-800 font-bold' : 'text-gray-500 hover:text-gray-700 font-medium'"
        class="py-3 px-1 transition-colors text-sm"
        @click="searchType = 'avatars'"
      >
        {{ t('search.type_avatars') || '模型' }}
      </button>
      <button
        :class="searchType === 'groups' ? 'border-b-2 border-amber-600 text-amber-800 font-bold' : 'text-gray-500 hover:text-gray-700 font-medium'"
        class="py-3 px-1 transition-colors text-sm"
        @click="searchType = 'groups'"
      >
        群组
      </button>
    </div>

    <!-- 搜索栏 -->
    <div class="flex gap-3 mb-6 px-2">
      <div class="flex-1 relative">
        <div class="absolute inset-y-0 left-0 pl-4 flex items-center pointer-events-none">
          <Search class="h-5 w-5 text-gray-400" />
        </div>
        <input
          v-model="searchQuery"
          type="text"
          class="block w-full pl-12 pr-4 py-3 bg-white border border-gray-200 shadow-sm rounded-xl text-gray-900 placeholder-gray-400 focus:outline-none focus:border-amber-500 focus:ring-2 focus:ring-amber-500/20 text-sm font-medium transition-all"
          :placeholder="t('search.placeholder')"
          @keyup.enter="doSearch"
        >
      </div>
      <button
        :disabled="loading || !searchQuery.trim()"
        class="px-8 py-3 bg-gray-900 hover:bg-black text-white font-bold rounded-xl shadow-sm transition-colors disabled:opacity-50 text-sm flex items-center gap-2"
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
        搜索
      </button>
    </div>

    <!-- 错误 -->
    <div
      v-if="errorMsg"
      class="bg-red-50 text-red-600 p-4 rounded-xl border border-red-200 text-sm font-bold mb-4 mx-2"
    >
      {{ errorMsg }}
    </div>

    <!-- 结果列表 -->
    <div class="flex-1 overflow-y-auto px-2 custom-scrollbar">
      <div
        v-if="loading"
        class="flex flex-col items-center justify-center py-20 text-amber-600/70"
      >
        <Loader2
          class="animate-spin mb-4"
          :size="36"
        /> 
        <span class="font-bold text-lg">{{ t('search.searching') }}</span>
      </div>

      <div
        v-else-if="hasSearched && results.length === 0"
        class="flex flex-col items-center justify-center py-24 text-gray-400"
      >
        <Search
          class="mb-4 opacity-50"
          :size="48"
        />
        <p class="font-bold text-lg text-gray-500">暂无数据</p>
        <p class="text-sm mt-1">没有找到任何匹配结果</p>
      </div>

      <div
        v-else-if="!hasSearched"
        class="flex flex-col items-center justify-center py-32 text-gray-300"
      >
        <Search
          class="mb-4 opacity-30"
          :size="64"
        />
        <p class="font-bold text-xl text-gray-400">输入关键词开始搜索</p>
      </div>

      <!-- 用户搜索结果 (列表或小网格) -->
      <div
        v-else-if="searchType === 'users'"
        class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 pb-10"
      >
        <div
          v-for="user in results"
          :key="user.id"
          class="bg-white rounded-xl p-4 border border-gray-100 hover:border-amber-300 transition-all shadow-sm hover:shadow-md flex items-center gap-4 cursor-pointer group"
          @click="openUserDetail(user.id)"
        >
          <div class="relative flex-shrink-0">
            <VrcAvatar
              :user="user"
              custom-class="w-14 h-14 rounded-full object-cover bg-gray-50 border border-gray-200"
            />
            <div
              class="absolute -bottom-0.5 -right-0.5 w-4 h-4 rounded-full border-2 border-white"
              :class="getStatusColor(user.status || 'offline')"
            />
          </div>
          <div class="flex-1 min-w-0">
            <h3 class="font-bold text-gray-900 truncate text-base group-hover:text-amber-700 transition-colors">
              {{ user.displayName }}
            </h3>
            <p class="text-[12px] text-gray-500 truncate mt-0.5 font-medium">
              {{ user.statusDescription || getStatusLabel(user.status) }}
            </p>
          </div>
        </div>
      </div>

      <!-- 世界搜索结果 (大网格瀑布流风格) -->
      <div
        v-else-if="searchType === 'worlds'"
        class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-5 pb-10"
      >
        <div
          v-for="world in results"
          :key="world.id"
          class="bg-white rounded-xl overflow-hidden border border-gray-100 hover:border-amber-300 transition-all shadow-sm hover:shadow-lg cursor-pointer group flex flex-col"
          @click="openWorldDetail(world.id)"
        >
          <div class="aspect-video bg-gray-100 overflow-hidden relative">
            <VrcAvatar
              :user="world"
              :url="world.imageUrl || world.thumbnailImageUrl"
              custom-class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-500"
            />
            <div class="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity flex flex-col justify-end p-3">
               <span class="text-white text-xs font-bold drop-shadow-md">查看世界详情</span>
            </div>
            <!-- 左上角标签 -->
            <div class="absolute top-2 left-2 flex gap-1">
               <span v-if="world.capacity" class="bg-black/60 backdrop-blur text-white text-[10px] font-bold px-2 py-1 rounded-md shadow flex items-center gap-1">
                 <Users class="w-3 h-3"/> {{ world.capacity }}
               </span>
            </div>
          </div>
          <div class="p-4 flex flex-col flex-1">
            <h3 class="font-extrabold text-gray-900 truncate text-base mb-1 group-hover:text-amber-700 transition-colors">
              {{ world.name }}
            </h3>
            <p class="text-xs text-gray-500 truncate font-medium mb-3">
              由 <span class="text-gray-700">{{ world.authorName }}</span> 创作
            </p>
            <div class="mt-auto pt-3 border-t border-gray-50 flex items-center justify-between text-xs text-gray-400 font-bold">
              <span class="flex items-center gap-1"><UsersRound class="w-3.5 h-3.5"/> {{ world.visits || 0 }} 访问</span>
              <span class="flex items-center gap-1 text-red-400">❤ {{ world.favorites || 0 }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- 模型搜索结果 (竖版网格) -->
      <div
        v-else-if="searchType === 'avatars'"
        class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6 gap-5 pb-10"
      >
        <div
          v-for="avatar in results"
          :key="avatar.id"
          class="bg-white rounded-xl overflow-hidden border border-gray-100 hover:border-amber-300 transition-all shadow-sm hover:shadow-lg cursor-pointer group flex flex-col"
          @click="openAvatarDetail(avatar)"
        >
          <div class="aspect-[3/4] bg-gray-100 overflow-hidden relative">
            <VrcAvatar
              :user="avatar"
              :url="avatar.imageUrl || avatar.thumbnailImageUrl"
              custom-class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-500"
            />
          </div>
          <div class="p-3 flex flex-col flex-1">
            <h3 class="font-bold text-gray-900 truncate text-sm mb-1 group-hover:text-amber-700 transition-colors">
              {{ avatar.name }}
            </h3>
            <p class="text-[11px] text-gray-500 truncate font-medium mt-auto">
              {{ t('search.author') }}: {{ avatar.authorName }}
            </p>
          </div>
        </div>
      </div>

      <!-- 群组搜索结果 (列表或网格) -->
      <div
        v-else-if="searchType === 'groups'"
        class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5 pb-10"
      >
        <div
          v-for="group in results"
          :key="group.id"
          class="bg-white rounded-xl overflow-hidden border border-gray-100 hover:border-amber-300 transition-all shadow-sm hover:shadow-lg flex flex-col group cursor-pointer"
          @click="openGroupDetail(group)"
        >
          <div class="h-24 bg-gray-100 relative overflow-hidden">
            <VrcAvatar
              :user="group"
              :url="group.bannerUrl || group.iconUrl"
              custom-class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-500 opacity-60"
            />
            <div class="absolute inset-0 bg-gradient-to-t from-black/50 to-transparent"></div>
          </div>
          <div class="p-4 relative -mt-10">
            <div class="flex items-end gap-3 mb-2">
              <VrcAvatar
                :user="group"
                :url="group.iconUrl || group.bannerUrl"
                custom-class="w-16 h-16 rounded-xl border-4 border-white shadow-md bg-white flex-shrink-0"
              />
              <div class="flex-1 min-w-0">
                <h3 class="font-bold text-gray-900 truncate text-base group-hover:text-amber-700 transition-colors">
                  {{ group.name }}
                </h3>
                <p class="text-xs font-bold text-gray-500 uppercase">
                  {{ group.shortCode }}
                </p>
              </div>
            </div>
            <p class="text-[11px] text-gray-500 line-clamp-2 mt-2 font-medium">
              {{ group.description || t('search.no_description') || '暂无简介' }}
            </p>
            <div class="mt-4 pt-3 border-t border-gray-50 flex items-center justify-between text-xs font-bold text-gray-400">
              <span class="flex items-center gap-1"><UsersRound class="w-3.5 h-3.5"/> {{ group.memberCount || 0 }} 成员</span>
              <span class="bg-amber-50 text-amber-600 px-2 py-0.5 rounded">{{ group.privacy === 'public' ? '公开' : '私密' }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 用户详情弹窗 -->
    <BaseModal
      :show="!!selectedUser"
      :loading="loadingUser"
      @close="selectedUser = null"
    >
      <template v-if="selectedUser">
        <div class="h-36 bg-gray-900 relative overflow-hidden">
          <VrcAvatar
            :user="selectedUser"
            :url="selectedUser.currentAvatarImageUrl || selectedUser.currentAvatarThumbnailImageUrl"
            custom-class="w-full h-full object-cover opacity-60"
          />
          <button
            class="absolute top-3 right-3 p-1.5 rounded-full bg-black/40 hover:bg-black/60 text-white backdrop-blur transition-colors"
            @click="selectedUser = null"
          >
            ✕
          </button>
        </div>
        <div class="p-6 -mt-10 relative">
          <div class="flex items-end gap-4 mb-4">
            <VrcAvatar
              :user="selectedUser"
              custom-class="w-24 h-24 rounded-2xl object-cover border-4 border-white shadow-xl bg-gray-100"
            />
            <div class="flex-1 min-w-0 pb-2">
              <h2 class="text-2xl font-black text-gray-900 truncate">
                {{ selectedUser.displayName }}
              </h2>
              <div class="flex items-center gap-2 mt-1">
                <span
                  class="w-3 h-3 rounded-full border border-white/50 shadow-sm"
                  :class="getStatusColor(selectedUser.status || 'offline')"
                />
                <span class="text-sm font-bold text-gray-600">{{ getStatusLabel(selectedUser.status || 'offline') }}</span>
              </div>
            </div>
          </div>
          <div
            v-if="selectedUser.statusDescription"
            class="bg-gray-50 rounded-xl p-3 mb-4 text-sm text-gray-700 italic border border-gray-100"
          >
            "{{ selectedUser.statusDescription }}"
          </div>
          <div class="grid grid-cols-2 gap-3 mb-4">
            <div class="bg-gray-50 rounded-xl p-3 border border-gray-100">
              <p class="text-[10px] text-gray-400 uppercase font-bold mb-1">
                {{ t('search.trust_level') }}
              </p>
              <p class="text-sm font-black text-gray-700 truncate">
                {{ selectedUser.tags?.find((t: string) => t.startsWith('system_trust'))?.replace('system_trust_', '') || 'visitor' }}
              </p>
            </div>
            <div class="bg-gray-50 rounded-xl p-3 border border-gray-100">
              <p class="text-[10px] text-gray-400 uppercase font-bold mb-1">
                {{ t('search.last_login') }}
              </p>
              <p class="text-sm font-black text-gray-700 truncate">
                {{ selectedUser.last_login?.slice(0,10) || t('search.unknown') }}
              </p>
            </div>
          </div>
          <div
            v-if="selectedUser.bio"
            class="mb-4"
          >
            <p class="text-[10px] text-gray-400 uppercase font-bold mb-1">
              {{ t('search.bio') }}
            </p>
            <p class="text-sm text-gray-700 leading-relaxed whitespace-pre-wrap max-h-40 overflow-y-auto custom-scrollbar">
              {{ selectedUser.bio }}
            </p>
          </div>
          <div
            v-if="selectedUser.bioLinks?.length"
            class="flex flex-wrap gap-2 mb-4"
          >
            <a
              v-for="(link, i) in selectedUser.bioLinks"
              :key="i"
              :href="link"
              target="_blank"
              class="text-xs text-blue-600 hover:text-blue-800 flex items-center gap-1 bg-blue-50 hover:bg-blue-100 transition-colors px-3 py-1.5 rounded-lg font-medium"
            >
              <ExternalLink :size="12" /> {{ link.replace(/^https?:\/\//, '').slice(0, 30) }}
            </a>
          </div>

          <!-- 本地备注 (SQLite) -->
          <div class="mb-4">
            <p class="text-[10px] text-amber-600 uppercase font-bold mb-1">
              {{ t('search.local_note') }}
            </p>
            <textarea
              v-model="userNote"
              :placeholder="t('search.local_note_placeholder')"
              class="w-full bg-amber-50/30 border border-amber-200 rounded-xl p-3 text-sm text-gray-900 placeholder-gray-400 focus:outline-none focus:border-amber-500 focus:ring-2 focus:ring-amber-500/20 resize-none h-20 transition-all font-medium" 
              @blur="saveUserNote"
            />
          </div>
          
          <div class="mt-5 pt-4 border-t border-gray-100 flex items-center justify-end">
            <button 
              v-if="!selectedUser.isFriend"
              :disabled="isSendingFriendRequest || friendRequestSuccess"
              class="px-6 py-2.5 bg-gray-900 hover:bg-black text-white font-bold rounded-xl text-sm transition-colors disabled:opacity-50 flex items-center gap-2 shadow-sm"
              @click="sendFriendRequest"
            >
              <UserPlus :size="16" /> {{ isSendingFriendRequest ? t('search.sending') : (friendRequestSuccess ? t('search.friend_request_sent') : t('search.send_friend_request')) }}
            </button>
            <span
              v-else
              class="text-green-700 font-bold text-sm bg-green-50 border border-green-100 px-5 py-2.5 rounded-xl flex items-center gap-2"
            >
              已是好友
            </span>
            <span
              v-if="friendRequestError"
              class="ml-3 text-xs text-red-500 font-bold"
            >{{ friendRequestError }}</span>
          </div>
        </div>
      </template>
    </BaseModal>

    <!-- 世界详情弹窗 -->
    <BaseModal
      :show="!!selectedWorld"
      :loading="loadingWorld"
      @close="selectedWorld = null"
    >
      <template v-if="selectedWorld">
        <div class="aspect-video bg-gray-900 relative overflow-hidden rounded-t-2xl">
          <VrcAvatar
            :user="selectedWorld"
            :url="selectedWorld.imageUrl || selectedWorld.thumbnailImageUrl"
            custom-class="w-full h-full object-cover"
          />
          <button
            class="absolute top-4 right-4 p-2 rounded-full bg-black/40 hover:bg-black/60 text-white backdrop-blur transition-colors"
            @click="selectedWorld = null"
          >
            ✕
          </button>
        </div>
        <div class="p-6">
          <h2 class="text-2xl font-black text-gray-900 mb-1 leading-tight">
            {{ selectedWorld.name }}
          </h2>
          <p class="text-sm text-amber-700 font-bold mb-4">
            {{ t('search.author') }}: {{ selectedWorld.authorName }}
          </p>
          <div class="grid grid-cols-3 gap-3 text-center mb-5">
            <div class="bg-gray-50 border border-gray-100 rounded-xl p-3">
              <p class="text-lg font-black text-gray-800">
                {{ selectedWorld.capacity || '?' }}
              </p>
              <p class="text-[10px] text-gray-400 font-bold uppercase tracking-wider mt-1">
                {{ t('search.capacity') }}
              </p>
            </div>
            <div class="bg-gray-50 border border-gray-100 rounded-xl p-3">
              <p class="text-lg font-black text-gray-800">
                {{ selectedWorld.favorites || 0 }}
              </p>
              <p class="text-[10px] text-gray-400 font-bold uppercase tracking-wider mt-1">
                {{ t('search.favorites_count') }}
              </p>
            </div>
            <div class="bg-gray-50 border border-gray-100 rounded-xl p-3">
              <p class="text-lg font-black text-gray-800">
                {{ selectedWorld.visits || 0 }}
              </p>
              <p class="text-[10px] text-gray-400 font-bold uppercase tracking-wider mt-1">
                {{ t('search.visits') }}
              </p>
            </div>
          </div>
          
          <div class="mb-4">
            <p class="text-sm text-gray-600 leading-relaxed whitespace-pre-wrap max-h-48 overflow-y-auto custom-scrollbar pr-2">
              {{ selectedWorld.description || '暂无简介' }}
            </p>
          </div>
                
          <div
            v-if="selectedWorld.tags?.length"
            class="flex flex-wrap gap-1.5 mb-5"
          >
            <span
              v-for="tag in selectedWorld.tags.filter((t: string) => !t.startsWith('system_') && !t.startsWith('admin_'))"
              :key="tag"
              class="text-[10px] font-bold bg-gray-100 border border-gray-200 text-gray-600 px-2.5 py-1 rounded-md uppercase"
            >{{ tag }}</span>
          </div>

          <div class="pt-4 border-t border-gray-100 flex items-center justify-end">
            <button
              class="px-6 py-2.5 font-bold rounded-xl text-sm transition-all flex items-center gap-2 shadow-sm" 
              :class="isWorldFavorited ? 'bg-red-50 border border-red-200 text-red-600 hover:bg-red-100' : 'bg-amber-50 border border-amber-200 text-amber-700 hover:bg-amber-100'"
              @click="toggleFavoriteWorld"
            >
              <span>{{ isWorldFavorited ? t('search.remove_favorite') : '⭐ ' + t('search.add_favorite') }}</span>
            </button>
          </div>
        </div>
      </template>
    </BaseModal>

    <!-- 模型详情弹窗 -->
    <BaseModal
      :show="!!selectedAvatar"
      :loading="loadingAvatar"
      @close="selectedAvatar = null"
    >
      <template v-if="selectedAvatar">
        <div class="aspect-[3/4] max-h-80 w-full bg-gray-900 relative overflow-hidden rounded-t-2xl">
          <VrcAvatar
            :user="selectedAvatar"
            :url="selectedAvatar.imageUrl || selectedAvatar.thumbnailImageUrl"
            custom-class="w-full h-full object-cover"
          />
          <button
            class="absolute top-4 right-4 p-2 rounded-full bg-black/40 hover:bg-black/60 text-white backdrop-blur transition-colors"
            @click="selectedAvatar = null"
          >
            ✕
          </button>
        </div>
        <div class="p-6">
          <h2 class="text-xl font-black text-gray-900 mb-1 leading-tight">
            {{ selectedAvatar.name }}
          </h2>
          <p class="text-sm text-amber-700 font-bold mb-4">
            {{ t('search.author') }}: {{ selectedAvatar.authorName }}
          </p>
          <p class="text-sm text-gray-600 mb-4 leading-relaxed whitespace-pre-wrap max-h-40 overflow-y-auto custom-scrollbar">
            {{ selectedAvatar.description || '暂无简介' }}
          </p>
          
          <div
            v-if="selectedAvatar.tags?.length"
            class="flex flex-wrap gap-1.5 mb-5"
          >
            <span
              v-for="tag in selectedAvatar.tags"
              :key="tag"
              class="text-[10px] font-bold bg-gray-100 border border-gray-200 text-gray-600 px-2.5 py-1 rounded-md uppercase"
            >{{ tag }}</span>
          </div>

          <div class="pt-4 border-t border-gray-100 flex items-center justify-end">
            <button
              class="px-6 py-2.5 font-bold rounded-xl text-sm transition-all flex items-center gap-2 shadow-sm" 
              :class="isAvatarFavorited ? 'bg-red-50 border border-red-200 text-red-600 hover:bg-red-100' : 'bg-amber-50 border border-amber-200 text-amber-700 hover:bg-amber-100'"
              @click="toggleFavoriteAvatar"
            >
              <span>{{ isAvatarFavorited ? t('search.remove_favorite') : '⭐ ' + t('search.add_favorite') }}</span>
            </button>
          </div>
        </div>
      </template>
    </BaseModal>

    <!-- 群组详情弹窗 -->
    <BaseModal
      :show="!!selectedGroup"
      :loading="loadingGroup"
      @close="selectedGroup = null"
    >
      <template v-if="selectedGroup">
        <div class="h-32 bg-gray-900 relative overflow-hidden rounded-t-2xl">
          <VrcAvatar
            :user="selectedGroup"
            :url="selectedGroup.bannerUrl"
            custom-class="w-full h-full object-cover opacity-80"
          />
          <button
            class="absolute top-4 right-4 p-2 rounded-full bg-black/40 hover:bg-black/60 text-white backdrop-blur transition-colors"
            @click="selectedGroup = null"
          >
            ✕
          </button>
        </div>
        <div class="p-6 relative">
          <div class="flex gap-4 mb-4">
            <div class="w-20 h-20 -mt-12 rounded-xl border-4 border-white shadow-md bg-white flex-shrink-0 relative z-10 overflow-hidden">
              <VrcAvatar
                :user="selectedGroup"
                :url="selectedGroup.iconUrl"
                custom-class="w-full h-full object-cover"
              />
            </div>
            <div class="flex-1 pb-1 min-w-0">
              <h2 class="text-xl font-black text-gray-900 truncate">
                {{ selectedGroup.name }}
              </h2>
              <div class="flex items-center gap-2 mt-1">
                <span class="text-xs font-bold text-gray-500 uppercase">{{ selectedGroup.shortCode }}</span>
                <span class="w-1 h-1 rounded-full bg-gray-300" />
                <span class="text-xs font-bold text-amber-600 flex items-center gap-1"><UsersRound :size="12"/> {{ selectedGroup.memberCount || 0 }} 成员</span>
              </div>
            </div>
          </div>
          
          <div class="mb-5">
            <p class="text-sm text-gray-700 leading-relaxed whitespace-pre-wrap max-h-48 overflow-y-auto custom-scrollbar">
              {{ selectedGroup.description || '暂无群组简介' }}
            </p>
          </div>

          <div class="grid grid-cols-2 gap-3 mb-5">
            <div class="bg-gray-50 border border-gray-100 rounded-xl p-3 flex flex-col justify-center">
              <p class="text-[10px] text-gray-400 font-bold uppercase tracking-wider mb-1">隐私状态</p>
              <p class="text-sm font-black text-gray-800 flex items-center gap-1">
                 <Shield :size="14" class="text-blue-500"/> {{ selectedGroup.privacy === 'public' ? '公开群组' : '私密群组' }}
              </p>
            </div>
            <div class="bg-gray-50 border border-gray-100 rounded-xl p-3 flex flex-col justify-center">
              <p class="text-[10px] text-gray-400 font-bold uppercase tracking-wider mb-1">加入方式</p>
              <p class="text-sm font-black text-gray-800 flex items-center gap-1">
                 <Check :size="14" class="text-green-500"/> {{ selectedGroup.joinState === 'open' ? '自由加入' : (selectedGroup.joinState === 'request' ? '需申请' : '邀请制') }}
              </p>
            </div>
          </div>
          
          <div class="pt-4 border-t border-gray-100 flex items-center justify-between">
            <div class="text-xs text-gray-400 font-mono">{{ selectedGroup.id }}</div>
            <button
              class="px-6 py-2.5 bg-indigo-600 hover:bg-indigo-700 text-white font-bold rounded-xl text-sm transition-colors shadow-sm" 
            >
              在 VRChat 中查看
            </button>
          </div>
        </div>
      </template>
    </BaseModal>
  </div>
</template>

<style scoped>
.fade-enter-active, .fade-leave-active { transition: opacity 0.3s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: #cbd5e1; border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: #94a3b8; }
</style>
