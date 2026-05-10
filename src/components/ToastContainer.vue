<template>
  <div class="fixed top-6 left-1/2 -translate-x-1/2 z-[9999] flex flex-col gap-3 pointer-events-none">
    <transition-group name="toast">
      <div 
        v-for="toast in toasts" 
        :key="toast.id"
        class="px-6 py-3 rounded-2xl shadow-xl backdrop-blur-2xl border flex items-center gap-3 font-bold text-[14px] pointer-events-auto"
        :class="{
          'bg-white/80 border-white/60 text-slate-800': toast.type === 'info',
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
  transition: all 0.4s cubic-bezier(0.175, 0.885, 0.32, 1.275);
}
.toast-enter-from {
  opacity: 0;
  transform: translateY(-30px) scale(0.9);
}
.toast-leave-to {
  opacity: 0;
  transform: translateY(-20px) scale(0.95);
}
</style>
