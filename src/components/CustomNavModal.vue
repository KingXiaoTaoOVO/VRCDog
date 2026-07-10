<script setup lang="ts">
import { ref, onBeforeUnmount, onMounted } from 'vue';
import { X, GripVertical, Folder, Plus, Check } from 'lucide-vue-next';
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

const draggedIndex = ref<number | null>(null);
const dragOverIndex = ref<number | null>(null);

const moveItem = (fromIndex: number, toIndex: number) => {
  if (fromIndex === toIndex) return;

  const items = [...navItems.value];
  const [draggedItem] = items.splice(fromIndex, 1);
  if (!draggedItem) return;

  items.splice(toIndex, 0, draggedItem);
  navItems.value = items;
  draggedIndex.value = toIndex;
  dragOverIndex.value = toIndex;
};

const findTargetIndex = (clientX: number, clientY: number) => {
  const target = document.elementFromPoint(clientX, clientY);
  const row = target?.closest?.('[data-nav-index]');
  if (!row) return null;

  const rawIndex = (row as HTMLElement).dataset.navIndex;
  if (rawIndex === undefined) return null;

  const index = Number(rawIndex);
  return Number.isFinite(index) ? index : null;
};

const onDragPointerDown = (index: number, event: PointerEvent) => {
  if (event.button !== 0) return;
  event.preventDefault();
  draggedIndex.value = index;
  dragOverIndex.value = index;

  window.addEventListener('pointermove', onDragPointerMove);
  window.addEventListener('pointerup', onDragPointerUp, { once: true });
  window.addEventListener('pointercancel', onDragPointerUp, { once: true });
};

const onDragPointerMove = (event: PointerEvent) => {
  if (draggedIndex.value === null) return;
  event.preventDefault();

  const targetIndex = findTargetIndex(event.clientX, event.clientY);
  if (targetIndex === null || targetIndex === draggedIndex.value) return;

  moveItem(draggedIndex.value, targetIndex);
};

const onDragPointerUp = () => {
  draggedIndex.value = null;
  dragOverIndex.value = null;
  window.removeEventListener('pointermove', onDragPointerMove);
  window.removeEventListener('pointercancel', onDragPointerUp);
};

onBeforeUnmount(() => {
  window.removeEventListener('pointermove', onDragPointerMove);
  window.removeEventListener('pointerup', onDragPointerUp);
  window.removeEventListener('pointercancel', onDragPointerUp);
});

const moveItemByKeyboard = (index: number, direction: -1 | 1) => {
  const nextIndex = index + direction;
  if (nextIndex < 0 || nextIndex >= navItems.value.length) return;
  moveItem(index, nextIndex);
  onDragPointerUp();
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
    <div class="absolute inset-0 bg-background/80 backdrop-blur-sm" @click="emit('close')"></div>
    
    <!-- Modal -->
    <div class="relative w-full max-w-[520px] h-[600px] flex flex-col bg-surface/95 backdrop-blur-md rounded-xl shadow-2xl ring-1 ring-border-soft overflow-hidden text-text">
      <!-- Header -->
      <div class="flex items-center justify-between px-4 py-3 bg-surface-hover/70">
        <h2 class="text-[15px] font-bold text-text">{{ $t('auto_966ed89c') }}</h2>
        <button class="p-1.5 hover:bg-surface-active/80 rounded-md transition-colors text-text-muted hover:text-text" @click="emit('close')">
          <X class="w-4 h-4" />
        </button>
      </div>

      <!-- Content / List -->
      <div class="flex-1 overflow-y-auto p-4 custom-scrollbar">
        <div class="space-y-1">
          <div 
            v-for="(item, index) in navItems" 
            :key="item.key"
            :data-nav-index="index"
            class="flex items-center gap-3 py-2 px-3 rounded-lg bg-surface-hover/60 hover:bg-surface-active/70 transition-colors"
            :class="{
              'opacity-50': !item.visible,
              'ring-2 ring-primary/40 bg-primary/10 shadow-sm': draggedIndex === index,
              'scale-[0.99]': dragOverIndex === index
            }"
          >
            <!-- Drag Handle -->
            <div
              class="cursor-grab active:cursor-grabbing text-text-muted hover:text-text touch-none select-none"
              role="button"
              tabindex="0"
              :aria-label="`${$t(item.label)} drag handle`"
              @pointerdown="onDragPointerDown(index, $event)"
              @keydown.up.prevent="moveItemByKeyboard(index, -1)"
              @keydown.down.prevent="moveItemByKeyboard(index, 1)"
            >
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
              :class="item.visible ? 'bg-primary text-white shadow-sm shadow-primary/20' : 'bg-surface text-transparent ring-1 ring-border-soft hover:ring-border-strong'"
              @click="toggleVisibility(index)"
            >
              <Check v-if="item.visible" class="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      </div>

      <!-- Footer Buttons -->
      <div class="p-4 flex items-center justify-between gap-3 bg-surface-hover/70">
        <div class="flex items-center gap-2 min-w-0">
          <button class="px-3 py-2 bg-surface hover:bg-surface-active text-text text-[13px] font-bold rounded-lg flex items-center gap-1.5 transition-colors ring-1 ring-border-soft whitespace-nowrap">
            <Plus class="w-4 h-4" />
            {{ t('nav.add_folder') || 'Add Folder' }}
          </button>
          <button class="px-3 py-2 bg-surface hover:bg-surface-active text-text text-[13px] font-bold rounded-lg flex items-center gap-1.5 transition-colors ring-1 ring-border-soft whitespace-nowrap">
            <Folder class="w-4 h-4" />
            {{ t('nav.add_dashboard') || 'New Dashboard' }}
          </button>
        </div>
        
        <div class="flex items-center gap-2 shrink-0">
          <button class="text-red-500 hover:text-red-600 text-[13px] font-bold px-2 transition-colors whitespace-nowrap" @click="restoreDefault">
            {{ t('nav.reset_default') || 'Reset' }}
          </button>
          <button class="px-4 py-2 bg-surface hover:bg-surface-active text-text text-[13px] font-bold rounded-lg transition-colors ring-1 ring-border-soft whitespace-nowrap" @click="emit('close')">
            {{ t('common.cancel') || 'Cancel' }}
          </button>
          <button class="px-4 py-2 bg-primary hover:bg-primary-hover text-white text-[13px] font-bold rounded-lg transition-colors shadow-sm shadow-primary/20 whitespace-nowrap" @click="saveAndClose">
            {{ t('common.confirm') || 'Confirm' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>


