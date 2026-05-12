<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue';
import { VrcApi, DbApi, SysApi, GamelogApi } from "../api";
import { Heart, Globe, UserCircle, Loader2 } from 'lucide-vue-next';
import VrcAvatarComp from './VrcAvatar.vue';
import VrcResourceCard from './VrcResourceCard.vue';
import { useI18n } from 'vue-i18n';
import type { VrcWorld, VrcAvatar } from '../types/vrc';

const { t } = useI18n();

const favoriteWorlds = ref<VrcWorld[]>([]);
const favoriteAvatars = ref<VrcAvatar[]>([]);
const favGroups = ref<any[]>([]);

const loadingWorlds = ref(true);
const loadingAvatars = ref(true);
const loadingGroups = ref(true);

const activeTab = ref<'worlds' | 'avatars'>('worlds');
const activeGroup = ref<string>('all');

const errorMsg = ref('');

const fetchGroups = async () => {
  loadingGroups.value = true;
  try {
    const res: any = await VrcApi.getFavoriteGroups();
    favGroups.value = Array.isArray(res) ? res : [];
  } catch (err: any) {
    console.warn('Failed to fetch favorite groups:', err);
  } finally {
    loadingGroups.value = false;
  }
};

const fetchFavorites = async () => {
  errorMsg.value = '';
  
  if (activeTab.value === 'worlds') {
    loadingWorlds.value = true;
    try {
      const localRes: any = activeGroup.value === 'all' ? await DbApi.getFavoriteWorlds() : [];
      const vrcRes: any = await VrcApi.getFavoriteWorlds({ tag: activeGroup.value === 'all' ? undefined : activeGroup.value });
      
      const local = Array.isArray(localRes) ? localRes.map(w => ({...w, id: w.world_id || w.id, imageUrl: w.image_url, authorName: w.author_name})) : [];
      const vrc = Array.isArray(vrcRes) ? vrcRes : [];
      
      const combined = [...vrc, ...local];
      favoriteWorlds.value = combined.filter((v, i, a) => a.findIndex(t => (t.id === v.id)) === i);
    } catch (err: any) {
      errorMsg.value = t('favorites.error_worlds') || t('auto_8de7d1ac');
    } finally {
      loadingWorlds.value = false;
    }
  } else {
    loadingAvatars.value = true;
    try {
      const localRes: any = activeGroup.value === 'all' ? await DbApi.getFavoriteAvatars() : [];
      const vrcRes: any = await VrcApi.getFavoriteAvatars({ tag: activeGroup.value === 'all' ? undefined : activeGroup.value });
      
      const local = Array.isArray(localRes) ? localRes.map(a => ({...a, id: a.avatar_id || a.id, imageUrl: a.image_url, authorName: a.author_name})) : [];
      const vrc = Array.isArray(vrcRes) ? vrcRes : [];
      
      const combined = [...vrc, ...local];
      favoriteAvatars.value = combined.filter((v, i, a) => a.findIndex(t => (t.id === v.id)) === i);
    } catch (err: any) {
      errorMsg.value = t('favorites.error_avatars') || t('auto_11c6e2cc');
    } finally {
      loadingAvatars.value = false;
    }
  }
};

watch(activeTab, () => {
  activeGroup.value = 'all';
  fetchFavorites();
});

watch(activeGroup, () => {
  fetchFavorites();
});

const getGroupsByType = (type: string) => {
  return favGroups.value.filter(g => g.type === type);
};

