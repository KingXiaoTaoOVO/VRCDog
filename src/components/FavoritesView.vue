<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { VrcApi, DbApi, SysApi, GamelogApi } from "../api";
import { Heart, Globe, UserCircle, Loader2 } from 'lucide-vue-next';
import VrcAvatarComp from './VrcAvatar.vue';
import { useI18n } from 'vue-i18n';
import type { VrcWorld, VrcAvatar } from '../types/vrc';

const { t } = useI18n();

const favoriteWorlds = ref<VrcWorld[]>([]);
const favoriteAvatars = ref<VrcAvatar[]>([]);
const loadingWorlds = ref(true);
const loadingAvatars = ref(true);
const activeTab = ref<'worlds' | 'avatars'>('worlds');

const errorMsg = ref('');

const fetchFavorites = async () => {
  errorMsg.value = '';
  loadingWorlds.value = true;
  loadingAvatars.value = true;
  
  try {
    const localRes: any = await DbApi.getFavoriteWorlds();
    const vrcRes: any = await VrcApi.getFavoriteWorlds({});
    const local = Array.isArray(localRes) ? localRes.map(w => ({...w, id: w.world_id || w.id, imageUrl: w.image_url, authorName: w.author_name})) : [];
    const vrc = Array.isArray(vrcRes) ? vrcRes : [];
    
    // Deduplicate by ID
    const combined = [...vrc, ...local];
    const unique = combined.filter((v, i, a) => a.findIndex(t => (t.id === v.id)) === i);
    favoriteWorlds.value = unique;
  } catch (err: any) {
    console.warn('Failed to fetch favorite worlds:', err);
    errorMsg.value = t('favorites.error_worlds') || '无法加载收藏世界';
  } finally {
    loadingWorlds.value = false;
  }

  try {
    const localRes: any = await DbApi.getFavoriteAvatars();
    const vrcRes: any = await VrcApi.getFavoriteAvatars({});
    const local = Array.isArray(localRes) ? localRes.map(a => ({...a, id: a.avatar_id || a.id, imageUrl: a.image_url, authorName: a.author_name})) : [];
    const vrc = Array.isArray(vrcRes) ? vrcRes : [];
    
    // Deduplicate by ID
    const combined = [...vrc, ...local];
    const unique = combined.filter((v, i, a) => a.findIndex(t => (t.id === v.id)) === i);
    favoriteAvatars.value = unique;
  } catch (err: any) {
    console.warn('Failed to fetch favorite avatars:', err);
    errorMsg.value = t('favorites.error_avatars') || '无法加载收藏头像';
  } finally {
    loadingAvatars.value = false;
  }
};

onMounted(() => {
  fetchFavorites();
});
</script>

<template>
  <div class="h-full flex flex-col">
    <div class="flex items-center justify-between mb-4">
      <h1 class="text-2xl font-extrabold text-[#451a03] tracking-tight flex items-center gap-2">
        <Heart
          class="text-pink-500"
          :size="28"
        /> {{ t('favorites.title') }}
      </h1>
      <div class="flex rounded-xl border border-pink-200 overflow-hidden bg-white/80 text-sm font-bold shadow-sm">
        <button
          :class="activeTab === 'worlds' ? 'bg-pink-500 text-white' : 'text-pink-700 hover:bg-pink-50'"
          class="px-4 py-2 flex items-center gap-1 transition-colors"
          @click="activeTab = 'worlds'"
        >
          <Globe :size="14" /> {{ t('favorites.worlds') }}
        </button>
        <button
          :class="activeTab === 'avatars' ? 'bg-pink-500 text-white' : 'text-pink-700 hover:bg-pink-50'"
          class="px-4 py-2 flex items-center gap-1 transition-colors"
          @click="activeTab = 'avatars'"
        >
          <UserCircle :size="14" /> {{ t('favorites.avatars') }}
        </button>
      </div>
    </div>

    <div
      v-if="errorMsg"
      class="bg-red-50 text-red-600 p-3 rounded-xl border border-red-200 text-sm font-bold mb-4"
    >
      {{ errorMsg }}
    </div>

    <div class="flex-1 overflow-y-auto pr-1">
      <!-- Worlds Tab -->
      <div v-if="activeTab === 'worlds'">
        <div
          v-if="loadingWorlds"
          class="flex items-center justify-center py-12 text-pink-500"
        >
          <Loader2
            class="animate-spin mr-2"
            :size="24"
          /> {{ t('favorites.loading_worlds') }}
        </div>
        <div
          v-else-if="favoriteWorlds.length === 0"
          class="flex flex-col items-center justify-center py-20 text-pink-900/40"
        >
          <Heart
            class="mb-4 opacity-50"
            :size="48"
          />
          <p class="font-bold text-lg">
            {{ t('search.no_results') || '暂无数据' }}
          </p>
          <p class="text-sm mt-1">
            {{ t('favorites.no_worlds') }}
          </p>
        </div>
        <div
          v-else
          class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4"
        >
          <div
            v-for="world in favoriteWorlds"
            :key="world.id" 
            class="bg-white/80 backdrop-blur rounded-2xl overflow-hidden border border-pink-100 hover:border-pink-300 transition-all shadow-sm group"
          >
            <div class="h-32 bg-pink-50 relative overflow-hidden">
              <VrcAvatarComp
                :user="world"
                :url="world.imageUrl || world.image_url || world.thumbnailImageUrl"
                custom-class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-500"
              />
            </div>
            <div class="p-4">
              <h3 class="font-bold text-pink-900 text-sm truncate">
                {{ world.name }}
              </h3>
              <p
                v-if="world.authorName || world.author_name"
                class="text-[10px] text-pink-600 truncate mt-1"
              >
                by {{ world.authorName || world.author_name }}
              </p>
            </div>
          </div>
        </div>
      </div>

      <!-- Avatars Tab -->
      <div v-if="activeTab === 'avatars'">
        <div
          v-if="loadingAvatars"
          class="flex items-center justify-center py-12 text-pink-500"
        >
          <Loader2
            class="animate-spin mr-2"
            :size="24"
          /> {{ t('favorites.loading_avatars') }}
        </div>
        <div
          v-else-if="favoriteAvatars.length === 0"
          class="flex flex-col items-center justify-center py-20 text-pink-900/40"
        >
          <Heart
            class="mb-4 opacity-50"
            :size="48"
          />
          <p class="font-bold text-lg">
            {{ t('search.no_results') || '暂无数据' }}
          </p>
          <p class="text-sm mt-1">
            {{ t('favorites.no_avatars') }}
          </p>
        </div>
        <div
          v-else
          class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4"
        >
          <div
            v-for="avatar in favoriteAvatars"
            :key="avatar.id" 
            class="bg-white/80 backdrop-blur rounded-2xl overflow-hidden border border-pink-100 hover:border-pink-300 transition-all shadow-sm group"
          >
            <div class="h-48 bg-pink-50 relative overflow-hidden">
              <VrcAvatarComp
                :user="avatar"
                :url="avatar.imageUrl || avatar.image_url || avatar.thumbnailImageUrl || avatar.currentAvatarThumbnailImageUrl || avatar.currentAvatarImageUrl"
                custom-class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-500"
              />
            </div>
            <div class="p-4">
              <h3 class="font-bold text-pink-900 text-sm truncate">
                {{ avatar.name }}
              </h3>
              <p
                v-if="avatar.authorName || avatar.author_name"
                class="text-[10px] text-pink-600 truncate mt-1"
              >
                by {{ avatar.authorName || avatar.author_name }}
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
