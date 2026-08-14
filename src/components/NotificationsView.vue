<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { VrcApi, DbApi, SysApi, GamelogApi } from "../api";
import { Bell, Loader2, UserPlus, Check, X, Megaphone, HelpCircle, UsersRound, MessageSquare, Info, UserCheck, UserX, MapPin, Trash2 } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import type { VrcNotification } from '../types/vrc';
import { useToast } from '../composables/useToast';
import {
  getDisplayNotificationDetails,
  getStoredNotificationMeta,
  normalizeNotificationForDb,
  parseStoredNotificationDetails,
} from '../api/notificationNormalization';

const { t, locale } = useI18n();
const l = (zh: string, en: string) => locale.value.startsWith('zh') ? zh : en;
const toast = useToast();

const notifications = ref<VrcNotification[]>([]);
const loading = ref(true);
const errorMsg = ref('');
const processingId = ref<string | null>(null);
const localNotificationTypes = new Set(['friend-online', 'friend-offline', 'friend-location']);
const actionableNotificationTypes = new Set(['friendRequest', 'requestInvite', 'group.invite', 'group.request']);

const getNotificationTitle = (notif: VrcNotification) => {
  const message = typeof notif.message === 'string' ? notif.message.trim() : '';
  if (message) return message;

  const sender = typeof notif.senderUsername === 'string' ? notif.senderUsername.trim() : '';
  switch (notif.type) {
    case 'friendRequest':
      return sender ? l(`${sender} 发送了好友请求`, `${sender} sent a friend request`) : l('好友请求', 'Friend request');
    case 'invite':
      return sender ? l(`${sender} 邀请你加入实例`, `${sender} invited you to an instance`) : l('实例邀请', 'Instance invite');
    case 'requestInvite':
      return sender ? l(`${sender} 请求加入你的位置`, `${sender} requested to join you`) : l('邀请请求', 'Invite request');
    case 'group.invite':
      return sender ? l(`${sender} 发送了群组邀请`, `${sender} sent a group invite`) : l('群组邀请', 'Group invite');
    case 'group.request':
      return sender ? l(`${sender} 发送了群组申请`, `${sender} sent a group request`) : l('群组申请', 'Group request');
    case 'friend-online':
      return sender ? l(`${sender} 已上线`, `${sender} is online`) : l('好友已上线', 'Friend online');
    case 'friend-offline':
      return sender ? l(`${sender} 已下线`, `${sender} is offline`) : l('好友已下线', 'Friend offline');
    case 'friend-location':
      return sender ? l(`${sender} 切换了世界`, `${sender} changed worlds`) : l('好友切换世界', 'Friend changed worlds');
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

const syncRemoteNotifications = async () => {
  try {
    const [legacyResult, v2Result] = await Promise.allSettled([
      VrcApi.getNotifications({ n: 100, offset: 0 }),
      VrcApi.getNotificationsV2({ n: 100, offset: 0 }),
    ]);
    const remote = [
      ...(legacyResult.status === 'fulfilled' && Array.isArray(legacyResult.value) ? legacyResult.value : []),
      ...(v2Result.status === 'fulfilled' && Array.isArray(v2Result.value) ? v2Result.value : []),
    ];
    if (remote.length > 0) {
      await DbApi.batchSaveNotifications({ notificationsJson: JSON.stringify(remote.map((item) => normalizeNotificationForDb(item))) });
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

  const meta = getStoredNotificationMeta(notif.details);
  const details = parseStoredNotificationDetails(notif.details);
  const responseTypes = action === 'reject' ? new Set(['reject', 'decline', 'delete']) : new Set([action]);
  const response = meta.responses?.find((item) => responseTypes.has(String(item.type || '').toLowerCase()));

  if (action === 'accept' && notif.type === 'requestInvite' && meta.version === 1) {
    const senderId = String(notif.senderUserId || '');
    if (!senderId) throw new Error(l('通知缺少发送者 ID', 'Notification is missing the sender user id'));
    const me: any = await VrcApi.getCurrentUser();
    const location = String(me?.location || '');
    const separator = location.indexOf(':');
    if (separator <= 0 || !location.slice(separator + 1)) {
      throw new Error(l('当前不在可被邀请的房间内', 'You are not currently in an inviteable instance'));
    }
    await VrcApi.sendInviteNotification({
      receiverUserId: senderId,
      instanceId: location.slice(separator + 1),
      worldId: location.slice(0, separator),
      worldName: details.worldName,
      rsvp: true,
    });
    await VrcApi.hideNotification(notif.id);
    return;
  }

  if (response) {
    await VrcApi.sendNotificationResponse({
      notificationId: notif.id,
      responseType: String(response.type),
      responseData: response.data || '',
    });
    return;
  }

  if (meta.version === 2) {
    if (action === 'accept') {
      throw new Error(l('该通知没有接受选项', 'This notification has no accept response'));
    }
    await VrcApi.deleteNotificationV2(notif.id);
    return;
  }

  if (action === 'accept' && notif.type === 'friendRequest') {
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
    toast.success(l('已接受通知', 'Notification accepted'));
  } catch (err: any) {
    errorMsg.value = err?.message || String(err);
    toast.error(l(`接受失败：${errorMsg.value}`, `Could not accept: ${errorMsg.value}`));
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
    toast.success(actionableNotificationTypes.has(notif.type)
      ? l('已拒绝通知', 'Notification declined')
      : l('已隐藏通知', 'Notification hidden'));
  } catch (err: any) {
    errorMsg.value = err?.message || String(err);
    toast.error(l(`操作失败：${errorMsg.value}`, `Action failed: ${errorMsg.value}`));
  } finally {
    processingId.value = null;
  }
};

onMounted(() => {
  refreshNotifications();
  window.addEventListener('vrc-notifications-synced', refreshLocalNotifications);
});

onUnmounted(() => {
  window.removeEventListener('vrc-notifications-synced', refreshLocalNotifications);
});

const filterTab = ref<'all' | 'friend' | 'invite' | 'other'>('all');

const filteredNotifications = computed(() => {
  if (filterTab.value === 'all') return notifications.value;
  if (filterTab.value === 'friend') {
    return notifications.value.filter(n => ['friend-online', 'friend-offline', 'friend-location'].includes(n.type));
  }
  if (filterTab.value === 'invite') {
    return notifications.value.filter(n => ['invite', 'requestInvite', 'group.invite', 'group.request'].includes(n.type));
  }
  return notifications.value.filter(n => !['friend-online', 'friend-offline', 'friend-location', 'invite', 'requestInvite', 'group.invite', 'group.request'].includes(n.type));
});

const clearAllNotifications = async () => {
  if (notifications.value.length === 0) return;
  loading.value = true;
  try {
    const [legacyResult, v2Result] = await Promise.allSettled([
      VrcApi.clearNotifications(),
      VrcApi.clearNotificationsV2(),
    ]);
    const remoteFailed = [legacyResult, v2Result].every((result) => result.status === 'rejected');
    if (remoteFailed) {
      throw new Error(l('远端通知清理失败，未删除本地记录', 'Remote notification clearing failed; local records were kept'));
    }
    await Promise.allSettled(notifications.value.map((notif) => DbApi.deleteNotification({ id: notif.id })));
    notifications.value = [];
    toast.success(l('已清空所有通知', 'All notifications cleared'));
  } catch (err: any) {
    errorMsg.value = err?.message || String(err);
    toast.error(l(`清空失败：${errorMsg.value}`, `Could not clear notifications: ${errorMsg.value}`));
  } finally {
    loading.value = false;
  }
};

const getNotificationIcon = (type: string) => {
  switch (type) {
    case 'friend-online': return UserCheck;
    case 'friend-offline': return UserX;
    case 'friend-location': return MapPin;
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

const renderDetails = (details: any) => {
  if (!details) return '';
  const parsed = getDisplayNotificationDetails(details);
  if (typeof parsed === 'string') return parsed;
  if (Object.keys(parsed).length === 0) return '';
  return parsed.worldName || parsed.message || parsed.location || parsed.imageUrl || JSON.stringify(parsed);
};

const canAcceptNotification = (notif: VrcNotification) => {
  if (notif.type === 'friendRequest' || notif.type === 'requestInvite') return true;
  return Boolean(getStoredNotificationMeta(notif.details).responses?.some(
    (response) => String(response.type || '').toLowerCase() === 'accept',
  ));
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

    <!-- Filter Tabs + Clear All -->
    <div class="flex items-center gap-2 mb-4 shrink-0 z-10">
      <div class="flex gap-1 bg-surface rounded-xl p-1 border-border-soft shadow-sm">
        <button
          v-for="tab in [
            { key: 'all', label: l('全部', 'All') },
            { key: 'friend', label: l('好友', 'Friends') },
            { key: 'invite', label: l('邀请', 'Invites') },
            { key: 'other', label: l('其他', 'Other') },
          ]"
          :key="tab.key"
          class="px-3 py-1.5 rounded-lg text-xs font-bold transition-all"
          :class="filterTab === tab.key ? 'bg-primary text-white shadow-sm' : 'text-text-muted hover:text-text hover:bg-surface-hover'"
          @click="filterTab = tab.key as any"
        >
          {{ tab.label }}
        </button>
      </div>
      <button
        v-if="notifications.length > 0"
        :disabled="loading"
        class="ml-auto px-3 py-1.5 rounded-lg text-xs font-bold bg-red-50 text-red-500 hover:bg-red-100 transition-all disabled:opacity-50 flex items-center gap-1.5 border border-red-200"
        @click="clearAllNotifications"
      >
        <Trash2 :size="14" />
        {{ l('清空全部', 'Clear all') }}
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
        v-else-if="filteredNotifications.length === 0"
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
          v-for="notif in filteredNotifications"
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
              v-if="canAcceptNotification(notif)"
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


