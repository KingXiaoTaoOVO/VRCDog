<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { VrcApi, DbApi, SysApi, GamelogApi } from "../api";
import { UserCircle, Loader2, PlayCircle, Eye, RefreshCcw, Search } from 'lucide-vue-next';
import VrcAvatarComp from './VrcAvatar.vue';
import VrcResourceCard from './VrcResourceCard.vue';
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
  <div class="h-full flex flex-col p-6 bg-surface-hover rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-primary/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <div class="flex flex-col sm:flex-row sm:items-center justify-between mb-8 gap-4 shrink-0 z-10">
      <div>
        <h1 class="text-3xl font-extrabold text-text tracking-tight flex items-center gap-3">
          <span class="inline-flex items-center justify-center p-2 bg-primary/10 rounded-2xl shadow-sm border-primary">
            <UserCircle class="w-6 h-6 text-primary" />
          </span>
          {{ t('my_avatars.title') }}
        </h1>
      </div>
      
      <div class="flex items-center gap-2">
        <div class="relative">
          <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
            <Search class="h-4 w-4 text-border-strong" />
          </div>
          <input
            v-model="searchQuery"
            type="text"
            class="block w-64 pl-10 pr-4 py-2 bg-surface border-border-soft shadow-sm rounded-xl text-text placeholder-slate-400 focus:outline-none   focus:ring-indigo-500/10 text-sm font-bold transition-all"
            :placeholder="t('my_avatars.search_placeholder')"
          >
        </div>
        
        <button
          :disabled="loading"
          class="p-2.5 rounded-xl bg-surface border-border-soft shadow-sm text-text-muted hover:text-primary hover:border-primary transition-all disabled:opacity-50"
          @click="fetchAvatars"
        >
          <Loader2
            v-if="loading"
            class="animate-spin"
            :size="20"
          />
          <RefreshCcw
            v-else
            :size="20"
          />
        </button>
      </div>
    </div>

    <div
      v-if="errorMsg"
      class="bg-red-50 text-red-600 p-3 rounded-xl border-red-200 text-sm font-bold mb-4 z-10"
    >
      {{ errorMsg }}
    </div>

    <div class="flex-1 overflow-y-auto pr-2 custom-scrollbar z-10 relative">
      <div
        v-if="loading && avatars.length === 0"
        class="absolute inset-0 flex flex-col items-center justify-center text-primary bg-surface-hover backdrop-blur-sm z-10"
      >
        <Loader2
          class="animate-spin mb-4"
          :size="48"
        />
        <span class="font-extrabold text-lg tracking-wide">{{ t('my_avatars.loading') }}</span>
      </div>

      <div
        v-else-if="avatars.length === 0"
        class="h-full flex flex-col items-center justify-center text-border-strong"
      >
        <UserCircle
          class="mb-4 opacity-30"
          :size="64"
        />
        <p class="font-bold text-xl text-text-muted">
          {{ t('my_avatars.no_avatars') }}
        </p>
      </div>

      <div
        v-else
        class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6 gap-5 pb-10"
      >
        <VrcResourceCard
          v-for="avatar in filteredAvatars"
          :key="avatar.id" 
          type="avatar"
          :data="avatar"
        />
      </div>
    </div>
  </div>
</template>


