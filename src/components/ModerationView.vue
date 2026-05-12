<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { ShieldAlert, UserX, VolumeX, EyeOff, Search, Unlock, Shield } from 'lucide-vue-next';
import { VrcApi } from '../api';
import { useI18n } from 'vue-i18n';
import VrcResourceCard from './VrcResourceCard.vue';
import { useUserProfileStore } from '../stores/userProfile';

const { t } = useI18n();
const profileStore = useUserProfileStore();

const activeTab = ref('blocked');
const searchQuery = ref('');

const moderations = ref<any[]>([]);
const loading = ref(true);
const errorMsg = ref('');

const resolvedUsers = new Map<string, any>();

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
      reason: '', // VRChat API doesn't usually return reasons unless locally stored
      userData: null,
      loadingData: false
    }));
    
    // Begin fetching user details in background
    for (const m of moderations.value) {
       resolveModeratedUser(m);
    }
  } catch (err: any) {
    errorMsg.value = err.message || err;
  } finally {
    loading.value = false;
  }
};

const resolveModeratedUser = async (m: any) => {
  if (resolvedUsers.has(m.targetUserId)) {
     m.userData = resolvedUsers.get(m.targetUserId);
     return;
  }
  
  m.loadingData = true;
  try {
     const res = await VrcApi.request(`/api/1/users/${m.targetUserId}`, { method: 'GET' });
     m.userData = res;
     resolvedUsers.set(m.targetUserId, res);
  } catch (e) {
     console.warn(`Failed to fetch user ${m.targetUserId}`, e);
  } finally {
     m.loadingData = false;
  }
};

onMounted(() => {
  fetchModerations();
});

const filteredModerations = computed(() => {
  return moderations.value.filter((m: any) => m.type === activeTab.value && (m.name || '').toLowerCase().includes(searchQuery.value.toLowerCase()));
});

const unblock = async (id: string) => {
  try {
    await VrcApi.request('/auth/user/unplayermoderate', { method: 'PUT', params: { moderated: id } });
    moderations.value = moderations.value.filter((m: any) => m.id !== id);
  } catch (err: any) {
    errorMsg.value = err.message || err;
  }
};

