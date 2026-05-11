<script setup lang="ts">
import { useI18n } from 'vue-i18n';
const { t } = useI18n();
import { ref, computed } from 'vue';
import { Users, Globe, UserCircle, Star, Copy, ExternalLink, Play, MoreVertical, Shield } from 'lucide-vue-next';
import { SysApi, VrcApi } from '../api';

const props = defineProps<{
  type: 'world' | 'avatar' | 'user' | 'group';
  data: any;
  minimal?: boolean;
  isUser?: boolean;
}>();

const emit = defineEmits(['click', 'favorite']);

const isHovered = ref(false);
const imageError = ref(false);

const imageUrl = computed(() => {
  if (imageError.value) return ''; // fallback to placeholder
  if (props.isUser || props.type === 'user') {
     return props.data.currentAvatarImageUrl || props.data.profilePicOverride || props.data.currentAvatarThumbnailImageUrl || '';
  }
  if (props.type === 'group') {
     return props.data.bannerUrl || props.data.iconUrl || '';
  }
  return props.data.imageUrl || props.data.thumbnailImageUrl || '';
});

const title = computed(() => props.data.displayName || props.data.name || 'Unknown');
const author = computed(() => {
   if (props.type === 'group') {
      return props.data.shortCode || 'GROUP';
   }
   return props.data.authorName || (props.isUser || props.type === 'user' ? props.data.statusDescription : 'Unknown');
});

const copyId = async () => {
  if (props.data.id) {
    await navigator.clipboard.writeText(props.data.id);
  }
};

const openInBrowser = () => {
  if (props.data.id) {
    let url = '';
    if (props.type === 'world') {
      url = `https://vrchat.com/home/world/${props.data.id}`;
    } else if (props.isUser || props.type === 'user') {
      url = `https://vrchat.com/home/user/${props.data.id}`;
    } else if (props.type === 'group') {
      url = `https://vrchat.com/home/group/${props.data.id}`;
    } else {
      url = `https://vrchat.com/home/avatar/${props.data.id}`;
    }
    import('@tauri-apps/plugin-shell').then(({ open }) => open(url)).catch(() => {});
  }
};

const showMenu = ref(false);
let closeMenuTimeout: any = null;

const toggleMenu = () => {
  showMenu.value = !showMenu.value;
};

const handleMouseLeave = () => {
  isHovered.value = false;
  closeMenuTimeout = setTimeout(() => {
    showMenu.value = false;
  }, 300);
};

const handleMouseEnter = () => {
  isHovered.value = true;
  if (closeMenuTimeout) {
    clearTimeout(closeMenuTimeout);
  }
};
</script>

