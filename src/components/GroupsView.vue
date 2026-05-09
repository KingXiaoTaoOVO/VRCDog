<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { VrcApi, DbApi, SysApi, GamelogApi } from "../api";
import { Users, Loader2, Shield, Search } from 'lucide-vue-next';
import VrcAvatar from './VrcAvatar.vue';
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
    groups.value = Array.isArray(res) ? res : [];
  } catch (err: any) {
    errorMsg.value = err.message || err;
  } finally {
    loading.value = false;
  }
};

const openGroupDetail = async (groupId: string) => {
  loadingGroup.value = true;
  selectedGroup.value = null;
  try {
    const group: any = await VrcApi.getGroup({ groupId: groupId });
    selectedGroup.value = group;
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
  <div class="h-full flex flex-col">
    <div class="flex flex-col sm:flex-row sm:items-center justify-between mb-4 gap-4">
      <h1 class="text-2xl font-extrabold text-[#451a03] tracking-tight flex items-center gap-2">
        <Users
          class="text-indigo-500"
          :size="28"
        /> {{ t('groups.title') }}
      </h1>
      <div class="flex items-center gap-2">
        <div class="relative">
          <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
            <Search class="h-4 w-4 text-indigo-400" />
          </div>
          <input
            v-model="searchQuery"
            type="text"
            class="block w-64 pl-10 pr-4 py-2 bg-white/80 backdrop-blur border border-indigo-200 rounded-xl text-indigo-900 placeholder-indigo-400 focus:outline-none focus:border-indigo-400 text-sm transition-colors"
            :placeholder="t('groups.search_placeholder')"
          >
        </div>
        <button
          :disabled="loading"
          class="p-2 rounded-xl bg-indigo-50 text-indigo-600 hover:bg-indigo-100 transition-colors disabled:opacity-50"
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
      class="bg-red-50 text-red-600 p-3 rounded-xl border border-red-200 text-sm font-bold mb-4"
    >
      {{ errorMsg }}
    </div>

    <div class="flex-1 overflow-y-auto pr-1">
      <div
        v-if="loading && groups.length === 0"
        class="flex items-center justify-center py-12 text-indigo-500 font-bold"
      >
        <Loader2
          class="animate-spin mr-2"
          :size="24"
        /> {{ t('groups.loading') }}
      </div>

      <div
        v-else-if="groups.length === 0"
        class="text-center text-indigo-500 py-12 text-sm bg-white/50 backdrop-blur rounded-2xl border-2 border-dashed border-indigo-200 font-bold"
      >
        <Users
          class="mx-auto mb-3 opacity-50"
          :size="48"
        />
        {{ t('groups.no_groups') }} 🐕
      </div>

      <div
        v-else
        class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4"
      >
        <div
          v-for="group in filteredGroups"
          :key="group.groupId || group.id"
          class="bg-white/80 backdrop-blur rounded-2xl overflow-hidden border border-indigo-100 hover:border-indigo-300 transition-all shadow-sm hover:shadow-md cursor-pointer group flex flex-col"
          @click="openGroupDetail(group.groupId || group.id)"
        >
          <div class="h-24 bg-indigo-50 relative overflow-hidden flex-shrink-0">
            <VrcAvatar
              :user="group"
              :url="group.bannerUrl"
              custom-class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-500 opacity-80"
            />
          </div>
          <div class="p-4 flex gap-3 -mt-8 relative flex-1">
            <VrcAvatar
              :user="group"
              :url="group.iconUrl"
              custom-class="w-14 h-14 rounded-xl object-cover border-4 border-white shadow-sm bg-indigo-50 flex-shrink-0"
            />
            <div class="flex-1 min-w-0 pt-6">
              <h3 class="font-bold text-indigo-900 text-sm truncate">
                {{ group.name }}
              </h3>
              <p class="text-[10px] text-indigo-600 truncate mt-1">
                {{ group.shortCode }}
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 群组详情弹窗 -->
    <Teleport to="body">
      <Transition name="fade">
        <div
          v-if="selectedGroup || loadingGroup"
          class="fixed inset-0 z-50 flex items-center justify-center p-4"
        >
          <div
            class="absolute inset-0 bg-black/30 backdrop-blur-sm"
            @click="selectedGroup = null"
          />
          <div class="bg-white/95 backdrop-blur-xl w-full max-w-lg rounded-3xl shadow-2xl relative z-10 overflow-hidden border border-indigo-100 flex flex-col max-h-[90vh]">
            <div
              v-if="loadingGroup"
              class="p-12 text-center"
            >
              <Loader2
                class="animate-spin mx-auto text-indigo-500"
                :size="32"
              />
            </div>
            <template v-else-if="selectedGroup">
              <div class="h-40 bg-indigo-50 relative overflow-hidden flex-shrink-0">
                <VrcAvatar
                  :user="selectedGroup"
                  :url="selectedGroup.bannerUrl"
                  custom-class="w-full h-full object-cover opacity-80"
                />
                <button
                  class="absolute top-3 right-3 p-1.5 rounded-full bg-black/20 hover:bg-black/40 text-white backdrop-blur"
                  @click="selectedGroup = null"
                >
                  ✕
                </button>
              </div>
              <div class="p-6 -mt-10 relative flex-1 overflow-y-auto">
                <div class="flex items-end gap-4 mb-4">
                  <VrcAvatar
                    :user="selectedGroup"
                    :url="selectedGroup.iconUrl"
                    custom-class="w-20 h-20 rounded-2xl object-cover border-4 border-white shadow-lg bg-indigo-50"
                  />
                  <div class="flex-1 min-w-0 pb-1">
                    <h2 class="text-xl font-extrabold text-[#451a03] truncate">
                      {{ selectedGroup.name }}
                    </h2>
                    <p class="text-sm font-bold text-indigo-700">
                      {{ selectedGroup.shortCode }}
                    </p>
                  </div>
                </div>
                
                <p class="text-sm text-gray-700 mb-6 leading-relaxed whitespace-pre-wrap">
                  {{ selectedGroup.description }}
                </p>
                
                <div class="grid grid-cols-2 gap-3">
                  <div class="bg-indigo-50 rounded-xl p-3 flex items-center gap-2">
                    <Users
                      class="text-indigo-500"
                      :size="16"
                    />
                    <div>
                      <p class="text-lg font-extrabold text-indigo-700">
                        {{ selectedGroup.memberCount || 0 }}
                      </p>
                      <p class="text-[10px] text-indigo-600 font-bold">
                        {{ t('groups.member_count') }}
                      </p>
                    </div>
                  </div>
                  <div class="bg-indigo-50 rounded-xl p-3 flex items-center gap-2">
                    <Shield
                      class="text-indigo-500"
                      :size="16"
                    />
                    <div>
                      <p class="text-lg font-extrabold text-indigo-700">
                        {{ selectedGroup.privacy || 'Public' }}
                      </p>
                      <p class="text-[10px] text-indigo-600 font-bold">
                        {{ t('groups.privacy') }}
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </template>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.fade-enter-active, .fade-leave-active { transition: opacity 0.3s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>
