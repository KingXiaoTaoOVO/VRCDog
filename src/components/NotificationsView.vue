<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { VrcApi, DbApi, SysApi, GamelogApi } from "../api";
import { Bell, Loader2, UserPlus, Check, X, Megaphone, HelpCircle, UsersRound, MessageSquare, Info, UserCheck, UserX } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import type { VrcNotification } from '../types/vrc';
import { useToast } from '../composables/useToast';

const { t } = useI18n();
const toast = useToast();

const notifications = ref<VrcNotification[]>([]);
const loading = ref(true);
const errorMsg = ref('');
const processingId = ref<string | null>(null);
const localNotificationTypes = new Set(['friend-online', 'friend-offline']);
const actionableNotificationTypes = new Set(['friendRequest', 'invite', 'requestInvite', 'group.invite', 'group.request']);

const getNotificationTitle = (notif: VrcNotification) => {
  const message = typeof notif.message === 'string' ? notif.message.trim() : '';
  if (message) return message;

  const sender = typeof notif.senderUsername === 'string' ? notif.senderUsername.trim() : '';
  switch (notif.type) {
    case 'friendRequest':
      return sender ? `${sender} 发送了好友请求` : '好友请求';
    case 'invite':
      return sender ? `${sender} 邀请你加入实例` : '实例邀请';
    case 'requestInvite':
      return sender ? `${sender} 请求加入你的位置` : '邀请请求';
    case 'group.invite':
      return sender ? `${sender} 发送了群组邀请` : '群组邀请';
    case 'group.request':
      return sender ? `${sender} 发送了群组申请` : '群组申请';
    case 'friend-online':
      return sender ? `${sender} 已上线` : '好友已上线';
    case 'friend-offline':
      return sender ? `${sender} 已下线` : '好友已下线';
    default:
      return sender || '';
  }
};

const isRenderableNotification = (notif: VrcNotification) => {
  if (!notif?.id) return false;
  if (getNotificationTitle(notif)) return true;
  if (renderDetails(notif.details)) return true;
  return localNotificationTypes.has(notif.type) || actionableNotificationTypes.has(notif.type);
};

const normalizeNotificationForDb = (notif: any) => {
  const createdAt = notif.created_at || notif.createdAt || (notif.createdAtMs ? new Date(Number(notif.createdAtMs)).toISOString() : '');
  return {
    id: notif.id,
    type: notif.type || 'notification',
    senderUserId: notif.senderUserId || null,
    senderUsername: notif.senderUsername || notif.senderDisplayName || '',
    receiverUserId: notif.receiverUserId || null,
    message: notif.message || notif.title || '',
    details: typeof notif.details === 'object' ? JSON.stringify(notif.details || {}) : (notif.details || ''),
    created_at: createdAt || new Date().toISOString()
  };
};

const syncRemoteNotifications = async () => {
  try {
    const remote: any = await VrcApi.getNotifications({ n: 100, offset: 0 });
    if (Array.isArray(remote) && remote.length > 0) {
      await DbApi.batchSaveNotifications({ notificationsJson: JSON.stringify(remote.map(normalizeNotificationForDb)) });
    }
  } catch (err) {
    console.warn('Sync VRChat notifications failed:', err);
  }
};

const fetchNotifications = async (syncRemote = true) => {
  loading.value = true;
  errorMsg.value = '';
  try {
    if (syncRemote) {
      await syncRemoteNotifications();
    }
    const res: any = await DbApi.getNotifications({ limit: 100, offset: 0 });
    const allNotifications = Array.isArray(res) ? res : [];
    const visibleNotifications = allNotifications.filter(isRenderableNotification);
    const staleNotifications = allNotifications.filter((notif: VrcNotification) => !isRenderableNotification(notif));
    notifications.value = visibleNotifications;
    await Promise.allSettled(staleNotifications.map((notif: VrcNotification) => DbApi.deleteNotification({ id: notif.id })));
  } catch (err: any) {
    errorMsg.value = err.message || err;
  } finally {
    loading.value = false;
  }
};

const refreshNotifications = () => {
  void fetchNotifications(true);
};

