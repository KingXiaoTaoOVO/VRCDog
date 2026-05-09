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
          class="absolute inset-0 bg-black/30 backdrop-blur-sm"
          @click="$emit('close')"
        />
        <div class="bg-white/95 backdrop-blur-xl w-full max-w-lg rounded-3xl shadow-2xl relative z-10 overflow-hidden border border-amber-100">
          <div
            v-if="loading"
            class="p-12 text-center"
          >
            <Loader2
              class="animate-spin mx-auto text-amber-500"
              :size="32"
            />
          </div>
          <template v-else-if="show">
            <slot />
          </template>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.fade-enter-active, .fade-leave-active { transition: opacity 0.3s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>
