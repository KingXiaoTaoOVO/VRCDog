<script setup lang="ts">
import { ref, onMounted, computed, onUnmounted } from 'vue';
import { VrcApi, DbApi, SysApi, GamelogApi } from "../api";
import { RefreshCcw, Search, MapPin, Bone, X, Save, StickyNote } from 'lucide-vue-next';
import VrcAvatar from './VrcAvatar.vue';
import BaseModal from './BaseModal.vue';
import type { VrcUser } from '../types/vrc';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const onlineFriends = ref<VrcUser[]>([]);
const offlineFriends = ref<VrcUser[]>([]);
const loading = ref(true);
const errorMsg = ref('');
const searchQuery = ref('');

// 好友详情弹窗
const selectedFriend = ref<VrcUser | null>(null);
const friendNote = ref('');
const savingNote = ref(false);

const fetchFriends = async () => {
  loading.value = true;
  errorMsg.value = '';
  try {
    const cached: any[] = await DbApi.getCachedFriends() || [];
    onlineFriends.value = cached.filter((f: any) => f.location && f.location !== 'offline');
    offlineFriends.value = cached.filter((f: any) => !f.location || f.location === 'offline');
  } catch (err: any) {
    errorMsg.value = err.message || err;
  } finally {
    loading.value = false;
  }
};

onMounted(() => {
  fetchFriends();
  window.addEventListener('vrc-friends-synced', fetchFriends);
  window.addEventListener('vrc-pipeline-event', fetchFriends);
});

onUnmounted(() => {
  window.removeEventListener('vrc-friends-synced', fetchFriends);
  window.removeEventListener('vrc-pipeline-event', fetchFriends);
});

const allFriends = computed(() => [...onlineFriends.value, ...offlineFriends.value]);
const onlineCount = computed(() => onlineFriends.value.length);
const offlineCount = computed(() => offlineFriends.value.length);
const totalCount = computed(() => allFriends.value.length);

const filteredFriends = computed(() => {
  const q = searchQuery.value.toLowerCase();
  if (!q) return allFriends.value;
  return allFriends.value.filter((f: any) =>
    f.displayName?.toLowerCase().includes(q)
  );
});

const getStatusColor = (status?: string) => {
  switch (status) {
    case 'active': return 'bg-green-500';
    case 'join me': return 'bg-blue-500';
    case 'ask me': return 'bg-orange-500';
    case 'busy': return 'bg-red-500';
    default: return 'bg-gray-400';
  }
};

const getStatusLabel = (status?: string) => {
  switch (status) {
    case 'active': return t('status.online');
    case 'join me': return t('status.join_me');
    case 'ask me': return t('status.ask_me');
    case 'busy': return t('status.busy');
    default: return t('status.offline');
  }
};

const getStatusText = (friend: any) => {
  if (friend.location === 'offline') return t('status.offline');
  if (friend.location === 'private') return t('friends_list.private_instance');
  return friend.statusDescription || getStatusLabel(friend.status);
};

const openDetail = async (friend: any) => {
  selectedFriend.value = friend;
  friendNote.value = '';
  try {
    const note: any = await DbApi.getNote({ userId: friend.id });
    if (note) friendNote.value = note.note || '';
  } catch { /* ignore */ }
};

const closeDetail = () => { selectedFriend.value = null; };

const saveNote = async () => {
  if (!selectedFriend.value) return;
  savingNote.value = true;
  try {
    await DbApi.saveNote({
      userId: selectedFriend.value.id,
      displayName: selectedFriend.value.displayName,
      note: friendNote.value,
    });
  } catch { /* ignore */ }
  savingNote.value = false;
};

const isUnfriending = ref(false);
const actionMessage = ref('');
const actionError = ref('');