const refreshLocalNotifications = () => {
  void fetchNotifications(false);
};

const tryRemoteNotificationAction = async (notif: VrcNotification, action: 'accept' | 'reject' | 'hide') => {
  if (localNotificationTypes.has(notif.type)) return;

  if (action === 'accept') {
    try {
      await VrcApi.acceptNotification(notif.id);
      return;
    } catch (legacyError) {
      await VrcApi.sendNotificationResponse({
        notificationId: notif.id,
        responseType: 'accept',
      }).catch(() => {
        throw legacyError;
      });
      return;
    }
  }

  if (action === 'reject') {
    try {
      await VrcApi.sendNotificationResponse({
        notificationId: notif.id,
        responseType: 'reject',
      });
      return;
    } catch {
      // Legacy notifications use hide as the effective reject/dismiss action.
    }
  }

  try {
    await VrcApi.hideNotification(notif.id);
  } catch (legacyError) {
    await VrcApi.hideNotificationV2(notif.id).catch(() => {
      throw legacyError;
    });
  }
};

const acceptNotification = async (id: string) => {
  const notif = notifications.value.find((item) => item.id === id);
  if (!notif) return;
  processingId.value = id;
  errorMsg.value = '';
  try {
    await tryRemoteNotificationAction(notif, 'accept');
    await DbApi.deleteNotification({ id });
    notifications.value = notifications.value.filter((item) => item.id !== id);
    toast.success('已接受通知');
  } catch (err: any) {
    errorMsg.value = err?.message || String(err);
    toast.error(`接受失败：${errorMsg.value}`);
  } finally {
    processingId.value = null;
  }
};

const rejectNotification = async (notif: VrcNotification) => {
  processingId.value = notif.id;
  errorMsg.value = '';
  try {
    await tryRemoteNotificationAction(notif, actionableNotificationTypes.has(notif.type) ? 'reject' : 'hide');
    await DbApi.deleteNotification({ id: notif.id });
    notifications.value = notifications.value.filter((item) => item.id !== notif.id);
    toast.success(actionableNotificationTypes.has(notif.type) ? '已拒绝通知' : '已隐藏通知');
  } catch (err: any) {
    errorMsg.value = err?.message || String(err);
    toast.error(`操作失败：${errorMsg.value}`);
  } finally {
    processingId.value = null;
  }
};

onMounted(() => {
  refreshNotifications();
  window.addEventListener('vrc-notifications-synced', refreshLocalNotifications);
  window.addEventListener('vrc-pipeline-event', handlePipelineEvent);
});

onUnmounted(() => {
  window.removeEventListener('vrc-notifications-synced', refreshLocalNotifications);
  window.removeEventListener('vrc-pipeline-event', handlePipelineEvent);
});

const handlePipelineEvent = (e: Event) => {
  const json = (e as CustomEvent).detail;
  if (json && ['notification', 'hide-notification', 'clear-notification', 'friend-online', 'friend-offline'].includes(json.type)) {
    fetchNotifications(false);
  }
};

const getNotificationIcon = (type: string) => {
  switch (type) {
    case 'friend-online': return UserCheck;
    case 'friend-offline': return UserX;
    case 'friendRequest': return UserPlus;
    case 'invite': 
    case 'requestInvite': return Megaphone;
    case 'group.invite':
    case 'group.request': return UsersRound;
    case 'message': return MessageSquare;
    case 'group.informational':
    case 'group.announcement': return Info;
    default: return HelpCircle;
  }
};

const parseDetails = (details: string | any) => {
  if (!details) return null;
  if (typeof details === 'string') {
    try { 
      const parsed = JSON.parse(details); 
      return Object.keys(parsed).length > 0 ? parsed : null;
    } catch { return details; }
  }
  return Object.keys(details).length > 0 ? details : null;
};

