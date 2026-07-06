<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue';
import { VrcApi, SysApi } from "../api";
import { Users, Loader2, Shield, Search, Check, UsersRound, Settings, ScrollText, Megaphone, ShieldAlert, FileText, UserX, Ban } from 'lucide-vue-next';
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

// Tabs state
const activeTab = ref('info');
const groupMembers = ref<any[]>([]);
const groupRoles = ref<any[]>([]);
const groupPosts = ref<any[]>([]);
const groupLogs = ref<any[]>([]);
const groupJoinRequests = ref<any[]>([]);
const groupPermissions = ref<string[]>([]);
const loadingTab = ref(false);
const actionBusy = ref('');

const hasGroupPermission = (permission: string) =>
  groupPermissions.value.includes('*') || groupPermissions.value.includes(permission);

const tabs = computed(() => [
  { id: 'info', name: t('groups.tabs.info'), icon: Shield },
  { id: 'members', name: t('groups.tabs.members'), icon: UsersRound },
  { id: 'roles', name: t('groups.tabs.roles'), icon: Settings },
  { id: 'posts', name: t('groups.tabs.posts'), icon: Megaphone },
  { id: 'logs', name: t('groups.tabs.logs'), icon: ScrollText },
  ...(hasGroupPermission('group-join-requests-manage')
    ? [{ id: 'requests', name: t('entity_modal.requests'), icon: UserX }]
    : [])
]);

const fetchGroupPermissions = async (groupId: string) => {
  groupPermissions.value = [];
  try {
    const res: any = await VrcApi.getUserGroupPermissions({ userId: 'me' });
    if (Array.isArray(res)) {
      const current = res.find((entry: any) => entry.groupId === groupId || entry.id === groupId);
      groupPermissions.value = current?.permissions || [];
    } else if (res && Array.isArray(res[groupId])) {
      groupPermissions.value = res[groupId];
    }
  } catch (err) {
    console.warn('Failed to load group permissions', err);
  }
};

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
  activeTab.value = 'info';
  
  // reset state
  groupMembers.value = [];
  groupRoles.value = [];
  groupPosts.value = [];
  groupLogs.value = [];
  groupJoinRequests.value = [];
  groupPermissions.value = [];
  
  try {
    const fetchedGroup: any = await VrcApi.getGroup({ groupId: groupId, includeRoles: true });
    selectedGroup.value = fetchedGroup;
    await fetchGroupPermissions(groupId);
  } catch (err: any) {
    errorMsg.value = err.message || err;
  } finally {
    loadingGroup.value = false;
  }
};

const fetchTabContent = async () => {
  if (!selectedGroup.value) return;
  const groupId = selectedGroup.value.id;
  loadingTab.value = true;
  try {
    if (activeTab.value === 'members' && groupMembers.value.length === 0) {
      const res: any = await VrcApi.getGroupMembers({ groupId });
      groupMembers.value = Array.isArray(res) ? res : [];
    } else if (activeTab.value === 'roles' && groupRoles.value.length === 0) {
      const res: any = await VrcApi.getGroupRoles({ groupId });
      groupRoles.value = Array.isArray(res) ? res : [];
    } else if (activeTab.value === 'posts' && groupPosts.value.length === 0) {
      const res: any = await VrcApi.getGroupPosts({ groupId });
      groupPosts.value = Array.isArray(res) ? res : [];
    } else if (activeTab.value === 'logs' && groupLogs.value.length === 0) {
      const res: any = await VrcApi.getGroupLogs({ groupId });
      groupLogs.value = Array.isArray(res) ? res : [];
    } else if (activeTab.value === 'requests' && groupJoinRequests.value.length === 0) {
      const res: any = await VrcApi.getGroupJoinRequests({ groupId });
      groupJoinRequests.value = Array.isArray(res) ? res : [];
    }
  } catch (err) {
    console.error("Failed to load tab content:", err);
  } finally {
    loadingTab.value = false;
  }
};

const respondJoinRequest = async (requestId: string, action: 'accept' | 'reject') => {
  if (!selectedGroup.value) return;
  actionBusy.value = `${action}:${requestId}`;
  try {
    await VrcApi.respondGroupJoinRequest({ groupId: selectedGroup.value.id, requestId, action });
    groupJoinRequests.value = groupJoinRequests.value.filter((req) => req.id !== requestId && req.userId !== requestId);
  } catch (err: any) {
    errorMsg.value = err.message || String(err);
  } finally {
    actionBusy.value = '';
  }
};