const unfriend = async () => {
  if (!selectedFriend.value) return;
  if (!confirm(t('friends_list.confirm_unfriend', { name: selectedFriend.value.displayName }))) return;
  isUnfriending.value = true;
  actionMessage.value = '';
  actionError.value = '';
  try {
    await VrcApi.unfriend({ userId: selectedFriend.value.id });
    actionMessage.value = t('friends_list.unfriend_success');
    setTimeout(() => { closeDetail(); fetchFriends(); }, 1500);
  } catch (err: any) {
    actionError.value = `${t('friends_list.unfriend_failed')}: ${err.message || err}`;
    setTimeout(() => { actionError.value = ''; }, 3000);
  } finally {
    isUnfriending.value = false;
  }
};

const isInviting = ref(false);
const joinInstance = async () => {
  if(!selectedFriend.value) return;
  const loc = selectedFriend.value.location;
  if (!loc || loc === 'offline' || loc === 'private' || loc === 'traveling') return;
  isInviting.value = true;
  actionMessage.value = '';
  actionError.value = '';
  try {
    const parts = loc.split(':');
    if (parts.length < 2) {
      actionError.value = t('friends_list.error_parse_location');
      setTimeout(() => { actionError.value = ''; }, 3000);
      isInviting.value = false;
      return;
    }
    const worldId = parts[0];
    const instanceId = loc.substring(worldId.length + 1);
    await VrcApi.inviteMyself({ worldId, instanceId });
    actionMessage.value = t('friends_list.invite_sent');
    setTimeout(() => { actionMessage.value = ''; }, 3000);
    // 异步记录这个追踪活动
    await DbApi.recordActivity({ userId: selectedFriend.value.id, displayName: selectedFriend.value.displayName, status: 'join_instance', location: loc });
  } catch (err: any) {
    actionError.value = `${t('friends_list.invite_failed')}: ${err.message || err}`;
    setTimeout(() => { actionError.value = ''; }, 3000);
  } finally {
    isInviting.value = false;
  }
};
</script>

