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
    <div class="absolute inset-0 bg-background/80 backdrop-blur-md/70 backdrop-blur-sm" @click="emit('close')"></div>
    
    <!-- Modal -->
    <div class="relative w-full max-w-[500px] h-[600px] flex flex-col bg-surface/60 backdrop-blur-md rounded-xl shadow-2xl border-transparent overflow-hidden text-text">
      <!-- Header -->
      <div class="flex items-center justify-between p-4 border-transparent">
        <h2 class="text-[15px] font-bold text-white">{{ $t('auto_966ed89c') }}</h2>
        <button class="p-1 hover:bg-white/10 rounded-md transition-colors" @click="emit('close')">
          <X class="w-4 h-4 text-text" />
        </button>
      </div>

      <!-- Content / List -->
      <div class="flex-1 overflow-y-auto p-4 custom-scrollbar">
        <div class="space-y-1">
          <div 
            v-for="(item, index) in navItems" 
            :key="item.key"
            class="flex items-center gap-3 py-2 px-3 rounded-lg bg-surface-hover/60 backdrop-blur-md hover:bg-surface-active/60 backdrop-blur-md border-transparent hover:border-transparent transition-colors"
            :class="{'opacity-50': !item.visible}"
            draggable="true"
            @dragstart="onDragStart(index)"
            @dragenter.prevent="onDragEnter(index)"
            @dragend="onDragEnd"
            @dragover.prevent
          >
            <!-- Drag Handle -->
            <div class="cursor-grab active:cursor-grabbing text-text-muted hover:text-text">
              <GripVertical class="w-4 h-4" />
            </div>
            
            <!-- Icon -->
            <div class="w-5 h-5 flex items-center justify-center text-text">
              <component v-if="item.icon" :is="item.icon" class="w-4 h-4" />
              <div v-else class="w-2 h-2 rounded-full bg-surface-hover"></div>
            </div>

            <!-- Label -->
            <span class="flex-1 text-[13px] font-bold text-text">
              {{ $t(item.label) }}
            </span>

            <!-- Visibility Toggle (like eye or check) -->
            <button 
              class="w-6 h-6 flex items-center justify-center rounded transition-colors"
              :class="item.visible ? 'bg-primary border-primary text-white' : 'bg-transparent border-border-soft text-transparent hover:border-border-soft'"
              @click="toggleVisibility(index)"
            >
              <Check v-if="item.visible" class="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      </div>

      <!-- Footer Buttons -->
      <div class="p-4 border-transparent flex items-center justify-between bg-surface/60 backdrop-blur-md">
        <div class="flex items-center gap-2">
          <button class="px-4 py-2 bg-surface-hover/60 backdrop-blur-md hover:bg-surface-active/60 backdrop-blur-md text-white text-[13px] font-bold rounded flex items-center gap-1.5 transition-colors">
            <Plus class="w-4 h-4" />
            {{ t('nav.add_folder') || 'Add Folder' }}
          </button>
          <button class="px-4 py-2 bg-surface-hover/60 backdrop-blur-md hover:bg-surface-active/60 backdrop-blur-md text-white text-[13px] font-bold rounded flex items-center gap-1.5 transition-colors">
            <Folder class="w-4 h-4" />
            {{ t('nav.add_dashboard') || 'New Dashboard' }}
          </button>
        </div>
        
        <div class="flex items-center gap-3">
          <button class="text-red-400 hover:text-red-300 text-[13px] font-bold px-2 transition-colors" @click="restoreDefault">
            {{ t('nav.reset_default') || 'Reset' }}
          </button>
          <button class="px-5 py-2 bg-surface-hover/60 backdrop-blur-md hover:bg-surface-active/60 backdrop-blur-md text-white text-[13px] font-bold rounded transition-colors" @click="emit('close')">
            {{ t('common.cancel') || 'Cancel' }}
          </button>
          <button class="px-5 py-2 bg-primary hover:bg-primary-hover text-white text-[13px] font-bold rounded transition-colors" @click="saveAndClose">
            {{ t('common.confirm') || 'Confirm' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>