onMounted(() => {
  fetchGroups();
  fetchFavorites();
});
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-surface-hover rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-primary/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <!-- 顶部控制栏 -->
    <div class="flex items-center justify-between mb-8 shrink-0 z-10">
      <h1 class="text-3xl font-extrabold text-text tracking-tight flex items-center gap-3">
        <span class="inline-flex items-center justify-center p-2 bg-primary/10 rounded-2xl shadow-sm border-primary">
          <Heart class="w-6 h-6 text-primary" />
        </span>
        {{ t('favorites.title') }}
      </h1>
      <div class="flex rounded-xl border-border-soft overflow-hidden bg-surface text-sm font-bold shadow-sm p-1">
        <button
          :class="activeTab === 'worlds' ? 'bg-primary text-white rounded-lg shadow-md shadow-indigo-500/20' : 'text-text-muted hover:bg-surface rounded-lg'"
          class="px-5 py-2 flex items-center gap-2 transition-all"
          @click="activeTab = 'worlds'"
        >
          <Globe :size="16" /> {{ t('favorites.worlds') }}
        </button>
        <button
          :class="activeTab === 'avatars' ? 'bg-primary text-white rounded-lg shadow-md shadow-indigo-500/20' : 'text-text-muted hover:bg-surface rounded-lg'"
          class="px-5 py-2 flex items-center gap-2 transition-all"
          @click="activeTab = 'avatars'"
        >
          <UserCircle :size="16" /> {{ t('favorites.avatars') }}
        </button>
      </div>
    </div>

    <div
      v-if="errorMsg"
      class="bg-red-50 text-red-600 p-3 rounded-xl border-red-200 text-sm font-bold mb-4 z-10"
    >
      {{ errorMsg }}
    </div>

    <!-- 主体：侧边栏 + 内容区 -->
    <div class="flex-1 flex gap-6 overflow-hidden z-10">
      <!-- 左侧收藏夹列表 -->
      <div class="w-56 flex-shrink-0 bg-surface backdrop-blur-xl rounded-2xl border-border-strong shadow-lg shadow-slate-200/40 p-2 overflow-y-auto flex flex-col gap-1 hide-scrollbar">
        <button
          class="px-4 py-3 rounded-xl text-left text-sm font-bold transition-all w-full flex items-center justify-between"
          :class="activeGroup === 'all' ? 'bg-primary text-white shadow-md shadow-indigo-500/20' : 'text-text-muted hover:bg-surface'"
          @click="activeGroup = 'all'"
        >
          <span>{{ t('favorites.all_groups') === 'favorites.all_groups' ? t('favorites.all_groups_fallback') : t('favorites.all_groups') }}</span>
        </button>
        
        <div
          v-if="loadingGroups"
          class="py-4 text-center text-primary"
        >
          <Loader2
            class="animate-spin mx-auto"
            :size="20"
          />
        </div>
        
        <template v-else>
          <button
            v-for="group in getGroupsByType(activeTab === 'worlds' ? 'world' : 'avatar')"
            :key="group.id"
            class="px-4 py-3 rounded-xl text-left text-sm font-bold transition-all w-full flex items-center justify-between group/btn"
            :class="activeGroup === group.name ? 'bg-primary text-white shadow-md shadow-indigo-500/20' : 'text-text-muted hover:bg-surface'"
            @click="activeGroup = group.name"
          >
            <span
              class="truncate flex-1"
              :title="group.displayName"
            >{{ group.displayName }}</span>
            <span
              class="text-[10px] font-mono px-1.5 py-0.5 rounded-md ml-2 flex-shrink-0 transition-colors"
              :class="activeGroup === group.name ? 'bg-primary/10 text-primary' : 'bg-background/20 text-text-muted group-hover/btn:bg-surface'"
            >
              {{ group.visibility === 'private' ? t('global.fav.private') : t('global.fav.public') }}
            </span>
          </button>
        </template>
      </div>

      <!-- 右侧内容区 -->
      <div class="flex-1 overflow-y-auto pr-2 custom-scrollbar">
        <!-- Worlds Tab -->
        <div v-if="activeTab === 'worlds'">
          <div
            v-if="loadingWorlds"
            class="flex items-center justify-center py-12 text-primary font-bold"
          >
            <Loader2
              class="animate-spin mr-2"
              :size="24"
            /> {{ t('favorites.loading_worlds') }}
          </div>
          <div
            v-else-if="favoriteWorlds.length === 0"
            class="flex flex-col items-center justify-center py-32 text-border-strong"
          >
            <Heart
              class="mb-4 opacity-30"
              :size="64"
            />
            <p class="font-bold text-xl text-text-muted">
              {{ t('search.no_results') }}
            </p>
            <p class="text-sm mt-2 font-medium">
              {{ t('favorites.no_worlds') }}
            </p>
          </div>
          <div
            v-else
            class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-5"
          >
            <VrcResourceCard
              v-for="world in favoriteWorlds"
              :key="world.id" 
              type="world"
              :data="world"
            />
          </div>
        </div>

        <!-- Avatars Tab -->
        <div v-if="activeTab === 'avatars'">
          <div
            v-if="loadingAvatars"
            class="flex items-center justify-center py-12 text-primary font-bold"
          >
            <Loader2
              class="animate-spin mr-2"
              :size="24"
            /> {{ t('favorites.loading_avatars') }}
          </div>
          <div
            v-else-if="favoriteAvatars.length === 0"
            class="flex flex-col items-center justify-center py-32 text-border-strong"
          >
            <Heart
              class="mb-4 opacity-30"
              :size="64"
            />
            <p class="font-bold text-xl text-text-muted">
              {{ t('search.no_results') }}
            </p>
            <p class="text-sm mt-2 font-medium">
              {{ t('favorites.no_avatars') }}
            </p>
          </div>
          <div
            v-else
            class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6 gap-5 pb-10"
          >
            <VrcResourceCard
              v-for="avatar in favoriteAvatars"
              :key="avatar.id"
              type="avatar"
              :data="avatar"
            />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>

.hide-scrollbar {
  -ms-overflow-style: none;
  scrollbar-width: none;
}
</style>
