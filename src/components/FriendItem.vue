<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useEntityModalStore } from '../stores/entityModal';
import { useUserProfileStore } from '../stores/userProfile';

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
const entityStore = useEntityModalStore();
const profileStore = useUserProfileStore();

// Map VRChat language tags to country flag emojis (same as VrcDog)
const LANGUAGE_FLAGS: Record<string, string> = {
  language_eng: '🇺🇸',
  language_kor: '🇰🇷',
  language_rus: '🇷🇺',
  language_spa: '🇪🇸',
  language_por: '🇧🇷',
  language_zho: '🇨🇳',
  language_deu: '🇩🇪',
  language_jpn: '🇯🇵',
  language_fra: '🇫🇷',
  language_swe: '🇸🇪',
  language_nld: '🇳🇱',
  language_pol: '🇵🇱',
  language_dan: '🇩🇰',
  language_nor: '🇳🇴',
  language_ita: '🇮🇹',
  language_tha: '🇹🇭',
  language_fin: '🇫🇮',
  language_hun: '🇭🇺',
  language_ces: '🇨🇿',
  language_tur: '🇹🇷',
  language_ara: '🇸🇦',
  language_ron: '🇷🇴',
  language_vie: '🇻🇳',
  language_msa: '🇲🇾',
  language_ind: '🇮🇩',
  language_ukr: '🇺🇦',
};

// Get country flags from user's language tags
const countryFlags = computed(() => {
  const tags: string[] = props.friend?.tags || [];
  return tags
    .filter(tag => tag.startsWith('language_'))
    .map(tag => LANGUAGE_FLAGS[tag])
    .filter(Boolean)
    .slice(0, 3); // max 3 flags like VrcDog
});

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
  // 对于 wrld_ 开头的，显示世界 ID 的简短形式（完整名称由父组件通过 slot 提供）
  if (location.startsWith('wrld_')) {
    const worldId = location.split(':')[0];
    return worldId.substring(0, 20) + '...';
  }
  const colonIdx = location.indexOf(':');
  return colonIdx > -1 ? location.substring(0, colonIdx) : location;
};

const isWorldLocation = (location: string) => {
  return location && location.startsWith('wrld_');
};

const handleWorldClick = (event: MouseEvent) => {
  event.stopPropagation();
  const location = props.friend?.location;
  if (isWorldLocation(location)) {
    const worldId = location.split(':')[0];
    entityStore.openWorld(worldId);
  }
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
      <div class="flex items-center gap-1 min-w-0">
        <!-- Country flags from language tags (like VrcDog) -->
        <span v-for="flag in countryFlags" :key="flag" class="text-[13px] leading-none shrink-0">{{ flag }}</span>
        <span 
          class="text-[15px] font-black truncate transition-colors" 
          :style="{ color: trustColor || undefined }"
          :class="{ 'text-[var(--theme-text-muted)]': isOffline && !trustColor }"
        >
          {{ friend.displayName || 'Unknown' }}
        </span>
      </div>
      <!-- slots for subtitle -->
      <slot name="subtitle">
         <div v-if="friend.location && !isOffline" class="flex items-center gap-1.5 mt-1 text-[12px] font-bold text-[var(--theme-text-muted)] truncate bg-[var(--theme-bg-main)]/10 dark:bg-[var(--theme-text)]/5 self-start px-2 py-0.5 rounded-lg border border-border-soft">
           <span v-if="friend.location === 'private'" class="shrink-0 text-orange-400 opacity-80 text-[10px]">🔒</span>
           <span v-else-if="friend.location.startsWith('wrld_')" class="shrink-0 text-[10px] cursor-pointer hover:text-primary hover:underline transition-colors" @click.stop="handleWorldClick($event)" title="点击查看世界详情">{{ getFlag(friend.location) }}</span>
           <span v-else class="shrink-0 text-[10px]">{{ getFlag(friend.location) }}</span>
           <span class="truncate cursor-pointer hover:text-primary hover:underline transition-colors"
                 v-if="friend.location.startsWith('wrld_')"
                 @click.stop="handleWorldClick($event)"
                 title="点击查看世界详情">{{ cleanLocName(friend.location) }}</span>
           <span v-else class="truncate">{{ cleanLocName(friend.location) }}</span>
         </div>
          <div v-else class="text-[12px] font-bold text-[var(--theme-text-muted)] mt-1 truncate bg-[var(--theme-bg-main)]/5 dark:bg-[var(--theme-text)]/5 self-start px-2 py-0.5 rounded-lg border border-border-soft">
           {{ isOffline ? t('auto_50d4a850') : t('auto_7cdc4c2a') }}
         </div>
       </slot>
    </div>
  </div>
</template>
