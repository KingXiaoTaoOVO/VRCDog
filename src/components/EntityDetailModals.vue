<script setup lang="ts">
import { useToast } from "../composables/useToast";

const toast = useToast();
import { useEntityModalStore } from '../stores/entityModal';
import BaseModal from './BaseModal.vue';
import VrcAvatar from './VrcAvatar.vue';
import { useI18n } from 'vue-i18n';
import { SysApi, VrcApi } from '../api';
import { Globe, UsersRound, Shield, Check } from 'lucide-vue-next';

import { useUserProfileStore } from '../stores/userProfile';

import { ref, computed } from 'vue';
const { t } = useI18n();
const entityStore = useEntityModalStore();
const profileStore = useUserProfileStore();

const groupActiveTab = ref<'info' | 'members' | 'requests'>('info');

const handleJoinRequest = async (requestId: string, action: 'accept' | 'reject') => {
  try {
    await VrcApi.respondGroupJoinRequest({ groupId: entityStore.selectedGroup!.id, requestId, action });
    // Refresh admin data
    entityStore.fetchGroupAdminData(entityStore.selectedGroup!.id);
  } catch (err) {
    console.error("Failed to handle join request", err);
  }
};

const imageInput = ref<HTMLInputElement | null>(null);
const isUploadingImage = ref(false);

const handleAvatarUpload = async (e: Event) => {
  const target = e.target as HTMLInputElement;
  if (!target.files?.length) return;
  const file = target.files[0];

  try {
    isUploadingImage.value = true;
    const reader = new FileReader();
    reader.onload = async (ev) => {
      try {
        const base64 = ev.target?.result as string;
        const res = await VrcApi.uploadVrcPlusImage(base64, 'avatarimage', entityStore.selectedAvatar!.id);
        
        if (res.versions && res.versions.length > 0) {
            const latestVer = res.versions[res.versions.length - 1];
            const fileUrl = latestVer.file?.url;
            
            if (fileUrl) {
               await VrcApi.updateAvatar({
                   id: entityStore.selectedAvatar!.id,
                   imageUrl: fileUrl
               });
               toast.info(t('entity_modal.upload_avatar_success'));
               entityStore.closeAvatar();
            } else {
               throw new Error(t('entity_modal.no_url_returned'));
            }
        }
      } catch (err: any) {
        toast.error(t('entity_modal.upload_failed') + (err.message || err));
      } finally {
        isUploadingImage.value = false;
      }
    };
    reader.readAsDataURL(file);
  } catch (err) {
    isUploadingImage.value = false;
  }
};

const worldImageInput = ref<HTMLInputElement | null>(null);
const isUploadingWorldImage = ref(false);

const handleWorldUpload = async (e: Event) => {
  const target = e.target as HTMLInputElement;
  if (!target.files?.length) return;
  const file = target.files[0];

  try {
    isUploadingWorldImage.value = true;
    const reader = new FileReader();
    reader.onload = async (ev) => {
      try {
        const base64 = ev.target?.result as string;
        const res = await VrcApi.uploadVrcPlusImage(base64, 'worldimage', entityStore.selectedWorld!.id);
        
        if (res.versions && res.versions.length > 0) {
            const latestVer = res.versions[res.versions.length - 1];
            const fileUrl = latestVer.file?.url;
            
            if (fileUrl) {
               await VrcApi.updateWorld({
                   id: entityStore.selectedWorld!.id,
                   imageUrl: fileUrl
               });
               toast.info(t('entity_modal.upload_world_success'));
               entityStore.closeWorld();
            } else {
               throw new Error(t('entity_modal.no_url_returned'));
            }
        }
      } catch (err: any) {
        toast.error(t('entity_modal.upload_failed') + (err.message || err));
      } finally {
        isUploadingWorldImage.value = false;
      }
    };
    reader.readAsDataURL(file);
  } catch (err) {
    isUploadingWorldImage.value = false;
  }
};
</script>

