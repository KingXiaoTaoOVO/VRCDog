<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { VrcApi, DbApi, SysApi, GamelogApi } from "../api";
import { UserCircle, Loader2, PlayCircle, Eye, RefreshCcw, Search } from 'lucide-vue-next';
import VrcAvatarComp from './VrcAvatar.vue';
import { useI18n } from 'vue-i18n';
import type { VrcAvatar } from '../types/vrc';

const { t } = useI18n();

const avatars = ref<VrcAvatar[]>([]);
const loading = ref(true);
const errorMsg = ref('');
const processingId = ref<string | null>(null);
const searchQuery = ref('');

const fetchAvatars = async () => {
  loading.value = true;
  errorMsg.value = '';
  try {
    const res: any = await VrcApi.getAvatars({ user: 'me', n: 100, offset: 0 });
    avatars.value = Array.isArray(res) ? res : [];
  } catch (err: any) {
    errorMsg.value = err.message || err;
  } finally {
    loading.value = false;
  }
};

const selectAvatar = async (id: string) => {
  processingId.value = id;
  try {
    await VrcApi.selectAvatar({ avatarId: id });
  } catch (err: any) {
    console.error('Select avatar failed:', err);
  } finally {
    processingId.value = null;
  }
};

const filteredAvatars = computed(() => {
  if (!searchQuery.value) return avatars.value;
  const lower = searchQuery.value.toLowerCase();
  return avatars.value.filter(a => a.name.toLowerCase().includes(lower) || (a.description && a.description.toLowerCase().includes(lower)));
});

onMounted(() => {
  fetchAvatars();
});
</script>

<template>
  <div class="h-full flex flex-col">
    <div class="flex flex-col sm:flex-row sm:items-end justify-between mb-6 gap-4">
      <div>
        <h1 class="text-3xl font-extrabold text-[#451a03] tracking-tight flex items-center gap-3">
          <span class="inline-flex items-center justify-center p-1.5 bg-amber-100 rounded-xl">
            <UserCircle
              class="text-amber-600"
              :size="24"
            />
          </span>
          {{ t('my_avatars.title') }}
        </h1>
        <p class="text-amber-700/80 font-medium mt-1">
          {{ t('my_avatars.subtitle') }}
        </p>
      </div>
      
      <div class="flex items-center gap-2">
        <div class="relative">
          <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
            <Search class="h-4 w-4 text-amber-400" />
          </div>
          <input
            v-model="searchQuery"
            type="text"
            class="block w-64 pl-10 pr-4 py-2 bg-white/80 backdrop-blur border border-amber-200 rounded-xl text-amber-900 placeholder-amber-400 focus:outline-none focus:border-amber-400 text-sm transition-colors"
            :placeholder="t('my_avatars.search_placeholder')"
          >
        </div>
        
        <button
          :disabled="loading"
          class="px-4 py-2 bg-white rounded-xl text-amber-700 font-bold border border-amber-200 shadow-sm hover:shadow-md transition-all flex items-center gap-2 disabled:opacity-50"
          @click="fetchAvatars"
        >
          <Loader2
            v-if="loading"
            class="animate-spin"
            :size="16"
          />
          <RefreshCcw
            v-else
            :size="16"
          />
          <span class="hidden sm:inline">{{ t('my_avatars.refresh') }}</span>
        </button>
      </div>
    </div>

    <div
      v-if="errorMsg"
      class="bg-red-50 text-red-600 p-3 rounded-xl border border-red-200 text-sm font-bold mb-4"
    >
      {{ errorMsg }}
    </div>

    <div class="flex-1 overflow-y-auto pr-1 custom-scrollbar">
      <div
        v-if="loading && avatars.length === 0"
        class="flex items-center justify-center py-12 text-amber-500 font-bold"
      >
        <Loader2
          class="animate-spin mr-2"
          :size="24"
        /> {{ t('my_avatars.loading') }}
      </div>

      <div
        v-else-if="avatars.length === 0"
        class="text-center text-amber-500 py-12 text-sm bg-white/50 backdrop-blur rounded-2xl border-2 border-dashed border-amber-200 font-bold"
      >
        <UserCircle
          class="mx-auto mb-3 opacity-50"
          :size="48"
        />
        {{ t('my_avatars.no_avatars') }} 🐕
      </div>

      <div
        v-else
        class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4"
      >
        <div
          v-for="avatar in filteredAvatars"
          :key="avatar.id" 
          class="bg-white/80 backdrop-blur rounded-2xl overflow-hidden border border-amber-100 hover:border-amber-300 transition-all shadow-sm hover:shadow-md group relative"
        >
          <div class="h-48 bg-amber-50 relative overflow-hidden">
            <VrcAvatarComp
              :user="avatar"
              custom-class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-500"
            />
            <div
              class="absolute top-2 left-2 px-2 py-0.5 rounded-md text-[10px] font-bold text-white shadow-sm flex items-center gap-1 backdrop-blur-md"
              :class="avatar.releaseStatus === 'public' ? 'bg-green-500/80' : 'bg-red-500/80'"
            >
              <Eye :size="10" /> {{ avatar.releaseStatus === 'public' ? t('my_avatars.public') : t('my_avatars.private') }}
            </div>
            <div class="absolute bottom-2 right-2 flex gap-1">
              <div
                v-if="avatar.supportedPlatforms && avatar.supportedPlatforms.includes('standalonewindows')"
                class="bg-blue-600/80 backdrop-blur text-white text-[9px] font-bold px-1.5 py-0.5 rounded shadow"
              >
                PC
              </div>
              <div
                v-if="avatar.supportedPlatforms && avatar.supportedPlatforms.includes('android')"
                class="bg-green-600/80 backdrop-blur text-white text-[9px] font-bold px-1.5 py-0.5 rounded shadow"
              >
                Quest
              </div>
            </div>
          </div>
          
          <div class="p-4">
            <h3
              class="font-bold text-amber-900 text-sm truncate mb-1"
              :title="avatar.name"
            >
              {{ avatar.name }}
            </h3>
            <p class="text-[10px] text-amber-600 line-clamp-2 leading-relaxed h-7 mb-3">
              {{ avatar.description || t('my_avatars.no_description') }}
            </p>
            
            <button
              :disabled="processingId === avatar.id"
              class="w-full bg-amber-500 hover:bg-amber-600 text-white font-bold py-2 px-4 rounded-xl shadow-md transition-all flex items-center justify-center gap-2 disabled:opacity-50 text-xs"
              @click="selectAvatar(avatar.id)"
            >
              <Loader2
                v-if="processingId === avatar.id"
                class="animate-spin"
                :size="14"
              />
              <PlayCircle
                v-else
                :size="14"
              />
              {{ t('my_avatars.wear_avatar') }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
