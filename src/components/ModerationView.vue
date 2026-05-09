<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { ShieldAlert, UserX, VolumeX, EyeOff, Search, Unlock } from 'lucide-vue-next';
import { VrcApi } from '../api';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const activeTab = ref('blocked');
const searchQuery = ref('');

const moderations = ref<any[]>([]);
const loading = ref(true);
const errorMsg = ref('');

const fetchModerations = async () => {
  loading.value = true;
  try {
    const res = await VrcApi.getModerations();
    moderations.value = res.map((m: any) => ({
      id: m.id,
      targetUserId: m.targetUserId,
      name: m.targetDisplayName,
      type: m.type === 'block' ? 'blocked' : m.type === 'mute' ? 'muted' : m.type === 'hideAvatar' ? 'hidden' : m.type,
      date: new Date(m.created).toLocaleDateString(),
      reason: '' // VRChat API doesn't usually return reasons unless locally stored
    }));
  } catch (err: any) {
    errorMsg.value = err.message || err;
  } finally {
    loading.value = false;
  }
};

onMounted(() => {
  fetchModerations();
});

const getFiltered = (type: string) => {
  return moderations.value.filter((m: any) => m.type === type && (m.name || '').toLowerCase().includes(searchQuery.value.toLowerCase()));
};

const unblock = async (id: string) => {
  try {
    await VrcApi.request('/auth/user/unplayermoderate', 'PUT', { moderated: id });
    moderations.value = moderations.value.filter((m: any) => m.id !== id);
  } catch (err: any) {
    errorMsg.value = err.message || err;
  }
};
</script>

<template>
  <div class="h-full flex flex-col">
    <header class="mb-6 flex justify-between items-end">
      <div>
        <h1 class="text-3xl font-extrabold text-[#451a03] tracking-tight flex items-center gap-3">
          {{ t('moderation.title') }}
          <span class="inline-flex items-center justify-center p-1.5 bg-red-100 rounded-xl">
            <ShieldAlert class="w-6 h-6 text-red-600" />
          </span>
        </h1>
        <p class="text-amber-700/80 font-medium mt-1">
          {{ t('moderation.subtitle') }}
        </p>
      </div>
      
      <div class="flex gap-2 bg-white/50 p-1 rounded-xl backdrop-blur border border-red-100">
        <button
          :class="activeTab === 'blocked' ? 'bg-red-500 text-white shadow-md' : 'text-red-700 hover:bg-red-100'"
          class="px-4 py-1.5 rounded-lg font-bold text-sm transition-all flex items-center gap-1"
          @click="activeTab = 'blocked'"
        >
          <UserX :size="16" /> {{ t('moderation.tab_blocked') }}
        </button>
        <button
          :class="activeTab === 'muted' ? 'bg-orange-500 text-white shadow-md' : 'text-orange-700 hover:bg-orange-100'"
          class="px-4 py-1.5 rounded-lg font-bold text-sm transition-all flex items-center gap-1"
          @click="activeTab = 'muted'"
        >
          <VolumeX :size="16" /> {{ t('moderation.tab_muted') }}
        </button>
        <button
          :class="activeTab === 'hidden' ? 'bg-purple-500 text-white shadow-md' : 'text-purple-700 hover:bg-purple-100'"
          class="px-4 py-1.5 rounded-lg font-bold text-sm transition-all flex items-center gap-1"
          @click="activeTab = 'hidden'"
        >
          <EyeOff :size="16" /> {{ t('moderation.tab_hidden') }}
        </button>
      </div>
    </header>

    <div class="flex-1 bg-white/60 backdrop-blur-md border-2 border-white rounded-3xl p-6 shadow-lg flex flex-col">
      <div class="relative mb-6 w-full max-w-md">
        <Search class="absolute left-3 top-1/2 -translate-y-1/2 text-red-300 w-5 h-5" />
        <input
          v-model="searchQuery"
          type="text"
          :placeholder="t('moderation.search_placeholder')"
          class="w-full pl-10 pr-4 py-2.5 bg-white border border-red-100 focus:border-red-400 focus:ring-0 rounded-xl outline-none transition-colors text-amber-900 font-medium"
        >
      </div>

      <div class="flex-1 overflow-y-auto custom-scrollbar pr-2 space-y-3">
        <div
          v-if="getFiltered(activeTab).length === 0"
          class="h-full flex flex-col items-center justify-center text-red-900/30"
        >
          <ShieldAlert class="w-16 h-16 mb-4 opacity-50" />
          <p class="font-bold text-lg">
            {{ t('moderation.empty') }}
          </p>
          <p class="text-sm mt-1">
            {{ t('moderation.empty_desc') }}
          </p>
        </div>

        <div
          v-for="item in getFiltered(activeTab)"
          :key="item.id"
          class="flex items-center justify-between p-4 bg-white rounded-2xl border border-red-50 hover:border-red-200 transition-colors shadow-sm group"
        >
          <div class="flex items-center gap-4">
            <div class="w-12 h-12 rounded-full bg-red-100 flex items-center justify-center text-red-500">
              <UserX v-if="item.type==='blocked'" />
              <VolumeX v-else-if="item.type==='muted'" />
              <EyeOff v-else />
            </div>
            <div>
              <h3 class="font-bold text-amber-950 text-lg">
                {{ item.name }}
              </h3>
              <div class="flex items-center gap-3 mt-1 text-xs font-bold text-red-900/40">
                <span class="bg-red-50 px-2 py-0.5 rounded-md">{{ item.date }}</span>
                <span v-if="item.reason">{{ t('moderation.reason', { reason: item.reason }) }}</span>
              </div>
            </div>
          </div>
          <button
            class="px-4 py-2 bg-red-50 hover:bg-red-500 text-red-600 hover:text-white rounded-xl font-bold transition-colors opacity-0 group-hover:opacity-100 flex items-center gap-1 text-sm"
            @click="unblock(item.id)"
          >
            <Unlock :size="14" /> {{ t('moderation.unblock') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(239, 68, 68, 0.2); border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(239, 68, 68, 0.4); }
</style>