<template>
  <div class="h-full flex flex-col">
    <div class="flex items-center justify-between mb-6">
      <h1 class="text-2xl font-extrabold text-[#451a03] tracking-tight">
        {{ t('friends_list.title') }}
      </h1>
      <div class="flex items-center gap-3">
        <span class="text-xs font-bold px-3 py-1 rounded-full bg-green-100 text-green-700">🟢 {{ onlineCount }}</span>
        <span class="text-xs font-bold px-3 py-1 rounded-full bg-gray-100 text-gray-600">⚫ {{ offlineCount }}</span>
        <span class="text-xs font-bold px-3 py-1 rounded-full bg-amber-100 text-amber-700">{{ t('friends_list.total_count', { count: totalCount }) }}</span>
        <button
          class="p-2 rounded-full bg-white hover:bg-amber-50 text-amber-700 shadow-sm border border-amber-100 transition-colors"
          @click="fetchFriends"
        >
          <RefreshCcw
            class="w-4 h-4"
            :class="{'animate-spin': loading}"
          />
        </button>
      </div>
    </div>

    <!-- 搜索框 -->
    <div class="relative mb-4">
      <div class="absolute inset-y-0 left-0 pl-4 flex items-center pointer-events-none">
        <Search class="h-4 w-4 text-amber-400" />
      </div>
      <input
        v-model="searchQuery"
        type="text"
        class="block w-full pl-10 pr-4 py-2.5 bg-white/80 backdrop-blur border border-amber-100 rounded-xl text-amber-900 placeholder-amber-400 focus:outline-none focus:border-amber-300 text-sm"
        :placeholder="t('friends_list.search_placeholder')"
      >
    </div>

    <!-- 加载中 -->
    <div
      v-if="loading && allFriends.length === 0"
      class="flex-1 flex flex-col items-center justify-center text-amber-500 opacity-70"
    >
      <Bone
        class="animate-bounce mb-4"
        :size="48"
      />
      <p class="font-bold">
        {{ t('friends_list.loading') }}
      </p>
    </div>

    <!-- 错误 -->
    <div
      v-else-if="errorMsg"
      class="bg-red-50 text-red-600 p-4 rounded-2xl border border-red-200 text-center font-bold text-sm"
    >
      {{ errorMsg }}
    </div>

    <!-- 好友列表 -->
    <div
      v-else
      class="flex-1 overflow-y-auto pr-1 custom-scrollbar"
    >
      <div
        v-if="filteredFriends.length === 0"
        class="text-center text-amber-500 py-12 text-sm"
      >
        {{ t('friends_list.no_results') }}
      </div>
      <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3">
        <div
          v-for="friend in filteredFriends"
          :key="friend.id"
          class="bg-white/80 backdrop-blur rounded-xl p-3 border border-amber-50 hover:border-amber-300 transition-all shadow-sm hover:shadow-md flex items-center gap-3 cursor-pointer group"
          @click="openDetail(friend)"
        >
          <div class="relative flex-shrink-0">
            <VrcAvatar
              :user="friend"
              custom-class="w-12 h-12 rounded-xl object-cover bg-amber-50 border border-amber-100 group-hover:border-amber-300 transition-colors"
            />
            <div
              class="absolute -bottom-0.5 -right-0.5 w-3.5 h-3.5 rounded-full border-2 border-white"
              :class="getStatusColor(friend.status || 'offline')"
            />
          </div>
          <div class="flex-1 min-w-0">
            <h3 class="font-bold text-amber-900 truncate text-sm">
              {{ friend.displayName }}
            </h3>
            <p class="text-[11px] text-amber-600 truncate mt-0.5">
              {{ getStatusText(friend) }}
            </p>
            <p
              v-if="friend.location && friend.location !== 'offline' && friend.location !== 'private'"
              class="text-[10px] text-amber-500 mt-0.5 truncate flex items-center gap-0.5"
            >
              <MapPin :size="9" /> {{ t('friends_list.public_world') }}
            </p>
          </div>
        </div>
      </div>
    </div>

    <!-- 好友详情弹窗 -->
    <BaseModal
      :show="!!selectedFriend"
      @close="closeDetail"
    >
      <template v-if="selectedFriend">
        <!-- 头图 -->
        <div class="h-40 bg-gradient-to-br from-amber-100 to-orange-100 relative overflow-hidden">
          <VrcAvatar
            :user="selectedFriend"
            :url="selectedFriend.currentAvatarImageUrl || selectedFriend.currentAvatarThumbnailImageUrl"
            custom-class="w-full h-full object-cover opacity-80"
          />
          <button
            class="absolute top-3 right-3 p-1.5 rounded-full bg-black/20 hover:bg-black/40 text-white backdrop-blur transition-colors"
            @click="closeDetail"
          >
            <X :size="18" />
          </button>
        </div>

        <div class="p-6 -mt-8 relative">
          <!-- 头像 -->
          <div class="flex items-end gap-4 mb-4">
            <VrcAvatar
              :user="selectedFriend"
              custom-class="w-20 h-20 rounded-2xl object-cover border-4 border-white shadow-lg bg-amber-50"
            />
            <div class="flex-1 min-w-0 pb-1">
              <h2 class="text-xl font-extrabold text-[#451a03] truncate">
                {{ selectedFriend.displayName }}
              </h2>
              <div class="flex items-center gap-2 mt-1">
                <span
                  class="w-2.5 h-2.5 rounded-full"
                  :class="getStatusColor(selectedFriend.status)"
                />
                <span
                  class="text-sm font-bold"
                  :class="{
                    'text-green-600': selectedFriend.status === 'active',
                    'text-blue-600': selectedFriend.status === 'join me',
                    'text-orange-600': selectedFriend.status === 'ask me',
                    'text-red-600': selectedFriend.status === 'busy',
                    'text-gray-500': !selectedFriend.status || selectedFriend.status === 'offline',
                  }"
                >{{ getStatusLabel(selectedFriend.status) }}</span>
              </div>
            </div>
          </div>

          <!-- 状态描述 -->
          <div
            v-if="selectedFriend.statusDescription"
            class="bg-amber-50 rounded-xl p-3 mb-4 text-sm text-amber-800 italic"
          >
            "{{ selectedFriend.statusDescription }}"
          </div>

          <!-- 信息卡片 -->
          <div class="grid grid-cols-2 gap-3 mb-4">
            <div class="bg-gray-50 rounded-xl p-3">
              <p class="text-[10px] text-gray-500 uppercase font-bold mb-1">
                {{ t('friends_list.location') }}
              </p>
              <p class="text-sm font-bold text-gray-800 truncate">
                {{ selectedFriend.location === 'offline' ? t('status.offline') : selectedFriend.location === 'private' ? t('friends_list.private_instance') : selectedFriend.location === 'traveling' ? t('friends_list.traveling') : t('friends_list.public_world') }}
              </p>
            </div>
            <div class="bg-gray-50 rounded-xl p-3">
              <p class="text-[10px] text-gray-500 uppercase font-bold mb-1">
                {{ t('friends_list.last_login') }}
              </p>
              <p class="text-sm font-bold text-gray-800 truncate">
                {{ selectedFriend.last_login?.slice(0, 10) || t('friends_list.unknown') }}
              </p>
            </div>
          </div>

          <!-- Bio -->
          <div
            v-if="selectedFriend.bio"
            class="mb-4"
          >
            <p class="text-[10px] text-gray-500 uppercase font-bold mb-1">
              {{ t('friends_list.bio') }}
            </p>
            <p class="text-sm text-gray-700 leading-relaxed line-clamp-4">
              {{ selectedFriend.bio }}
            </p>
          </div>

          <!-- 备忘录 -->
          <div class="border-t border-amber-100 pt-4">
            <p class="text-xs font-bold text-amber-900 mb-2 flex items-center gap-1">
              <StickyNote :size="14" /> {{ t('friends_list.private_note') }}
            </p>
            <textarea
              v-model="friendNote"
              class="w-full px-3 py-2 rounded-xl border border-amber-200 focus:border-amber-400 focus:ring-0 outline-none bg-amber-50/30 text-sm resize-none"
              rows="2"
              :placeholder="t('friends_list.note_placeholder')"
            />
            <button
              :disabled="savingNote"
              class="mt-2 bg-amber-500 hover:bg-amber-600 text-white font-bold text-xs px-4 py-1.5 rounded-lg flex items-center gap-1 transition-colors disabled:opacity-50"
              @click="saveNote"
            >
              <Save :size="12" /> {{ savingNote ? t('friends_list.saving') : t('friends_list.save_note') }}
            </button>
          </div>
              
          <!-- 快捷操作区 -->
          <div class="mt-4 pt-4 border-t border-amber-100 flex flex-col gap-2">
            <div
              v-if="actionMessage"
              class="text-green-600 bg-green-50 px-3 py-2 rounded-xl text-xs font-bold text-center border border-green-200"
            >
              {{ actionMessage }}
            </div>
            <div
              v-if="actionError"
              class="text-red-600 bg-red-50 px-3 py-2 rounded-xl text-xs font-bold text-center border border-red-200"
            >
              {{ actionError }}
            </div>
            <div class="flex items-center gap-2">
              <button 
                v-if="selectedFriend.location && selectedFriend.location !== 'offline' && selectedFriend.location !== 'private' && selectedFriend.location !== 'traveling'"
                :disabled="isInviting || !!actionMessage"
                class="flex-1 bg-blue-500 hover:bg-blue-600 text-white font-bold py-2 rounded-xl flex justify-center items-center gap-1 text-sm transition-colors disabled:opacity-50"
                @click="joinInstance"
              >
                <MapPin :size="16" /> {{ isInviting ? t('friends_list.sending_request') : t('friends_list.request_invite') }}
              </button>
              <button 
                :disabled="isUnfriending || !!actionMessage"
                class="px-4 py-2 bg-red-50 hover:bg-red-500 text-red-500 hover:text-white font-bold rounded-xl text-sm transition-colors disabled:opacity-50 flex items-center gap-1"
                @click="unfriend"
              >
                <X :size="16" /> {{ t('friends_list.unfriend') }}
              </button>
            </div>
          </div>
        </div>
      </template>
    </BaseModal>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 5px; }
.custom-scrollbar::-webkit-scrollbar-track { background: rgba(254,243,199,0.5); border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(252,211,77,0.8); border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(251,191,36,1); }
.fade-enter-active, .fade-leave-active { transition: opacity 0.3s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>
