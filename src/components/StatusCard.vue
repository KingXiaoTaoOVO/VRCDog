<script setup lang="ts">
import { CheckCircle2, Download, Trash2, Loader2, AlertCircle } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

export type ComponentStatus = 'checking' | 'not_installed' | 'installing' | 'installed' | 'error';

defineProps<{
  title: string;
  description: string;
  status: ComponentStatus;
  installLabel?: string;
  errorMessage?: string;
  progress?: number;
  progressMessage?: string;
  iconSrc?: string;
}>();

const emit = defineEmits(['install', 'uninstall', 'launch']);
</script>

<template>
  <div class="glass-panel p-6 flex flex-col justify-between transition-all duration-300 glass-panel-hover group relative overflow-hidden min-h-[220px]">
    <!-- Status glow effect -->
    <div 
      class="absolute -top-10 -right-10 w-32 h-32 rounded-full blur-3xl opacity-30 transition-colors duration-500"
      :class="{
        'bg-blue-300': status === 'checking',
        'bg-red-300': status === 'not_installed',
        'bg-indigo-300': status === 'installing',
        'bg-green-300': status === 'installed',
        'bg-red-500': status === 'error'
      }"
    />

    <div class="z-10 relative">
      <div class="flex items-start justify-between mb-4">
        <div class="flex items-center gap-3">
          <img
            v-if="iconSrc"
            :src="iconSrc"
            class="w-8 h-8 object-contain drop-shadow-sm"
            alt="Icon"
          >
          <h3 class="text-xl font-extrabold text-text">
            {{ title }}
          </h3>
        </div>
        
        <!-- Status Icon -->
        <div class="flex items-center justify-center w-10 h-10 rounded-full bg-surface border-2 border-border-soft shadow-sm">
          <Loader2
            v-if="status === 'checking' || status === 'installing'"
            class="w-5 h-5 text-indigo-500 animate-spin"
          />
          <CheckCircle2
            v-else-if="status === 'installed'"
            class="w-5 h-5 text-green-500"
          />
          <AlertCircle
            v-else-if="status === 'not_installed' || status === 'error'"
            class="w-5 h-5 text-red-400"
          />
        </div>
      </div>
      
      <p class="text-text text-sm mb-6 h-10 font-medium">
        {{ description }}
      </p>
      
      <div
        v-if="status === 'error' && errorMessage"
        class="text-xs text-red-600 mb-4 p-3 bg-red-100 border-2 border-red-200 rounded-2xl font-bold"
      >
        {{ errorMessage }}
      </div>
    </div>
    
    <!-- Actions -->
    <div class="flex items-center gap-2 z-10 relative mt-auto flex-shrink-0">
      <button 
        v-if="status === 'not_installed' || status === 'error'"
        class="btn-cute flex-1 flex items-center justify-center gap-2 bg-[#f59e0b] hover:bg-[#d97706] text-white py-2 px-3 rounded-[20px] font-bold shadow-lg shadow-indigo-500/30 border border-border-soft text-sm flex-shrink-0"
        @click="emit('install')"
      >
        <Download class="w-4 h-4" />
        {{ installLabel || 'Install' }}
      </button>
      
      <button 
        v-if="status === 'installed'"
        class="btn-cute flex-1 flex items-center justify-center gap-2 bg-indigo-500 hover:bg-indigo-600 text-white border-2 border-indigo-600 py-2 px-2 rounded-[20px] font-bold shadow-sm text-sm flex-shrink-0"
        @click="emit('launch')"
      >
        {{ t('status_card.launch') }}
      </button>

      <button 
        v-if="status === 'installed'"
        class="btn-cute flex-1 flex items-center justify-center gap-1 bg-surface hover:bg-red-50 text-text hover:text-red-500 border-2 border-border-soft hover:border-red-300 py-2 px-2 rounded-[20px] font-bold shadow-sm text-sm flex-shrink-0"
        @click="emit('uninstall')"
      >
        <Trash2 class="w-4 h-4" />
        {{ t('status_card.uninstall') }}
      </button>
      
      <!-- Progress Bar Area -->
      <div
        v-if="status === 'installing'"
        class="flex-1 w-full flex flex-col gap-2 bg-surface p-4 rounded-[20px] border-2 border-border-soft shadow-inner"
      >
        <div class="flex justify-between text-xs font-bold text-text">
          <span class="truncate pr-2">{{ progressMessage || t('status_card.processing') }}</span>
          <span>{{ progress ? progress.toFixed(1) : 0 }}%</span>
        </div>
        <div class="h-2 w-full bg-indigo-50 rounded-full overflow-hidden shadow-inner">
          <div 
            class="h-full bg-gradient-to-r from-indigo-400 to-indigo-500 rounded-full transition-all duration-300"
            :style="{ width: `${progress || 0}%` }"
          />
        </div>
      </div>
      
      <button 
        v-else-if="status === 'checking'"
        disabled
        class="flex-1 flex items-center justify-center gap-2 bg-surface text-indigo-600/60 py-3 px-4 rounded-[20px] font-bold cursor-not-allowed border-2 border-border-soft"
      >
        <Loader2 class="w-5 h-5 animate-spin" />
        {{ t('status_card.sniffing') }}
      </button>
    </div>
  </div>
</template>
