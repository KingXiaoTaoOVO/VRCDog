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
    <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" @click="isVisible = false"></div>
    
    <!-- Modal -->
    <div class="relative w-full max-w-[600px] bg-[#1a1b1e] rounded-xl shadow-2xl border border-white/10 overflow-hidden transform transition-all text-slate-200">
      <!-- Search Input -->
      <div class="flex items-center px-4 py-3 border-b border-white/5">
        <Search class="w-5 h-5 text-slate-400 mr-3" />
        <input 
          ref="inputRef"
          v-model="searchQuery"
          type="text" 
          placeholder="搜索" 
          class="flex-1 bg-transparent border-none outline-none text-lg text-white placeholder-slate-500"
          @keydown.enter="selectOption('all')"
        >
      </div>

      <!-- Options -->
      <div class="p-2">
        <div class="px-3 py-2 text-xs font-bold text-slate-500 mb-1">搜点什么...</div>
        
        <div 
          class="flex items-center justify-between px-3 py-3 rounded-lg hover:bg-white/10 cursor-pointer transition-colors group"
          @click="selectOption('user')"
        >
          <div class="flex items-center gap-3">
            <Users class="w-4 h-4 text-slate-400 group-hover:text-white" />
            <span class="text-sm font-medium">好友</span>
          </div>
          <span class="text-xs text-slate-500 group-hover:text-slate-400">以及你给他们的备注</span>
        </div>

        <div 
          class="flex items-center justify-between px-3 py-3 rounded-lg hover:bg-white/10 cursor-pointer transition-colors group"
          @click="selectOption('avatar')"
        >
          <div class="flex items-center gap-3">
            <Ghost class="w-4 h-4 text-slate-400 group-hover:text-white" />
            <span class="text-sm font-medium">模型</span>
          </div>
          <span class="text-xs text-slate-500 group-hover:text-slate-400">看看有什么好玩的模型</span>
        </div>

        <div 
          class="flex items-center justify-between px-3 py-3 rounded-lg hover:bg-white/10 cursor-pointer transition-colors group"
          @click="selectOption('world')"
        >
          <div class="flex items-center gap-3">
            <Globe class="w-4 h-4 text-slate-400 group-hover:text-white" />
            <span class="text-sm font-medium">世界</span>
          </div>
          <span class="text-xs text-slate-500 group-hover:text-slate-400">找个好玩的地方玩玩</span>
        </div>

        <div 
          class="flex items-center justify-between px-3 py-3 rounded-lg hover:bg-white/10 cursor-pointer transition-colors group"
          @click="selectOption('group')"
        >
          <div class="flex items-center gap-3">
            <UsersRound class="w-4 h-4 text-slate-400 group-hover:text-white" />
            <span class="text-sm font-medium">群组</span>
          </div>
          <span class="text-xs text-slate-500 group-hover:text-slate-400">找不到组织了吗?</span>
        </div>
      </div>
    </div>
  </div>
</template>
