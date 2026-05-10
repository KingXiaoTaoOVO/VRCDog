<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { X, GripVertical, Folder, Plus, RotateCcw, Check } from 'lucide-vue-next';
import { DbApi } from '../api';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const emit = defineEmits(['close', 'save']);

const props = defineProps<{
  initialNavConfig: any[]
}>();

const navItems = ref<any[]>([]);

onMounted(() => {
  if (props.initialNavConfig && props.initialNavConfig.length > 0) {
    navItems.value = JSON.parse(JSON.stringify(props.initialNavConfig));
  }
});

// Basic Drag and Drop
const draggedIndex = ref<number | null>(null);

const onDragStart = (index: number) => {
  draggedIndex.value = index;
};

const onDragEnter = (index: number) => {
  if (draggedIndex.value === null) return;
  if (draggedIndex.value === index) return;
  
  const items = [...navItems.value];
  const draggedItem = items[draggedIndex.value];
  
  items.splice(draggedIndex.value, 1);
  items.splice(index, 0, draggedItem);
  
  navItems.value = items;
  draggedIndex.value = index;
};

const onDragEnd = () => {
  draggedIndex.value = null;
};

const toggleVisibility = (index: number) => {
  navItems.value[index].visible = !navItems.value[index].visible;
};

const restoreDefault = () => {
  // We'll emit a special event for this
  emit('save', null);
};

const saveAndClose = () => {
  emit('save', navItems.value);
};

</script>

<template>
  <div class="fixed inset-0 z-[9999] flex items-center justify-center p-4">
    <!-- Backdrop -->
    <div class="absolute inset-0 bg-black/70 backdrop-blur-sm" @click="emit('close')"></div>
    
    <!-- Modal -->
    <div class="relative w-full max-w-[500px] h-[600px] flex flex-col bg-[#1e1f22] rounded-xl shadow-2xl border border-white/10 overflow-hidden text-slate-200">
      <!-- Header -->
      <div class="flex items-center justify-between p-4 border-b border-white/5">
        <h2 class="text-[15px] font-bold text-white">自定义导航栏</h2>
        <button class="p-1 hover:bg-white/10 rounded-md transition-colors" @click="emit('close')">
          <X class="w-4 h-4 text-slate-400" />
        </button>
      </div>

      <!-- Content / List -->
      <div class="flex-1 overflow-y-auto p-4 custom-scrollbar">
        <div class="space-y-1">
          <div 
            v-for="(item, index) in navItems" 
            :key="item.key"
            class="flex items-center gap-3 py-2 px-3 rounded-lg bg-[#2b2d31] hover:bg-[#35373c] border border-transparent hover:border-white/5 transition-colors"
            :class="{'opacity-50': !item.visible}"
            draggable="true"
            @dragstart="onDragStart(index)"
            @dragenter.prevent="onDragEnter(index)"
            @dragend="onDragEnd"
            @dragover.prevent
          >
            <!-- Drag Handle -->
            <div class="cursor-grab active:cursor-grabbing text-slate-500 hover:text-slate-300">
              <GripVertical class="w-4 h-4" />
            </div>
            
            <!-- Icon -->
            <div class="w-5 h-5 flex items-center justify-center text-slate-400">
              <component v-if="item.icon" :is="item.icon" class="w-4 h-4" />
              <div v-else class="w-2 h-2 rounded-full bg-slate-500"></div>
            </div>

            <!-- Label -->
            <span class="flex-1 text-[13px] font-bold text-slate-200">
              {{ $t(item.label) }}
            </span>

            <!-- Visibility Toggle (like eye or check) -->
            <button 
              class="w-6 h-6 flex items-center justify-center rounded border transition-colors"
              :class="item.visible ? 'bg-indigo-500 border-indigo-500 text-white' : 'bg-transparent border-slate-600 text-transparent hover:border-slate-400'"
              @click="toggleVisibility(index)"
            >
              <Check v-if="item.visible" class="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      </div>

      <!-- Footer Buttons -->
      <div class="p-4 border-t border-white/5 flex items-center justify-between bg-[#1e1f22]">
        <div class="flex items-center gap-2">
          <button class="px-4 py-2 bg-[#2b2d31] hover:bg-[#35373c] text-white text-[13px] font-bold rounded flex items-center gap-1.5 transition-colors">
            <Plus class="w-4 h-4" />
            添加文件夹
          </button>
          <button class="px-4 py-2 bg-[#2b2d31] hover:bg-[#35373c] text-white text-[13px] font-bold rounded flex items-center gap-1.5 transition-colors">
            <Folder class="w-4 h-4" />
            新建仪表板
          </button>
        </div>
        
        <div class="flex items-center gap-3">
          <button class="text-red-400 hover:text-red-300 text-[13px] font-bold px-2 transition-colors" @click="restoreDefault">
            恢复默认
          </button>
          <button class="px-5 py-2 bg-[#2b2d31] hover:bg-[#35373c] text-white text-[13px] font-bold rounded transition-colors" @click="emit('close')">
            取消
          </button>
          <button class="px-5 py-2 bg-white hover:bg-slate-200 text-slate-900 text-[13px] font-bold rounded transition-colors" @click="saveAndClose">
            确认
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 8px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: #3f4147; border-radius: 4px; border: 2px solid #1e1f22; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: #565860; }
</style>