<template>
  <div 
    class="bg-surface backdrop-blur-md rounded-2xl overflow-hidden shadow-sm relative group hover:shadow-lg transition-all cursor-pointer border"
    :class="[minimal ? 'border-transparent hover:border-indigo-200 flex items-center gap-3 p-2.5 bg-surface-hover hover:bg-surface' : 'border-border-soft hover:border-indigo-300 flex-col hover:-translate-y-1']"
    @click="emit('click', data)"
    @mouseenter="handleMouseEnter"
    @mouseleave="handleMouseLeave"
  >
    <!-- Image Area -->
    <div 
      class="bg-background/10 relative overflow-hidden shrink-0"
      :class="[minimal ? 'w-11 h-11 rounded-xl shadow-sm' : 'aspect-video w-full rounded-t-2xl']"
    >
      <!-- Image or Placeholder -->
      <img 
        v-if="imageUrl"
        :src="imageUrl" 
        class="w-full h-full object-cover group-hover:scale-110 transition-transform duration-700 ease-out"
        @error="imageError = true" 
      >
      <div
        v-else
        class="w-full h-full flex items-center justify-center bg-surface0/50 text-border-strong"
      >
        <Globe
          v-if="type === 'world'"
          :size="minimal ? 18 : 32"
        />
        <Shield
          v-else-if="type === 'group'"
          :size="minimal ? 18 : 32"
        />
        <UserCircle
          v-else
          :size="minimal ? 18 : 32"
        />
      </div>
      
      <!-- Group Icon Badge -->
      <img
        v-if="!minimal && type === 'group' && data.iconUrl"
        :src="data.iconUrl"
        class="absolute -bottom-4 left-4 w-14 h-14 rounded-xl border-[3px] border-border-strong shadow-md object-cover z-10 bg-surface"
      >

      <!-- Quick Action Overlay on Hover (Not in minimal mode) -->
      <div 
        v-if="!minimal"
        class="absolute inset-0 bg-gradient-to-t from-slate-900/80 via-slate-900/20 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300 flex flex-col justify-between p-3 z-20"
      >
        <div class="flex justify-end relative">
          <!-- Context Menu Button -->
          <button 
            class="p-2 bg-black/40 hover:bg-black/60 backdrop-blur-md rounded-xl text-white transition-all hover:scale-105 active:scale-95"
            @click.stop="toggleMenu"
          >
            <MoreVertical :size="16" />
          </button>
          
          <!-- Dropdown Menu -->
          <div 
            v-if="showMenu"
            class="absolute top-full right-0 mt-2 w-40 bg-surface backdrop-blur-xl rounded-xl shadow-xl border border-border-soft0/60 overflow-hidden z-30 py-1.5 animate-fade-in origin-top-right"
          >
            <button
              class="w-full text-left px-4 py-2.5 text-xs font-bold text-text-muted hover:bg-surface-hover hover:text-indigo-600 flex items-center gap-2.5 transition-colors"
              @click.stop="copyId"
            >
              <Copy :size="14" /> 复制 ID
            </button>
            <button
              class="w-full text-left px-4 py-2.5 text-xs font-bold text-text-muted hover:bg-surface-hover hover:text-indigo-600 flex items-center gap-2.5 transition-colors"
              @click.stop="openInBrowser"
            >
              <ExternalLink :size="14" /> 浏览器打开
            </button>
            <button
              v-if="type !== 'group'"
              class="w-full text-left px-4 py-2.5 text-xs font-bold text-indigo-500 hover:bg-surface-hover hover:text-indigo-600 flex items-center gap-2.5 transition-colors"
              @click.stop="emit('favorite', data)"
            >
              <Star :size="14" /> 收藏/分组
            </button>
          </div>
        </div>
        
        <!-- Join/Launch Action for Worlds -->
        <div
          v-if="type === 'world'"
          class="flex justify-start"
        >
          <button
            class="px-4 py-2 bg-indigo-500 hover:bg-indigo-600 text-white text-xs font-extrabold rounded-xl shadow-lg shadow-indigo-500/30 backdrop-blur-md flex items-center gap-2 transition-all transform translate-y-4 opacity-0 group-hover:translate-y-0 group-hover:opacity-100 active:scale-95"
            @click.stop=""
          >
            <Play :size="14" /> 创建实例
          </button>
        </div>
      </div>
      
      <!-- Badges -->
      <div
        v-if="!minimal"
        class="absolute bottom-3 right-3 flex gap-1.5 z-10"
      >
        <span
          v-if="type === 'world' && (data.capacity || data.occupants)"
          class="px-2 py-1 bg-black/60 backdrop-blur-md text-white text-[10px] font-black rounded-lg flex items-center gap-1.5 shadow-sm border border-white/10"
        >
          <Users :size="12" /> {{ data.occupants || 0 }} / {{ data.capacity || '?' }}
        </span>
        <span
          v-if="type === 'group' && data.memberCount"
          class="px-2 py-1 bg-black/60 backdrop-blur-md text-white text-[10px] font-black rounded-lg flex items-center gap-1.5 shadow-sm border border-white/10"
        >
          <Users :size="12" /> {{ data.memberCount }}
        </span>
      </div>
    </div>

    <!-- Info Area -->
    <div :class="[minimal ? 'flex-1 min-w-0' : 'p-4 pt-4']">
      <h4 
        class="font-extrabold text-text truncate" 
        :class="[minimal ? 'text-[13px] leading-tight' : 'text-[15px]', type === 'group' && !minimal ? 'ml-12' : '']"
        :title="title"
      >
        {{ title }}
      </h4>
      <p 
        class="text-text-muted truncate mt-1" 
        :class="[minimal ? 'text-[11px] font-medium' : 'text-xs font-bold', type === 'group' && !minimal ? 'ml-12 uppercase tracking-wider' : '']"
        :title="author"
      >
        {{ (isUser || type === 'user' || type === 'group') ? author : 'by ' + author }}
      </p>
      
      <div
        v-if="!minimal && data.tags && data.tags.length"
        class="mt-3 flex gap-1.5 overflow-hidden h-5"
      >
        <span 
          v-for="tag in data.tags.slice(0, 3).filter((t: string) => !t.startsWith('author_tag'))" 
          :key="tag"
          class="px-2 py-0.5 bg-background/10 text-text-muted border border-border-soft0/50 rounded-md text-[10px] font-bold whitespace-nowrap uppercase tracking-wide truncate max-w-[70px]"
        >
          {{ tag.replace('author_tag_', '') }}
        </span>
      </div>
    </div>
  </div>
</template>