const kickMember = async (member: any) => {
  if (!selectedGroup.value) return;
  const userId = member.userId || member.user?.id || member.id;
  if (!userId) return;
  actionBusy.value = `kick:${userId}`;
  try {
    await VrcApi.kickGroupMember({ groupId: selectedGroup.value.id, userId });
    groupMembers.value = groupMembers.value.filter((entry) => (entry.userId || entry.user?.id || entry.id) !== userId);
  } catch (err: any) {
    errorMsg.value = err.message || String(err);
  } finally {
    actionBusy.value = '';
  }
};

const banMember = async (member: any) => {
  if (!selectedGroup.value) return;
  const userId = member.userId || member.user?.id || member.id;
  if (!userId) return;
  actionBusy.value = `ban:${userId}`;
  try {
    await VrcApi.banGroupMember({ groupId: selectedGroup.value.id, userId });
    groupMembers.value = groupMembers.value.filter((entry) => (entry.userId || entry.user?.id || entry.id) !== userId);
  } catch (err: any) {
    errorMsg.value = err.message || String(err);
  } finally {
    actionBusy.value = '';
  }
};

watch(activeTab, () => {
  fetchTabContent();
});

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
  <div class="h-full flex flex-col p-6 bg-surface-hover rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-primary/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <div class="flex flex-col sm:flex-row sm:items-center justify-between mb-8 gap-4 shrink-0 z-10">
      <h1 class="text-3xl font-extrabold text-text tracking-tight flex items-center gap-3">
        <span class="inline-flex items-center justify-center p-2 bg-primary/10 rounded-2xl shadow-sm border-primary">
          <Users class="w-6 h-6 text-primary" />
        </span>
        {{ t('groups.title') }}
      </h1>
      <div class="flex items-center gap-2">
        <div class="relative">
          <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
            <Search class="h-4 w-4 text-border-strong" />
          </div>
          <input
            v-model="searchQuery"
            type="text"
            class="block w-64 pl-10 pr-4 py-2 bg-surface border-border-soft shadow-sm rounded-xl text-text placeholder-slate-400 focus:outline-none   focus:ring-indigo-500/10 text-sm font-bold transition-all"
            :placeholder="t('groups.search_placeholder')"
          >
        </div>
        <button
          :disabled="loading"
          class="p-2.5 rounded-xl bg-surface border-border-soft shadow-sm text-text-muted hover:text-primary hover:border-primary transition-all disabled:opacity-50"
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
      class="bg-red-50 text-red-600 p-3 rounded-xl border-red-200 text-sm font-bold mb-4 z-10"
    >
      {{ errorMsg }}
    </div>

    <div class="flex-1 overflow-y-auto pr-2 custom-scrollbar z-10 relative">
      <div
        v-if="loading && groups.length === 0"
        class="absolute inset-0 flex flex-col items-center justify-center text-primary bg-surface-hover backdrop-blur-sm z-10"
      >
        <Loader2
          class="animate-spin mb-4"
          :size="48"
        />
        <span class="font-extrabold text-lg tracking-wide">{{ t('groups.loading') }}</span>
      </div>

      <div
        v-else-if="groups.length === 0"
        class="h-full flex flex-col items-center justify-center text-border-strong"
      >
        <Users
          class="mb-4 opacity-30"
          :size="64"
        />
        <p class="font-bold text-xl text-text-muted">
          {{ t('groups.no_groups') }}
        </p>
      </div>

      <div
        v-else
        class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-5 pb-10"
      >
        <VrcResourceCard
          v-for="(group, index) in filteredGroups"
          :key="group.id || index"
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
      custom-classes="w-full max-w-4xl h-[85vh] flex flex-col bg-surface shadow-2xl overflow-hidden relative"
      @close="selectedGroup = null"
    >
      <template v-if="selectedGroup">
        <!-- 头部背景图 -->
        <div class="h-40 bg-surface relative overflow-hidden rounded-t-2xl shrink-0">
          <VrcAvatar
            :user="selectedGroup"
            :url="selectedGroup.bannerUrl"
            custom-class="w-full h-full object-cover opacity-80"
          />
          <!-- 磨砂渐变遮罩 -->
          <div class="absolute inset-0 bg-gradient-to-t from-surface via-surface/40 to-transparent"></div>
          <button
            class="absolute top-4 right-4 p-2 rounded-full bg-surface/50 hover:bg-background/80 backdrop-blur-md text-text transition-colors z-20"
            @click="selectedGroup = null"
          >
            ✕
          </button>
        </div>

        <div class="px-6 relative flex flex-col flex-1 min-h-0 -mt-16 z-10">
          <!-- 头部信息 -->
          <div class="flex gap-5 mb-6 shrink-0 items-end">
            <div class="w-28 h-28 rounded-2xl border-4 border-surface shadow-xl bg-surface flex-shrink-0 relative overflow-hidden">
              <VrcAvatar
                :user="selectedGroup"
                :url="selectedGroup.iconUrl"
                custom-class="w-full h-full object-cover"
              />
            </div>
            <div class="flex-1 pb-1 min-w-0">
              <h2 class="text-3xl font-black text-text truncate tracking-tight drop-shadow-md">
                {{ selectedGroup.name }}
              </h2>
              <div class="flex items-center gap-3 mt-2">
                <span class="text-sm font-bold text-text-muted bg-surface-hover px-2 py-0.5 rounded-md uppercase">{{ selectedGroup.shortCode }}</span>
                <span class="text-sm font-bold text-primary flex items-center gap-1 bg-primary/10 px-2 py-0.5 rounded-md">
                  <UsersRound :size="14" /> {{ selectedGroup.memberCount || 0 }} {{ t('global.groups.members') }}
                </span>
                <span class="text-sm font-bold text-text-muted flex items-center gap-1 bg-surface-hover px-2 py-0.5 rounded-md">
                  <Shield :size="14" :class="selectedGroup.privacy === 'public' ? 'text-blue-500' : 'text-yellow-500'" />
                  {{ selectedGroup.privacy === 'public' ? t('global.groups.public') : t('global.groups.private') }}
                </span>
              </div>
            </div>
          </div>

          <!-- 导航 Tabs -->
          <div class="flex space-x-1 border-b border-border-soft shrink-0 mb-4 overflow-x-auto custom-scrollbar">
            <button
              v-for="tab in tabs"
              :key="tab.id"
              class="px-5 py-3 text-sm font-bold transition-all flex items-center gap-2 whitespace-nowrap border-b-2"
              :class="activeTab === tab.id ? 'border-primary text-primary' : 'border-transparent text-text-muted hover:text-text hover:border-border-strong'"
              @click="activeTab = tab.id"
            >
              <component :is="tab.icon" :size="16" />
              {{ tab.name }}
              <span
                v-if="tab.id === 'requests' && groupJoinRequests.length > 0"
                class="px-1.5 py-0.5 rounded-full bg-red-500 text-white text-[10px] leading-none"
              >
                {{ groupJoinRequests.length }}
              </span>
            </button>
          </div>
          
          <!-- 内容区 -->
          <div class="flex-1 overflow-y-auto custom-scrollbar pr-2 pb-6 relative min-h-0">
            <!-- 加载中 -->
            <div v-if="loadingTab" class="absolute inset-0 flex items-center justify-center text-primary bg-surface/50 z-10 backdrop-blur-sm rounded-xl">
              <Loader2 class="animate-spin" :size="32" />
            </div>

            <!-- Tab: 信息 -->
            <div v-if="activeTab === 'info'" class="space-y-6">
              <div class="bg-surface-hover rounded-xl p-5 border border-border-soft">
                <h3 class="text-lg font-bold text-text mb-2 flex items-center gap-2"><FileText :size="18" class="text-primary" /> {{ t('groups.tabs.info') }}</h3>
                <p class="text-sm text-text-muted leading-relaxed whitespace-pre-wrap">
                  {{ selectedGroup.description || t('global.groups.no_desc') }}
                </p>
              </div>

              <div class="bg-surface-hover rounded-xl p-5 border border-border-soft">
                <h3 class="text-lg font-bold text-text mb-2 flex items-center gap-2"><ShieldAlert :size="18" class="text-primary" /> {{ t('groups.info.rules') }}</h3>
                <p class="text-sm text-text-muted leading-relaxed whitespace-pre-wrap">
                  {{ selectedGroup.rules || t('groups.info.no_rules') }}
                </p>
              </div>

              <div class="grid grid-cols-2 gap-4">
                <div class="bg-surface-hover border border-border-soft rounded-xl p-4 flex flex-col justify-center">
                  <p class="text-xs text-border-strong font-bold uppercase tracking-wider mb-1">
                    {{ t('global.groups.join_state') }}
                  </p>
                  <p class="text-sm font-black text-text flex items-center gap-1">
                    <Check :size="14" class="text-green-500" /> 
                    {{ selectedGroup.joinState === 'open' ? t('global.groups.join_open') : (selectedGroup.joinState === 'request' ? t('global.groups.join_request') : t('global.groups.join_invite')) }}
                  </p>
                </div>
                <div class="bg-surface-hover border border-border-soft rounded-xl p-4 flex flex-col justify-center">
                  <p class="text-xs text-border-strong font-bold uppercase tracking-wider mb-1">
                    {{ t('groups.info.owner') }}
                  </p>
                  <p class="text-sm font-black text-text truncate">
                    {{ selectedGroup.ownerId }}
                  </p>
                </div>
              </div>
            </div>

            <!-- Tab: 成员 -->
            <div v-else-if="activeTab === 'members'" class="h-full">
              <div v-if="groupMembers.length === 0" class="h-full flex flex-col items-center justify-center text-border-strong">
                <UsersRound class="mb-4 opacity-30" :size="48" />
                <p class="font-bold text-text-muted">{{ t('groups.members.no_members') }}</p>
              </div>
              <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
                <div v-for="member in groupMembers" :key="member.id || member.userId || member.user?.id" class="flex items-center gap-3 p-3 bg-surface-hover rounded-xl border border-border-soft hover:border-primary/50 transition-colors">
                  <VrcAvatar :user="member.user" custom-class="w-10 h-10 rounded-full object-cover shadow-sm border border-border-soft shrink-0" />
                  <div class="min-w-0 flex-1">
                    <p class="text-sm font-bold text-text truncate">{{ member.user?.displayName || member.userId }}</p>
                    <p class="text-xs text-text-muted truncate">{{ (member.roleIds || [member.roleId]).filter(Boolean).join(', ') || 'Member' }}</p>
                  </div>
                  <div v-if="hasGroupPermission('group-members-remove') || hasGroupPermission('group-bans-manage')" class="flex items-center gap-1 shrink-0">
                    <button
                      v-if="hasGroupPermission('group-members-remove')"
                      :disabled="actionBusy === `kick:${member.userId || member.user?.id || member.id}`"
                      class="p-1.5 rounded-lg bg-surface border border-border-soft text-text-muted hover:text-red-600 hover:border-red-300 disabled:opacity-50"
                      title="Kick member"
                      @click="kickMember(member)"
                    >
                      <UserX :size="14" />
                    </button>
                    <button
                      v-if="hasGroupPermission('group-bans-manage')"
                      :disabled="actionBusy === `ban:${member.userId || member.user?.id || member.id}`"
                      class="p-1.5 rounded-lg bg-surface border border-border-soft text-text-muted hover:text-red-700 hover:border-red-300 disabled:opacity-50"
                      title="Ban member"
                      @click="banMember(member)"
                    >
                      <Ban :size="14" />
                    </button>
                  </div>
                </div>
              </div>
            </div>

            <!-- Tab: 角色 -->
            <div v-else-if="activeTab === 'roles'" class="h-full">
              <div v-if="groupRoles.length === 0" class="h-full flex flex-col items-center justify-center text-border-strong">
                <Settings class="mb-4 opacity-30" :size="48" />
                <p class="font-bold text-text-muted">{{ t('groups.roles.no_roles') }}</p>
              </div>
              <div v-else class="flex flex-col gap-3">
                <div v-for="role in groupRoles" :key="role.id" class="p-4 bg-surface-hover rounded-xl border border-border-soft">
                  <div class="flex items-center justify-between mb-2">
                    <h4 class="text-base font-bold text-text flex items-center gap-2">
                      <Shield class="text-primary" :size="16"/> {{ role.name }}
                    </h4>
                    <span class="text-xs font-bold bg-background px-2 py-1 rounded text-text-muted">{{ role.id }}</span>
                  </div>
                  <p class="text-sm text-text-muted mb-2">{{ role.description || t('global.groups.no_desc') }}</p>
                  <div class="flex flex-wrap gap-1">
                    <span v-for="perm in role.permissions" :key="perm" class="text-xs px-2 py-0.5 bg-primary/10 text-primary rounded-md">
                      {{ perm }}
                    </span>
                  </div>
                </div>
              </div>
            </div>

            <!-- Tab: 动态 -->
            <div v-else-if="activeTab === 'posts'" class="h-full">
              <div v-if="groupPosts.length === 0" class="h-full flex flex-col items-center justify-center text-border-strong">
                <Megaphone class="mb-4 opacity-30" :size="48" />
                <p class="font-bold text-text-muted">{{ t('groups.posts.no_posts') }}</p>
              </div>
              <div v-else class="flex flex-col gap-4">
                <div v-for="post in groupPosts" :key="post.id" class="p-5 bg-surface-hover rounded-xl border border-border-soft shadow-sm">
                  <div class="flex items-center justify-between mb-3">
                    <h4 class="text-lg font-bold text-text">{{ post.title }}</h4>
                    <span class="text-xs text-text-muted font-mono">{{ new Date(post.createdAt).toLocaleString() }}</span>
                  </div>
                  <p class="text-sm text-text-muted whitespace-pre-wrap">{{ post.text }}</p>
                  <div class="mt-3 flex items-center gap-2" v-if="post.authorId">
                    <VrcAvatar :user="{ id: post.authorId }" custom-class="w-6 h-6 rounded-full shrink-0" />
                    <span class="text-xs text-text-muted truncate">{{ post.authorId }}</span>
                  </div>
                </div>
              </div>
            </div>

            <!-- Tab: Requests -->
            <div v-else-if="activeTab === 'requests'" class="h-full">
              <div v-if="groupJoinRequests.length === 0" class="h-full flex flex-col items-center justify-center text-border-strong">
                <UserX class="mb-4 opacity-30" :size="48" />
                <p class="font-bold text-text-muted">{{ t('entity_modal.no_pending_requests') }}</p>
              </div>
              <div v-else class="grid grid-cols-1 md:grid-cols-2 gap-3">
                <div
                  v-for="req in groupJoinRequests"
                  :key="req.id || req.userId || req.user?.id"
                  class="flex items-center justify-between gap-3 p-3 bg-surface-hover rounded-xl border border-border-soft hover:border-primary/50 transition-colors"
                >
                  <div class="flex items-center gap-3 min-w-0">
                    <VrcAvatar :user="req.user" custom-class="w-10 h-10 rounded-full object-cover shadow-sm border border-border-soft shrink-0" />
                    <div class="min-w-0">
                      <p class="text-sm font-bold text-text truncate">{{ req.user?.displayName || req.userId || req.id }}</p>
                      <p class="text-xs text-text-muted truncate">{{ req.createdAt ? new Date(req.createdAt).toLocaleString() : req.userId }}</p>
                    </div>
                  </div>
                  <div class="flex gap-2 shrink-0">
                    <button
                      :disabled="actionBusy === `accept:${req.id || req.userId || req.user?.id}`"
                      class="p-2 bg-green-100 hover:bg-green-200 text-green-700 rounded-lg transition-colors disabled:opacity-50"
                      title="Accept request"
                      @click="respondJoinRequest(req.id || req.userId || req.user?.id, 'accept')"
                    >
                      <Check :size="16" />
                    </button>
                    <button
                      :disabled="actionBusy === `reject:${req.id || req.userId || req.user?.id}`"
                      class="p-2 bg-red-100 hover:bg-red-200 text-red-700 rounded-lg transition-colors disabled:opacity-50"
                      title="Reject request"
                      @click="respondJoinRequest(req.id || req.userId || req.user?.id, 'reject')"
                    >
                      <Shield :size="16" />
                    </button>
                  </div>
                </div>
              </div>
            </div>

            <!-- Tab: 审计日志 -->
            <div v-else-if="activeTab === 'logs'" class="h-full">
              <div v-if="groupLogs.length === 0" class="h-full flex flex-col items-center justify-center text-border-strong">
                <ScrollText class="mb-4 opacity-30" :size="48" />
                <p class="font-bold text-text-muted">{{ t('groups.logs.no_logs') }}</p>
              </div>
              <div v-else class="flex flex-col gap-2">
                <div v-for="log in groupLogs" :key="log.id" class="p-3 bg-surface-hover rounded-lg border border-border-soft flex items-center justify-between text-sm">
                  <div class="flex flex-col min-w-0">
                    <span class="font-bold text-text truncate">{{ log.eventType }}</span>
                    <span class="text-xs text-text-muted truncate mt-0.5">{{ log.actorId }} {{ log.description ? ' - ' + log.description : '' }}</span>
                  </div>
                  <span class="text-xs text-border-strong font-mono shrink-0 ml-4">{{ new Date(log.created_at).toLocaleString() }}</span>
                </div>
              </div>
            </div>

          </div>
        </div>
      </template>
    </BaseModal>
  </div>
</template>


