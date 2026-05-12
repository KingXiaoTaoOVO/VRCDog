<template>
  <div class="fixed bottom-6 right-6 z-[9999] flex flex-col items-end gap-2 pointer-events-none">
    <transition-group name="toast">
      <div 
        v-for="toast in toasts" 
        :key="toast.id"
        class="px-4 py-2 rounded-lg shadow-lg backdrop-blur-md flex items-center gap-2 font-medium text-[13px] border pointer-events-auto min-w-[250px]"
        :class="{
          'bg-surface border-border-soft text-text': toast.type === 'info',
          'bg-green-500/80 border-green-400 text-white': toast.type === 'success',
          'bg-red-500/80 border-red-400 text-white': toast.type === 'error',
          'bg-amber-500/80 border-amber-400 text-white': toast.type === 'warning',
        }"
      >
        <span>{{ toast.message }}</span>
      </div>
    </transition-group>
  </div>
</template>
<script setup lang="ts">
import { useToast } from '../composables/useToast';
const { toasts } = useToast();
</script>
<style scoped>
.toast-enter-active,
.toast-leave-active {
  transition: all 0.4s ease-out;
}
.toast-enter-from {
  opacity: 0;
  transform: translateX(100%);
}
.toast-leave-to {
  opacity: 0;
  transform: translateX(100%);
}
</style>
