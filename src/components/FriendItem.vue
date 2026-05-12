<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';

const props = defineProps<{
  friend: any;
  statusColor: string;
  trustColor: string;
  isOffline?: boolean;
}>();

const emit = defineEmits<{
  (e: 'click', friend: any): void;
}>();

const { t } = useI18n();

const getFlag = (location: string) => {
  if (location === 'private') return '🔒';
  if (location === 'traveling') return '✈️';
  if (location === 'offline') return '💤';
  if (location.includes('wrld_')) return '🌎';
  return '🏠';
};

const cleanLocName = (location: string) => {
  if (location === 'private') return t('friends.loc_private');
  if (location === 'traveling') return t('friends.loc_traveling');
  if (location === 'offline') return t('friends.loc_offline');
  const colonIdx = location.indexOf(':');
  return colonIdx > -1 ? location.substring(0, colonIdx) : location;
};
</script>

<template>
  <div 
    class="group flex items-center gap-3.5 py-2.5 px-3.5 cursor-pointer bg-[var(--theme-surface)] border-2 border-transparent hover:border-primary/40 hover:bg-[var(--theme-surface)]-hover rounded-[24px] transition-all duration-300 hover:scale-[1.02] hover:-translate-y-1 hover:shadow-xl hover:shadow-primary/10 mx-2 mb-2"
    @click="emit('click', friend)"
  >
    <div class="relative shrink-0 overflow-visible">
      <div class="w-12 h-12 rounded-[18px] overflow-hidden border-[3px] border-border-soft group-hover:border-primary/50 transition-colors shadow-sm bg-[var(--theme-surface)]">
        <img 
          :src="friend.currentAvatarThumbnailImageUrl || friend.currentAvatarImageUrl || friend.profilePicOverride || 'https://via.placeholder.com/150'" 
          class="w-full h-full object-cover transition-transform duration-500 group-hover:scale-110 group-hover:-rotate-6" 
          :class="{ 'grayscale opacity-60': isOffline }"
        />
      </div>
      <div 
        class="absolute -bottom-1 -right-1 w-4 h-4 rounded-full border-[3px] border-[var(--theme-bg-main)] shadow-md transition-transform group-hover:scale-110" 
        :style="{ backgroundColor: statusColor }"
        :class="{ 'bg-slate-400': isOffline && !statusColor }"
      ></div>
    </div>
    <div class="flex-1 min-w-0 flex flex-col justify-center leading-tight">
      <span 
        class="text-[15px] font-black truncate transition-colors" 
        :style="{ color: trustColor }"
        :class="{ 'text-[var(--theme-text-muted)]': isOffline && !trustColor }"
      >
        {{ friend.displayName || 'Unknown' }}
      </span>
      <!-- slots for subtitle -->
      <slot name="subtitle">
        <div v-if="friend.location && !isOffline" class="flex items-center gap-1.5 mt-1 text-[12px] font-bold text-[var(--theme-text-muted)] truncate bg-black/10 dark:bg-white/5 self-start px-2 py-0.5 rounded-lg border border-border-soft">
          <span v-if="friend.location === 'private'" class="shrink-0 text-orange-400 opacity-80 text-[10px]">🔒</span>
          <span v-else class="shrink-0 text-[10px]">{{ getFlag(friend.location) }}</span>
          <span class="truncate">{{ cleanLocName(friend.location) }}</span>
        </div>
        <div v-else class="text-[12px] font-bold text-[var(--theme-text-muted)] mt-1 truncate bg-black/5 dark:bg-white/5 self-start px-2 py-0.5 rounded-lg border border-border-soft">
          {{ isOffline ? t('auto_50d4a850') : t('auto_7cdc4c2a') }}
        </div>
      </slot>
    </div>
  </div>
</template>