const renderDetails = (details: any) => {
  const parsed = parseDetails(details);
  if (!parsed) return '';
  if (typeof parsed === 'string') return parsed;
  return parsed.worldName || parsed.message || parsed.location || parsed.imageUrl || JSON.stringify(parsed);
};
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-surface-hover rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-primary/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <div class="flex items-center justify-between mb-8 shrink-0 z-10">
      <h1 class="text-3xl font-extrabold text-text tracking-tight flex items-center gap-3">
        <span class="inline-flex items-center justify-center p-2 bg-primary/10 rounded-2xl shadow-sm border-primary">
          <Bell class="w-6 h-6 text-primary" />
        </span>
        {{ t('notifications.title') }}
      </h1>
      <button
        :disabled="loading"
        class="p-2.5 rounded-xl bg-surface border-border-soft shadow-sm text-text-muted hover:text-primary hover:border-primary transition-all disabled:opacity-50"
        @click="refreshNotifications"
      >
        <Loader2
          v-if="loading"
          class="animate-spin"
          :size="20"
        />
        <Bell
          v-else
          :size="20"
        />
      </button>
    </div>

    <!-- 错误 -->
    <div
      v-if="errorMsg"
      class="bg-red-50 text-red-600 p-3 rounded-xl border-red-200 text-sm font-bold mb-4 z-10"
    >
      {{ errorMsg }}
    </div>

    <div class="flex-1 overflow-y-auto pr-2 custom-scrollbar z-10 relative">
      <div
        v-if="loading && notifications.length === 0"
        class="absolute inset-0 flex flex-col items-center justify-center text-primary bg-surface-hover backdrop-blur-sm z-10"
      >
        <Loader2
          class="animate-spin mb-4"
          :size="48"
        />
        <span class="font-extrabold text-lg tracking-wide">{{ t('notifications.loading') }}</span>
      </div>

      <div
        v-else-if="notifications.length === 0"
        class="h-full flex flex-col items-center justify-center text-border-strong"
      >
        <Bell
          class="mb-4 opacity-30"
          :size="64"
        />
        <p class="font-bold text-xl text-text-muted">
          {{ t('notifications.empty') }}
        </p>
      </div>

      <div
        v-else
        class="grid gap-3 pb-10"
      >
        <div
          v-for="notif in notifications"
          :key="notif.id" 
          class="bg-surface backdrop-blur-xl rounded-2xl p-4 border-border-soft shadow-sm flex items-start gap-4 transition-all hover:border-primary hover:shadow-md group"
        >
          <div class="w-12 h-12 rounded-2xl bg-primary/10 border-primary flex items-center justify-center flex-shrink-0 text-primary group-hover:scale-105 transition-transform">
            <component
              :is="getNotificationIcon(notif.type)"
              :size="24"
            />
          </div>
          
          <div class="flex-1 min-w-0 py-1">
            <h3 class="font-bold text-text text-base mb-1 truncate">
              {{ getNotificationTitle(notif) }}
            </h3>
            <p
              v-if="renderDetails(notif.details)"
              class="text-sm text-text-muted bg-surface-hover p-3 rounded-xl break-words whitespace-pre-wrap leading-relaxed border-border-soft"
            >
              {{ renderDetails(notif.details) }}
            </p>
            <p class="text-[11px] text-border-strong mt-2 font-mono font-bold tracking-wider">
              {{ new Date(notif.created_at).toLocaleString() }}
            </p>
          </div>

          <div class="flex flex-col gap-2 flex-shrink-0">
            <button
              v-if="actionableNotificationTypes.has(notif.type)"
              :disabled="processingId === notif.id"
              class="w-10 h-10 rounded-xl bg-green-500 text-white hover:bg-green-600 transition-colors flex items-center justify-center shadow-sm disabled:opacity-50"
              @click="acceptNotification(notif.id)"
            >
              <Loader2
                v-if="processingId === notif.id"
                class="animate-spin"
                :size="18"
              />
              <Check
                v-else
                :size="20"
              />
            </button>
            <button 
              :disabled="processingId === notif.id"
              class="w-10 h-10 rounded-xl bg-surface text-text-muted hover:bg-red-50 hover:text-red-500 hover:hover:border-red-200 transition-all flex items-center justify-center disabled:opacity-50"
              @click="rejectNotification(notif)"
            >
              <X :size="20" />
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>


