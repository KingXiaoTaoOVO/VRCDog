<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue';
import { Search, Users, Ghost, Globe, UsersRound } from 'lucide-vue-next';

const isVisible = ref(false);
const searchQuery = ref('');
const inputRef = ref<HTMLInputElement | null>(null);

const emit = defineEmits(['navigate']);

const toggleModal = () => {
  isVisible.value = !isVisible.value;
  if (isVisible.value) {
    searchQuery.value = '';
    nextTick(() => {
      if (inputRef.value) inputRef.value.focus();
    });
  }
};

const handleKeyDown = (e: KeyboardEvent) => {
  if (e.ctrlKey && e.key === 'k') {
    e.preventDefault();
    toggleModal();
  }
  if (e.key === 'Escape' && isVisible.value) {
    isVisible.value = false;
  }
};

onMounted(() => {
  window.addEventListener('keydown', handleKeyDown);
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeyDown);
});

const selectOption = (type: string) => {
  isVisible.value = false;
  emit('navigate', 'search');
  // Optional: we can send the type to the search view via a global store or window event
  window.dispatchEvent(new CustomEvent('vrc-global-search', { detail: { type, query: searchQuery.value } }));
};
</script>

<template>
  <div v-if="isVisible" class="fixed inset-0 z-[9999] flex items-start justify-center pt-[15vh] px-4">
    <!-- Backdrop -->
    <div class="absolute inset-0 bg-background/80 backdrop-blur-md/60 backdrop-blur-sm" @click="isVisible = false"></div>
    
    <!-- Modal -->
    <div class="relative w-full max-w-[600px] bg-surface rounded-xl shadow-2xl border-transparent overflow-hidden transform transition-all text-text-muted">
      <!-- Search Input -->
      <div class="flex items-center px-4 py-3 border-transparent">
        <Search class="w-5 h-5 text-border-strong mr-3" />
        <input 
          ref="inputRef"
          v-model="searchQuery"
          type="text" 
          :placeholder="$t('auto_e5f71fc3')" 
          class="flex-1 bg-transparent border-none outline-none text-lg text-white placeholder-slate-500"
          @keydown.enter="selectOption('all')"
        >
      </div>

      <!-- Options -->
      <div class="p-2">
        <div class="px-3 py-2 text-xs font-bold text-text-muted mb-1">{{ $t('auto_e978cbe8') }}</div>
        
        <div 
          class="flex items-center justify-between px-3 py-3 rounded-lg hover:bg-surface cursor-pointer transition-colors group"
          @click="selectOption('user')"
        >
          <div class="flex items-center gap-3">
            <Users class="w-4 h-4 text-border-strong group-hover:text-white" />
            <span class="text-sm font-medium">{{ $t('auto_59d29a36') }}</span>
          </div>
          <span class="text-xs text-text-muted group-hover:text-border-strong">{{ $t('auto_6d337922') }}</span>
        </div>

        <div 
          class="flex items-center justify-between px-3 py-3 rounded-lg hover:bg-surface cursor-pointer transition-colors group"
          @click="selectOption('avatar')"
        >
          <div class="flex items-center gap-3">
            <Ghost class="w-4 h-4 text-border-strong group-hover:text-white" />
            <span class="text-sm font-medium">{{ $t('auto_8000f187') }}</span>
          </div>
          <span class="text-xs text-text-muted group-hover:text-border-strong">{{ $t('auto_f7c19728') }}</span>
        </div>

        <div 
          class="flex items-center justify-between px-3 py-3 rounded-lg hover:bg-surface cursor-pointer transition-colors group"
          @click="selectOption('world')"
        >
          <div class="flex items-center gap-3">
            <Globe class="w-4 h-4 text-border-strong group-hover:text-white" />
            <span class="text-sm font-medium">{{ $t('auto_c086b300') }}</span>
          </div>
          <span class="text-xs text-text-muted group-hover:text-border-strong">{{ $t('auto_c201c3af') }}</span>
        </div>

        <div 
          class="flex items-center justify-between px-3 py-3 rounded-lg hover:bg-surface cursor-pointer transition-colors group"
          @click="selectOption('group')"
        >
          <div class="flex items-center gap-3">
            <UsersRound class="w-4 h-4 text-border-strong group-hover:text-white" />
            <span class="text-sm font-medium">{{ $t('auto_4b0ab7ba') }}</span>
          </div>
          <span class="text-xs text-text-muted group-hover:text-border-strong">{{ $t('auto_7bd27eb1') }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