const openPlayerProfile = (m: any) => {
  if (m.userData) {
    profileStore.openProfile(m.userData.id, m.userData);
  }
};
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-surface-hover rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-primary/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <header class="mb-8 flex justify-between items-end shrink-0 z-10">
      <div>
        <h1 class="text-3xl font-extrabold text-text tracking-tight flex items-center gap-3">
          <span class="inline-flex items-center justify-center p-2 bg-primary/10 rounded-2xl shadow-sm border-primary">
            <Shield class="w-6 h-6 text-primary" />
          </span>
          {{ t('moderation.title') }}
        </h1>
        <p class="text-text-muted font-medium mt-2 text-sm ml-1">
          {{ t('moderation.subtitle') }}
        </p>
      </div>
      
      <div class="flex gap-2 bg-surface backdrop-blur-md p-1.5 rounded-2xl shadow-sm border-border-soft/60">
        <button
          :class="activeTab === 'blocked' ? 'bg-red-500 text-white shadow-md shadow-red-500/30' : 'text-text-muted hover:bg-surface hover:text-red-500'"
          class="px-4 py-2.5 rounded-xl font-bold text-sm transition-all flex items-center gap-2"
          @click="activeTab = 'blocked'"
        >
          <UserX :size="16" /> {{ t('moderation.tab_blocked') }}
        </button>
        <button
          :class="activeTab === 'muted' ? 'bg-orange-500 text-white shadow-md shadow-orange-500/30' : 'text-text-muted hover:bg-surface hover:text-orange-500'"
          class="px-4 py-2.5 rounded-xl font-bold text-sm transition-all flex items-center gap-2"
          @click="activeTab = 'muted'"
        >
          <VolumeX :size="16" /> {{ t('moderation.tab_muted') }}
        </button>
        <button
          :class="activeTab === 'hidden' ? 'bg-primary text-white shadow-md shadow-purple-500/30' : 'text-text-muted hover:bg-surface hover:text-primary'"
          class="px-4 py-2.5 rounded-xl font-bold text-sm transition-all flex items-center gap-2"
          @click="activeTab = 'hidden'"
        >
          <EyeOff :size="16" /> {{ t('moderation.tab_hidden') }}
        </button>
      </div>
    </header>

    <div class="flex-1 flex flex-col overflow-hidden z-10 relative">
      <!-- Search Bar -->
      <div class="relative mb-6 shrink-0 w-full max-w-md">
        <Search class="absolute left-4 top-1/2 -translate-y-1/2 text-border-strong w-5 h-5" />
        <input
          v-model="searchQuery"
          type="text"
          :placeholder="t('moderation.search_placeholder')"
          class="w-full pl-12 pr-4 py-3 bg-surface backdrop-blur-xl border-border-strong shadow-sm hover:shadow-md focus:shadow-md  focus:ring-4 focus:ring-indigo-500/10 rounded-2xl outline-none transition-all text-text text-sm font-bold placeholder:text-border-strong placeholder:font-medium"
        >
      </div>

      <div class="flex-1 overflow-y-auto custom-scrollbar pr-2">
        <div
          v-if="filteredModerations.length === 0"
          class="h-full flex flex-col items-center justify-center text-border-strong"
        >
          <ShieldAlert class="w-16 h-16 mb-4 opacity-30 text-border-strong" />
          <p class="font-extrabold text-lg text-text-muted">
            {{ t('moderation.empty') }}
          </p>
          <p class="text-sm mt-2 text-text-muted font-medium">
            {{ t('moderation.empty_desc') }}
          </p>
        </div>

        <div
          v-else
          class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-5 pb-4"
        >
          <div
            v-for="item in filteredModerations"
            :key="item.id"
            class="relative group bg-surface backdrop-blur-md rounded-3xl border-border-soft hover:border-primary shadow-sm hover:shadow-lg transition-all overflow-hidden flex flex-col "
          >
            <div
              class="flex-1 p-3 cursor-pointer"
              @click="openPlayerProfile(item)"
            >
              <VrcResourceCard
                v-if="item.userData"
                type="avatar"
                :data="item.userData"
                :is-user="true"
                :minimal="true"
              />
              <div
                v-else
                class="flex items-center gap-4 p-3 rounded-2xl bg-surface-hover border-border-soft"
              >
                <div class="w-12 h-12 rounded-xl bg-surface/60 flex items-center justify-center text-text-muted font-black uppercase text-lg shadow-inner relative shrink-0">
                  <span v-if="!item.loadingData">{{ item.name.charAt(0) }}</span>
                  <div
                    v-else
                    class="animate-spin w-5 h-5 border-2 border-border-soft border-t-transparent rounded-full"
                  />
                </div>
                <div class="min-w-0">
                  <h3
                    class="font-extrabold text-base text-text truncate group-hover:text-primary transition-colors"
                    :title="item.name"
                  >
                    {{ item.name }}
                  </h3>
                  <p class="text-[10px] text-border-strong font-mono font-bold mt-1 truncate">
                    {{ item.targetUserId }}
                  </p>
                </div>
              </div>
            </div>
             
            <!-- Action Bar -->
            <div class="px-5 py-4 bg-surface-hover backdrop-blur border-border-soft flex items-center justify-between">
              <div class="flex items-center gap-2.5">
                <div 
                  class="w-7 h-7 rounded-lg flex items-center justify-center shadow-sm border"
                  :class="{
                    'bg-red-500/10 text-red-500 border-red-500/20': item.type === 'blocked',
                    'bg-orange-500/10 text-orange-500 border-orange-500/20': item.type === 'muted',
                    'bg-primary/10 text-primary border-primary/20': item.type === 'hidden',
                  }"
                >
                  <UserX
                    v-if="item.type==='blocked'"
                    :size="14"
                  />
                  <VolumeX
                    v-else-if="item.type==='muted'"
                    :size="14"
                  />
                  <EyeOff
                    v-else
                    :size="14"
                  />
                </div>
                <span class="text-[11px] font-bold text-text-muted">{{ item.date }}</span>
              </div>
                
              <button
                class="px-4 py-2 bg-surface border-border-soft hover:bg-red-500 hover:border-red-500 hover:text-white hover:shadow-md text-text-muted rounded-xl font-bold transition-all flex items-center gap-2 text-xs shadow-sm active:scale-95"
                title="Remove Moderation"
                @click="unblock(item.id)"
              >
                <Unlock :size="14" /> {{ t('moderation.unblock') }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>


