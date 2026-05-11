<script setup lang="ts">
import { Loader2 } from 'lucide-vue-next';

defineProps<{
  show: boolean;
  loading?: boolean;
}>();

defineEmits<{
  (e: 'close'): void;
}>();
</script>

<template>
  <Teleport to="body">
    <Transition name="fade">
      <div
        v-if="show || loading"
        class="fixed inset-0 z-50 flex items-center justify-center p-4"
      >
        <div
          class="absolute inset-0 bg-background/80 backdrop-blur-md/30 backdrop-blur-sm"
          @click="$emit('close')"
        />
        <div class="bg-surface backdrop-blur-xl w-full max-w-lg rounded-3xl shadow-2xl relative z-10 overflow-hidden border-border-soft flex flex-col max-h-[90vh]">
          <div
            v-if="loading"
            class="p-12 text-center"
          >
            <Loader2
              class="animate-spin mx-auto text-primary"
              :size="32"
            />
          </div>
          <div
            v-else-if="show"
            class="overflow-y-auto custom-scrollbar flex-shrink w-full"
          >
            <slot />
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.fade-enter-active, .fade-leave-active { transition: opacity 0.3s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: #cbd5e1; border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: #94a3b8; }
</style>
