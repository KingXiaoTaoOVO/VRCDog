<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { VrcApi, DbApi, SysApi, GamelogApi } from "../api";
import { Users, Loader2, Shield, Search, Check, UsersRound } from 'lucide-vue-next';
import VrcAvatar from './VrcAvatar.vue';
import VrcResourceCard from './VrcResourceCard.vue';
import BaseModal from './BaseModal.vue';
import { useI18n } from 'vue-i18n';
import type { VrcGroup } from '../types/vrc';

const { t } = useI18n();

const groups = ref<VrcGroup[]>([]);
const loading = ref(true);
const errorMsg = ref('');
const selectedGroup = ref<any>(null);
const loadingGroup = ref(false);
const searchQuery = ref('');

const fetchGroups = async () => {
  loading.value = true;
  errorMsg.value = '';
  try {
    const res: any = await VrcApi.getGroups();
    const list = Array.isArray(res) ? res : [];
    groups.value = list.map(g => g.group || g);
  } catch (err: any) {
    errorMsg.value = err.message || err;
  } finally {
    loading.value = false;
  }
};

const openGroupDetail = async (group: any) => {
  const groupId = group.groupId || group.id;
  loadingGroup.value = true;
  selectedGroup.value = null;
  try {
    const fetchedGroup: any = await VrcApi.getGroup({ groupId: groupId });
    selectedGroup.value = fetchedGroup;
  } catch (err: any) {
    errorMsg.value = err.message || err;
  } finally {
    loadingGroup.value = false;
  }
};

const filteredGroups = computed(() => {
  if (!searchQuery.value) return groups.value;
  const lower = searchQuery.value.toLowerCase();
  return groups.value.filter(g => 
    g.name.toLowerCase().includes(lower) || 
    (g.shortCode && g.shortCode.toLowerCase().includes(lower))
  );
});

onMounted(() => {
  fetchGroups();
});
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-slate-50/50 rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-indigo-500/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <div class="flex flex-col sm:flex-row sm:items-center justify-between mb-8 gap-4 shrink-0 z-10">
      <h1 class="text-3xl font-extrabold text-slate-900 tracking-tight flex items-center gap-3">
        <span class="inline-flex items-center justify-center p-2 bg-indigo-100 rounded-2xl shadow-sm border border-indigo-200/50">
          <Users class="w-6 h-6 text-indigo-600" />
        </span>
        {{ t('groups.title') }}
      </h1>
      <div class="flex items-center gap-2">
        <div class="relative">
          <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
            <Search class="h-4 w-4 text-slate-400" />
          </div>
          <input
            v-model="searchQuery"
            type="text"
            class="block w-64 pl-10 pr-4 py-2 bg-white border border-slate-200 shadow-sm rounded-xl text-slate-900 placeholder-slate-400 focus:outline-none focus:border-indigo-400 focus:ring-2 focus:ring-indigo-500/10 text-sm font-bold transition-all"
            :placeholder="t('groups.search_placeholder')"
          >
        </div>
        <button
          :disabled="loading"
          class="p-2.5 rounded-xl bg-white border border-slate-200 shadow-sm text-slate-600 hover:text-indigo-600 hover:border-indigo-200 transition-all disabled:opacity-50"
          @click="fetchGroups"
        >
          <Loader2
            v-if="loading"
            class="animate-spin"
            :size="20"
          />
          <Users
            v-else
            :size="20"
          />
        </button>
      </div>
    </div>

    <div
      v-if="errorMsg"
      class="bg-red-50 text-red-600 p-3 rounded-xl border border-red-200 text-sm font-bold mb-4 z-10"
    >
      {{ errorMsg }}
    </div>

    <div class="flex-1 overflow-y-auto pr-2 custom-scrollbar z-10 relative">
      <div
        v-if="loading && groups.length === 0"
        class="absolute inset-0 flex flex-col items-center justify-center text-indigo-500/80 bg-slate-50/50 backdrop-blur-sm z-10"
      >
        <Loader2
          class="animate-spin mb-4"
          :size="48"
        />
        <span class="font-extrabold text-lg tracking-wide">{{ t('groups.loading') }}</span>
      </div>

      <div
        v-else-if="groups.length === 0"
        class="h-full flex flex-col items-center justify-center text-slate-400"
      >
        <Users
          class="mb-4 opacity-30"
          :size="64"
        />
        <p class="font-bold text-xl text-slate-500">
          {{ t('groups.no_groups') }}
        </p>
      </div>

      <div
        v-else
        class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-5 pb-10"
      >
        <VrcResourceCard
          v-for="group in filteredGroups"
          :key="group.groupId || group.id"
          type="group"
          :data="group"
          @click="openGroupDetail(group)"
        />
      </div>
    </div>

    <!-- 群组详情弹窗 -->
    <BaseModal
      :show="!!selectedGroup"
      :loading="loadingGroup"
      @close="selectedGroup = null"
    >
      <template v-if="selectedGroup">
        <div class="h-32 bg-slate-900 relative overflow-hidden rounded-t-2xl">
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
              <h2 class="text-xl font-black text-slate-900 truncate">
                {{ selectedGroup.name }}
              </h2>
              <div class="flex items-center gap-2 mt-1">
                <span class="text-xs font-bold text-slate-500 uppercase">{{ selectedGroup.shortCode }}</span>
                <span class="w-1 h-1 rounded-full bg-slate-300" />
                <span class="text-xs font-bold text-indigo-600 flex items-center gap-1"><UsersRound :size="12" /> {{ selectedGroup.memberCount || 0 }} {{ t('global.groups.members') }}</span>
              </div>
            </div>
          </div>
          
          <div class="mb-5">
            <p class="text-sm text-slate-700 leading-relaxed whitespace-pre-wrap max-h-48 overflow-y-auto custom-scrollbar">
              {{ selectedGroup.description || t('global.groups.no_desc') }}
            </p>
          </div>

          <div class="grid grid-cols-2 gap-3 mb-5">
            <div class="bg-slate-50 border border-slate-100 rounded-xl p-3 flex flex-col justify-center">
              <p class="text-[10px] text-slate-400 font-bold uppercase tracking-wider mb-1">
                隐私状态
              </p>
              <p class="text-sm font-black text-slate-800 flex items-center gap-1">
                <Shield
                  :size="14"
                  class="text-blue-500"
                /> {{ selectedGroup.privacy === 'public' ? t('global.groups.public') : t('global.groups.private') }}
              </p>
            </div>
            <div class="bg-slate-50 border border-slate-100 rounded-xl p-3 flex flex-col justify-center">
              <p class="text-[10px] text-slate-400 font-bold uppercase tracking-wider mb-1">
                加入方式
              </p>
              <p class="text-sm font-black text-slate-800 flex items-center gap-1">
                <Check
                  :size="14"
                  class="text-green-500"
                /> {{ selectedGroup.joinState === 'open' ? '自由加入' : (selectedGroup.joinState === 'request' ? '需申请' : '邀请制') }}
              </p>
            </div>
          </div>
          
          <div class="pt-4 border-t border-slate-100 flex items-center justify-between">
            <div class="text-xs text-slate-400 font-mono">
              {{ selectedGroup.id }}
            </div>
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
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: #cbd5e1; border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: #94a3b8; }
</style>
