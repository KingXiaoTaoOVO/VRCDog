<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { ChevronDown } from 'lucide-vue-next';

const props = defineProps<{
  modelValue?: string | number | null;
  options: { label: string; value: string | number | null }[];
}>();

const emit = defineEmits(['update:modelValue']);

const isOpen = ref(false);
const containerRef = ref<HTMLElement | null>(null);

const toggleOpen = () => {
  isOpen.value = !isOpen.value;
};

const selectOption = (value: string | number | null) => {
  emit('update:modelValue', value);
  isOpen.value = false;
};

const handleClickOutside = (event: MouseEvent) => {
  if (containerRef.value && !containerRef.value.contains(event.target as Node)) {
    isOpen.value = false;
  }
};

onMounted(() => {
  document.addEventListener('mousedown', handleClickOutside);
});

onUnmounted(() => {
  document.removeEventListener('mousedown', handleClickOutside);
});
</script>

<template>
  <div class="relative" ref="containerRef">
    <div 
      class="flex items-center justify-between gap-2 px-3 py-1.5 bg-surface-hover/60 backdrop-blur-md border border-border-soft rounded-lg cursor-pointer hover:bg-surface-active/60 transition-colors"
      @click="toggleOpen"
    >
      <span class="text-[13px] text-text-strong">
        {{ options.find(o => o.value === modelValue)?.label || options[0]?.label }}
      </span>
      <ChevronDown :size="14" class="text-text-muted transition-transform duration-200" :class="{ 'rotate-180': isOpen }" />
    </div>

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
        class="absolute z-50 w-full min-w-[120px] right-0 mt-1 bg-surface/90 backdrop-blur-xl border border-border-soft rounded-lg shadow-xl overflow-hidden"
      >
        <div class="max-h-60 overflow-y-auto custom-scrollbar p-1">
          <div 
            v-for="option in options" 
            :key="option.value ?? 'null'"
            class="px-3 py-2 text-[13px] rounded-md cursor-pointer transition-colors flex items-center justify-between"
            :class="modelValue === option.value ? 'bg-primary/10 text-primary font-bold' : 'text-text-strong hover:bg-surface-active/60'"
            @click="selectOption(option.value)"
          >
            {{ option.label }}
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>
