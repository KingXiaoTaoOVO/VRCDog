<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from 'vue';
import { ChevronDown } from 'lucide-vue-next';

const props = defineProps<{
  modelValue?: string | number | null;
  options: { label: string; value: string | number | null }[];
}>();

const emit = defineEmits(['update:modelValue']);

const isOpen = ref(false);
const containerRef = ref<HTMLElement | null>(null);
const triggerRef = ref<HTMLElement | null>(null);
const menuRef = ref<HTMLElement | null>(null);
const menuStyle = ref<Record<string, string>>({});

const updateMenuPosition = () => {
  if (!triggerRef.value || !isOpen.value) return;
  const rect = triggerRef.value.getBoundingClientRect();
  const menuHeight = Math.min(248, menuRef.value?.offsetHeight || 248);
  const spaceBelow = window.innerHeight - rect.bottom;
  const openAbove = spaceBelow < menuHeight + 12 && rect.top > spaceBelow;
  const top = openAbove
    ? Math.max(8, rect.top - menuHeight - 6)
    : Math.min(window.innerHeight - menuHeight - 8, rect.bottom + 6);
  const width = Math.max(180, rect.width);
  const left = Math.min(
    Math.max(8, rect.right - width),
    Math.max(8, window.innerWidth - width - 8),
  );

  menuStyle.value = {
    top: `${top}px`,
    left: `${left}px`,
    width: `${width}px`,
  };
};

const toggleOpen = async () => {
  isOpen.value = !isOpen.value;
  if (isOpen.value) {
    await nextTick();
    updateMenuPosition();
  }
};

const selectOption = (value: string | number | null) => {
  emit('update:modelValue', value);
  isOpen.value = false;
};

const handleClickOutside = (event: MouseEvent) => {
  const target = event.target as Node;
  if (
    containerRef.value
    && !containerRef.value.contains(target)
    && !menuRef.value?.contains(target)
  ) {
    isOpen.value = false;
  }
};

onMounted(() => {
  document.addEventListener('mousedown', handleClickOutside);
  window.addEventListener('resize', updateMenuPosition);
  window.addEventListener('scroll', updateMenuPosition, true);
});

onUnmounted(() => {
  document.removeEventListener('mousedown', handleClickOutside);
  window.removeEventListener('resize', updateMenuPosition);
  window.removeEventListener('scroll', updateMenuPosition, true);
});
</script>

<template>
  <div class="relative min-w-0 w-full" ref="containerRef">
    <div
      ref="triggerRef"
      class="flex items-center justify-between gap-2 px-3 py-1.5 bg-surface-hover/60 backdrop-blur-md border border-border-soft rounded-lg cursor-pointer hover:bg-surface-active/60 transition-colors min-w-0"
      @click="toggleOpen"
    >
      <span class="text-[13px] text-text-strong truncate min-w-0" :title="String(options.find(o => o.value === modelValue)?.label || options[0]?.label || '')">
        {{ options.find(o => o.value === modelValue)?.label || options[0]?.label }}
      </span>
      <ChevronDown :size="14" class="text-text-muted transition-transform duration-200 shrink-0" :class="{ 'rotate-180': isOpen }" />
    </div>

    <Teleport to="body">
      <Transition
        enter-active-class="transition duration-100 ease-out"
        enter-from-class="transform scale-95 opacity-0"
        enter-to-class="transform scale-100 opacity-100"
        leave-active-class="transition duration-75 ease-in"
        leave-from-class="transform scale-100 opacity-100"
        leave-to-class="transform scale-95 opacity-0"
      >
        <div
          v-if="isOpen"
          ref="menuRef"
          class="fixed z-[10000] min-w-[180px] bg-surface/95 backdrop-blur-xl border border-border-soft rounded-lg shadow-xl overflow-hidden"
          :style="menuStyle"
        >
          <div class="max-h-60 overflow-y-auto custom-scrollbar p-1">
            <div
              v-for="option in options"
              :key="option.value ?? 'null'"
              class="px-3 py-2 text-[13px] rounded-md cursor-pointer transition-colors flex items-center justify-between gap-2 min-w-0"
              :class="modelValue === option.value ? 'bg-primary/10 text-primary font-bold' : 'text-text-strong hover:bg-surface-active/60'"
              @click="selectOption(option.value)"
            >
              <span class="truncate" :title="String(option.label)">{{ option.label }}</span>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>