<template>
  <!-- 世界详情弹窗 -->
  <BaseModal
    :show="entityStore.isWorldOpen"
    :loading="entityStore.loadingWorld"
    @close="entityStore.closeWorld()"
  >
    <template v-if="entityStore.selectedWorld">
      <div class="aspect-video bg-surface relative overflow-hidden rounded-t-2xl">
        <VrcAvatar
          :user="entityStore.selectedWorld"
          :url="entityStore.selectedWorld.imageUrl || entityStore.selectedWorld.thumbnailImageUrl"
          custom-class="w-full h-full object-cover"
        />
        <button
          class="absolute top-4 right-4 p-2 rounded-full bg-surface backdrop-blur-md hover:bg-background/80 backdrop-blur-md/60 text-white backdrop-blur transition-colors"
          @click="entityStore.closeWorld()"
        >
          ✕
        </button>
      </div>
      <div class="p-6">
        <h2 class="text-2xl font-black text-text mb-1 leading-tight">
          {{ entityStore.selectedWorld.name }}
        </h2>
        <p class="text-sm text-text-muted font-bold mb-4">
          {{ t('search.author') }}: {{ entityStore.selectedWorld.authorName }}
        </p>
        <div class="grid grid-cols-3 gap-3 text-center mb-5">
          <div class="bg-surface-hover border-border-soft rounded-xl p-3">
            <p class="text-lg font-black text-text">
              {{ entityStore.selectedWorld.capacity || '?' }}
            </p>
            <p class="text-[10px] text-border-strong font-bold uppercase tracking-wider mt-1">
              {{ t('search.capacity') }}
            </p>
          </div>
          <div class="bg-surface-hover border-border-soft rounded-xl p-3">
            <p class="text-lg font-black text-text">
              {{ entityStore.selectedWorld.favorites || 0 }}
            </p>
            <p class="text-[10px] text-border-strong font-bold uppercase tracking-wider mt-1">
              {{ t('search.favorites_count') }}
            </p>
          </div>
          <div class="bg-surface-hover border-border-soft rounded-xl p-3">
            <p class="text-lg font-black text-text">
              {{ entityStore.selectedWorld.visits || 0 }}
            </p>
            <p class="text-[10px] text-border-strong font-bold uppercase tracking-wider mt-1">
              {{ t('search.visits') }}
            </p>
          </div>
        </div>
        
        <div class="mb-4">
          <p class="text-sm text-text-muted leading-relaxed whitespace-pre-wrap max-h-48 overflow-y-auto custom-scrollbar pr-2">
            {{ entityStore.selectedWorld.description || t('entity_modal.no_description') }}
          </p>
        </div>
              
        <div
          v-if="entityStore.selectedWorld.tags?.length"
          class="flex flex-wrap gap-1.5 mb-5"
        >
          <span
            v-for="tag in entityStore.selectedWorld.tags.filter((t: string) => !t.startsWith('system_') && !t.startsWith('admin_'))"
            :key="tag"
            class="text-[10px] font-bold bg-surface border-border-soft text-text-muted px-2.5 py-1 rounded-md uppercase"
          >{{ tag }}</span>
        </div>

        <div class="pt-4 border-border-soft flex items-center justify-end gap-3">
          <input
            ref="worldImageInput"
            type="file"
            accept="image/png, image/jpeg, image/webp"
            class="hidden"
            @change="handleWorldUpload"
          >
          <button
            v-if="entityStore.selectedWorld.authorId === profileStore.baseInfo?.id"
            class="px-6 py-2.5 font-bold rounded-xl text-sm transition-all flex items-center gap-2 shadow-sm bg-surface-hover border-border-soft text-text-muted hover:bg-surface"
            :disabled="isUploadingWorldImage"
            @click="worldImageInput?.click()"
          >
            <span v-if="isUploadingWorldImage">{{ t('entity_modal.uploading') }}</span>
            <span v-else>{{ t('entity_modal.change_world_img') }}</span>
          </button>

          <button
            class="px-6 py-2.5 font-bold rounded-xl text-sm transition-all flex items-center gap-2 shadow-sm" 
            :class="entityStore.isWorldFavorited ? 'bg-red-50 border-red-200 text-red-600 hover:bg-red-100' : 'bg-primary/10 border-primary text-primary hover:bg-primary/10'"
            @click="entityStore.toggleFavoriteWorld()"
          >
            <span>{{ entityStore.isWorldFavorited ? t('search.remove_favorite') : '⭐ ' + t('search.add_favorite') }}</span>
          </button>
        </div>

        <div
          v-if="entityStore.selectedWorld.instances?.length"
          class="mt-6 pt-5 border-border-soft"
        >
          <h3 class="text-base font-extrabold text-text mb-3 flex items-center gap-2">
            <Globe class="text-primary w-5 h-5" /> {{ t('entity_modal.active_instances') }}
          </h3>
          <div class="space-y-2 max-h-48 overflow-y-auto custom-scrollbar pr-2">
            <div
              v-for="inst in entityStore.selectedWorld.instances"
              :key="inst[0]"
              class="p-3 bg-surface-hover hover:bg-primary/10 rounded-xl border-border-soft transition-colors flex items-center justify-between"
            >
              <div class="flex items-center gap-2">
                <span class="font-bold text-text">#{{ inst[0] }}</span>
                <span class="text-xs font-bold text-primary bg-primary/10 px-2 py-0.5 rounded-full">{{ inst[1] }} / {{ entityStore.selectedWorld.capacity }}</span>
              </div>
              <div class="flex gap-2">
                <button
                  class="px-3 py-1.5 bg-surface border-border-soft hover:border-primary text-primary rounded-lg text-xs font-bold shadow-sm transition-all"
                  @click="SysApi.launchVrc({ launchArgs: `vrchat://launch?id=${entityStore.selectedWorld.id}:${inst[0]}` })"
                >
                  {{ t('entity_modal.join') }}
                </button>
                <button
                  class="px-3 py-1.5 bg-primary/10 hover:bg-primary/10 text-white rounded-lg text-xs font-bold shadow-sm transition-all"
                  @click="VrcApi.inviteMyself({ worldId: entityStore.selectedWorld.id, instanceId: inst[0] })"
                >
                  {{ t('entity_modal.drop_portal') }}
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </template>
  </BaseModal>

  <!-- 模型详情弹窗 -->
  <BaseModal
    :show="entityStore.isAvatarOpen"
    :loading="entityStore.loadingAvatar"
    @close="entityStore.closeAvatar()"
  >
    <template v-if="entityStore.selectedAvatar">
      <div class="aspect-[3/4] max-h-80 w-full bg-surface relative overflow-hidden rounded-t-2xl">
        <VrcAvatar
          :user="entityStore.selectedAvatar"
          :url="entityStore.selectedAvatar.imageUrl || entityStore.selectedAvatar.thumbnailImageUrl"
          custom-class="w-full h-full object-cover"
        />
        <button
          class="absolute top-4 right-4 p-2 rounded-full bg-surface backdrop-blur-md hover:bg-background/80 backdrop-blur-md/60 text-white backdrop-blur transition-colors"
          @click="entityStore.closeAvatar()"
        >
          ✕
        </button>
      </div>
      <div class="p-6">
        <h2 class="text-xl font-black text-text mb-1 leading-tight">
          {{ entityStore.selectedAvatar.name }}
        </h2>
        <p class="text-sm text-text-muted font-bold mb-4">
          {{ t('search.author') }}: {{ entityStore.selectedAvatar.authorName }}
        </p>
        <p class="text-sm text-text-muted mb-4 leading-relaxed whitespace-pre-wrap max-h-40 overflow-y-auto custom-scrollbar">
          {{ entityStore.selectedAvatar.description || t('entity_modal.no_description') }}
        </p>
        
        <div
          v-if="entityStore.selectedAvatar.tags?.length"
          class="flex flex-wrap gap-1.5 mb-5"
        >
          <span
            v-for="tag in entityStore.selectedAvatar.tags"
            :key="tag"
            class="text-[10px] font-bold bg-surface border-border-soft text-text-muted px-2.5 py-1 rounded-md uppercase"
          >{{ tag }}</span>
        </div>

        <div class="pt-4 border-border-soft flex items-center justify-end gap-3">
          <input
            ref="imageInput"
            type="file"
            accept="image/png, image/jpeg, image/webp"
            class="hidden"
            @change="handleAvatarUpload"
          >
          <button
            v-if="entityStore.selectedAvatar.authorId === profileStore.baseInfo?.id"
            class="px-6 py-2.5 font-bold rounded-xl text-sm transition-all flex items-center gap-2 shadow-sm bg-surface-hover border-border-soft text-text-muted hover:bg-surface"
            :disabled="isUploadingImage"
            @click="imageInput?.click()"
          >
            <span v-if="isUploadingImage">{{ t('entity_modal.uploading') }}</span>
            <span v-else>{{ t('entity_modal.change_avatar_img') }}</span>
          </button>
          
          <button
            class="px-6 py-2.5 font-bold rounded-xl text-sm transition-all flex items-center gap-2 shadow-sm" 
            :class="entityStore.isAvatarFavorited ? 'bg-red-50 border-red-200 text-red-600 hover:bg-red-100' : 'bg-primary/10 border-primary text-primary hover:bg-primary/10'"
            @click="entityStore.toggleFavoriteAvatar()"
          >
            <span>{{ entityStore.isAvatarFavorited ? t('search.remove_favorite') : '⭐ ' + t('search.add_favorite') }}</span>
          </button>
        </div>
      </div>
    </template>
  </BaseModal>

  <!-- 群组详情弹窗 -->
  <BaseModal
    :show="entityStore.isGroupOpen"
    :loading="entityStore.loadingGroup"
    @close="entityStore.closeGroup()"
  >
    <template v-if="entityStore.selectedGroup">
      <div class="h-32 bg-surface relative overflow-hidden rounded-t-2xl">
        <VrcAvatar
          :user="entityStore.selectedGroup"
          :url="entityStore.selectedGroup.bannerUrl"
          custom-class="w-full h-full object-cover opacity-80"
        />
        <button
          class="absolute top-4 right-4 p-2 rounded-full bg-surface backdrop-blur-md hover:bg-background/80 backdrop-blur-md/60 text-white backdrop-blur transition-colors"
          @click="entityStore.closeGroup()"
        >
          ✕
        </button>
      </div>
      <div class="p-6 relative">
        <div class="flex gap-4 mb-4">
          <div class="w-20 h-20 -mt-12 rounded-xl border-4 border-border-strong shadow-md bg-surface flex-shrink-0 relative z-10 overflow-hidden">
            <VrcAvatar
              :user="entityStore.selectedGroup"
              :url="entityStore.selectedGroup.iconUrl"
              custom-class="w-full h-full object-cover"
            />
          </div>
          <div class="flex-1 pb-1 min-w-0">
            <h2 class="text-xl font-black text-text truncate">
              {{ entityStore.selectedGroup.name }}
            </h2>
            <div class="flex items-center gap-2 mt-1">
              <span class="text-xs font-bold text-text-muted uppercase">{{ entityStore.selectedGroup.shortCode }}</span>
              <span class="w-1 h-1 rounded-full bg-surface" />
              <span class="text-xs font-bold text-primary flex items-center gap-1"><UsersRound :size="12" /> {{ entityStore.selectedGroup.memberCount || 0 }} {{ t('entity_modal.members') }}</span>
            </div>
          </div>
        </div>
        
        <div class="flex border-border-soft mb-4 px-2">
          <button
            class="px-4 py-2 text-sm font-bold border-b-2 transition-colors"
            :class="groupActiveTab === 'info' ? 'border-primary text-primary' : 'border-transparent text-text-muted hover:text-text-muted'"
            @click="groupActiveTab = 'info'"
          >
            {{ t('entity_modal.info') }}
          </button>
          <button
            class="px-4 py-2 text-sm font-bold border-b-2 transition-colors"
            :class="groupActiveTab === 'members' ? 'border-primary text-primary' : 'border-transparent text-text-muted hover:text-text-muted'"
            @click="groupActiveTab = 'members'"
          >
            {{ t('entity_modal.members') }} ({{ entityStore.groupMembers.length }})
          </button>
          <button
            v-if="entityStore.groupPermissions.includes('group-join-requests-manage')"
            class="px-4 py-2 text-sm font-bold border-b-2 transition-colors flex items-center gap-2"
            :class="groupActiveTab === 'requests' ? 'border-primary text-primary' : 'border-transparent text-text-muted hover:text-text-muted'"
            @click="groupActiveTab = 'requests'"
          >
            {{ t('entity_modal.requests') }}
            <span
              v-if="entityStore.groupJoinRequests.length > 0"
              class="px-1.5 py-0.5 bg-red-500 text-white text-[10px] rounded-full"
            >{{ entityStore.groupJoinRequests.length }}</span>
          </button>
        </div>

        <div v-if="groupActiveTab === 'info'">
          <div class="mb-5">
            <p class="text-sm text-text-muted leading-relaxed whitespace-pre-wrap max-h-48 overflow-y-auto custom-scrollbar">
              {{ entityStore.selectedGroup.description || t('entity_modal.no_group_desc') }}
            </p>
          </div>

          <div class="grid grid-cols-2 gap-3 mb-5">
            <div class="bg-surface-hover border-border-soft rounded-xl p-3 flex flex-col justify-center">
              <p class="text-[10px] text-border-strong font-bold uppercase tracking-wider mb-1">
                {{ t('entity_modal.privacy_status') }}
              </p>
              <p class="text-sm font-black text-text flex items-center gap-1">
                <Shield
                  :size="14"
                  class="text-blue-500"
                /> {{ entityStore.selectedGroup.privacy === 'public' ? t('entity_modal.public_group') : t('entity_modal.private_group') }}
              </p>
            </div>
            <div class="bg-surface-hover border-border-soft rounded-xl p-3 flex flex-col justify-center">
              <p class="text-[10px] text-border-strong font-bold uppercase tracking-wider mb-1">
                {{ t('entity_modal.join_state') }}
              </p>
              <p class="text-sm font-black text-text flex items-center gap-1">
                <Check
                  :size="14"
                  class="text-green-500"
                /> {{ entityStore.selectedGroup.joinState === 'open' ? t('entity_modal.open_join') : (entityStore.selectedGroup.joinState === 'request' ? t('entity_modal.request_join') : t('entity_modal.invite_only')) }}
              </p>
            </div>
          </div>
        </div>

        <div
          v-else-if="groupActiveTab === 'members'"
          class="max-h-64 overflow-y-auto custom-scrollbar pr-2 space-y-2"
        >
          <div
            v-if="entityStore.loadingGroupData"
            class="text-center py-4 text-border-strong font-bold text-sm"
          >
            {{ t('entity_modal.loading_members') }}
          </div>
          <div
            v-else-if="entityStore.groupMembers.length === 0"
            class="text-center py-4 text-border-strong font-bold text-sm"
          >
            {{ t('entity_modal.no_member_data') }}
          </div>
          
          <div
            v-for="member in entityStore.groupMembers"
            :key="member.id"
            class="flex items-center justify-between p-3 bg-surface-hover border-border-soft rounded-xl hover:border-primary transition-colors"
          >
            <div class="flex items-center gap-3">
              <VrcAvatar
                :user="member.user"
                :url="member.user?.profilePicOverride || member.user?.thumbnailUrl || member.user?.currentAvatarThumbnailImageUrl"
                custom-class="w-10 h-10 rounded-full"
              />
              <div>
                <div class="font-bold text-text text-sm">
                  {{ member.user?.displayName }}
                </div>
                <div class="flex gap-1 mt-0.5">
                  <span
                    v-for="roleId in member.roleIds"
                    :key="roleId"
                    class="text-[10px] bg-background/20 text-text-muted px-1.5 rounded"
                  >{{ entityStore.groupRoles.find((r:any) => r.id === roleId)?.name || 'Role' }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div
          v-else-if="groupActiveTab === 'requests'"
          class="max-h-64 overflow-y-auto custom-scrollbar pr-2 space-y-2"
        >
          <div
            v-if="entityStore.loadingGroupData"
            class="text-center py-4 text-border-strong font-bold text-sm"
          >
            {{ t('entity_modal.loading_requests') }}
          </div>
          <div
            v-else-if="entityStore.groupJoinRequests.length === 0"
            class="text-center py-4 text-border-strong font-bold text-sm"
          >
            {{ t('entity_modal.no_pending_requests') }}
          </div>
          
          <div
            v-for="req in entityStore.groupJoinRequests"
            :key="req.id"
            class="flex items-center justify-between p-3 bg-surface-hover border-border-soft rounded-xl hover:border-primary transition-colors"
          >
            <div class="flex items-center gap-3">
              <VrcAvatar
                :user="req.user"
                :url="req.user?.profilePicOverride || req.user?.thumbnailUrl || req.user?.currentAvatarThumbnailImageUrl"
                custom-class="w-10 h-10 rounded-full"
              />
              <div>
                <div class="font-bold text-text text-sm">
                  {{ req.user?.displayName }}
                </div>
                <div class="text-[10px] text-text-muted">
                  {{ new Date(req.createdAt).toLocaleString() }}
                </div>
              </div>
            </div>
            <div class="flex gap-2">
              <button
                class="p-1.5 bg-green-100 hover:bg-green-200 text-green-700 rounded-lg transition-colors"
                @click="handleJoinRequest(req.id, 'accept')"
              >
                <Check :size="16" />
              </button>
              <button
                class="p-1.5 bg-red-100 hover:bg-red-200 text-red-700 rounded-lg transition-colors"
                @click="handleJoinRequest(req.id, 'reject')"
              >
                <Shield :size="16" />
              </button>
            </div>
          </div>
        </div>
        
        <div class="pt-4 border-border-soft flex items-center justify-between">
          <div class="text-xs text-border-strong font-mono">
            {{ entityStore.selectedGroup.id }}
          </div>
          <button
            class="px-6 py-2.5 bg-primary/10 hover:bg-primary/10 text-white font-bold rounded-xl text-sm transition-colors shadow-sm"
            @click="SysApi.launchVrc({ launchArgs: `vrchat://launch?id=${entityStore.selectedGroup.id}` })"
          >
            {{ t('entity_modal.view_in_vrc') }}
          </button>
        </div>
      </div>
    </template>
  </BaseModal>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: #cbd5e1; border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: #94a3b8; }
</style>
