<script setup lang="ts">
import { ref, computed } from 'vue';
import { X, FolderOpen, CheckCircle2 } from 'lucide-vue-next';
import vrchatImg from '../assets/vrchat.png';
import { open } from '@tauri-apps/plugin-dialog';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const props = defineProps<{
  show: boolean;
  title: string;
  isVccSelection?: boolean;
  vccInstalled?: boolean;
  alcomInstalled?: boolean;
}>();

const emit = defineEmits(['close', 'confirm', 'uninstall']);

const installPath = ref('C:\\Program Files\\');
const autoDeleteInstaller = ref(true);
const selectedTool = ref<'vcc' | 'alcom'>('vcc');

const isCurrentToolInstalled = computed(() => {
  if (selectedTool.value === 'vcc') return props.vccInstalled;
  if (selectedTool.value === 'alcom') return props.alcomInstalled;
  return false;
});

const handleConfirm = () => {
  if (props.isVccSelection && isCurrentToolInstalled.value) {
    emit('uninstall', selectedTool.value);
    return;
  }
  emit('confirm', {
    path: installPath.value,
    autoDelete: autoDeleteInstaller.value,
    tool: props.isVccSelection ? selectedTool.value : undefined
  });
};

const selectFolder = async () => {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: t('install_dialog.select_dir')
    });
    if (selected && typeof selected === 'string') {
      // 确保路径以斜杠结尾，符合直觉
      installPath.value = selected.endsWith('\\') || selected.endsWith('/') ? selected : selected + '\\';
    }
  } catch (err) {
    console.error("无法打开文件浏览器:", err);
  }
};
</script>

<template>
  <Teleport to="body">
    <Transition name="fade">
      <div
        v-if="show"
        class="fixed inset-0 z-50 flex items-center justify-center p-4"
      >
        <!-- Backdrop -->
        <div
          class="absolute inset-0 bg-slate-900/40 backdrop-blur-sm"
          @click="emit('close')"
        />
        
        <!-- Dialog -->
        <div class="bg-white/95 backdrop-blur-xl w-full max-w-md rounded-[32px] shadow-2xl relative z-10 transform transition-all p-8 border-4 border-white">
          <div class="flex justify-between items-center mb-6">
            <h2 class="text-2xl font-extrabold text-slate-900">
              {{ title }}
            </h2>
            <button
              class="btn-cute p-2 rounded-full hover:bg-indigo-50 text-slate-600 transition-colors"
              @click="emit('close')"
            >
              <X class="w-6 h-6" />
            </button>
          </div>
          
          <div class="space-y-6">
            <!-- VCC vs ALCOM Selection -->
            <div
              v-if="isVccSelection"
              class="space-y-3"
            >
              <label class="block text-sm font-bold text-slate-800 mb-2">{{ t('install_dialog.choose_toy') }}</label>
              <div class="grid grid-cols-2 gap-3">
                <button 
                  class="btn-cute p-4 rounded-[20px] border-2 transition-all text-left bg-white"
                  :class="selectedTool === 'vcc' ? 'border-indigo-400 bg-slate-50 shadow-md shadow-indigo-200' : 'border-slate-200 hover:border-indigo-300'"
                  @click="selectedTool = 'vcc'"
                >
                  <h4 class="font-black text-lg flex items-center justify-between mb-1">
                    <div class="flex items-center gap-2">
                      <img
                        :src="vrchatImg"
                        class="w-6 h-6 object-contain drop-shadow-sm"
                      >
                      VCC
                    </div>
                    <span
                      v-if="props.vccInstalled"
                      class="text-[10px] bg-green-100 text-green-700 px-2 py-0.5 rounded-full border border-green-200"
                    >{{ t('install_dialog.installed') }}</span>
                  </h4>
                  <p
                    class="text-xs mt-1 transition-colors"
                    :class="selectedTool === 'vcc' ? 'text-slate-800' : 'text-stone-500'"
                  >
                    {{ t('install_dialog.vcc_desc') }}
                  </p>
                </button>
                <button 
                  class="btn-cute p-4 rounded-[20px] border-2 transition-all text-left bg-white"
                  :class="selectedTool === 'alcom' ? 'border-indigo-400 bg-slate-50 shadow-md shadow-indigo-200' : 'border-slate-200 hover:border-indigo-300'"
                  @click="selectedTool = 'alcom'"
                >
                  <h4 class="font-black text-lg flex items-center justify-between mb-1">
                    <div class="flex items-center gap-2">
                      <img
                        src="https://vrc-get.anatawa12.com/resources/alcom.svg"
                        class="w-6 h-6 object-contain drop-shadow-sm"
                      >
                      ALCOM
                    </div>
                    <span
                      v-if="props.alcomInstalled"
                      class="text-[10px] bg-green-100 text-green-700 px-2 py-0.5 rounded-full border border-green-200"
                    >{{ t('install_dialog.installed') }}</span>
                  </h4>
                  <p
                    class="text-xs mt-1 transition-colors"
                    :class="selectedTool === 'alcom' ? 'text-slate-800' : 'text-stone-500'"
                  >
                    {{ t('install_dialog.alcom_desc') }}
                  </p>
                </button>
              </div>
            </div>
            
            <!-- Path Selection -->
            <div>
              <label class="block text-sm font-bold text-slate-800 mb-2">{{ t('install_dialog.location') }}</label>
              <div class="flex gap-2">
                <input 
                  v-model="installPath"
                  type="text" 
                  class="flex-1 bg-slate-50 border-2 border-slate-200 rounded-[20px] px-4 py-3 text-slate-900 font-medium focus:outline-none focus:border-indigo-400 focus:bg-white transition-colors"
                >
                <button 
                  class="btn-cute px-5 py-3 bg-white hover:bg-indigo-50 rounded-[20px] border-2 border-slate-200 text-indigo-600 font-bold shadow-sm transition-colors flex items-center justify-center"
                  @click="selectFolder"
                >
                  <FolderOpen class="w-5 h-5" />
                </button>
              </div>
            </div>
            
            <!-- Options -->
            <div class="bg-slate-50 p-4 rounded-[20px] border-2 border-slate-200">
              <label class="flex items-center gap-3 cursor-pointer group">
                <div class="relative flex items-center justify-center w-6 h-6 rounded-lg border-2 border-indigo-300 bg-white group-hover:border-slate-1000 transition-colors">
                  <input
                    v-model="autoDeleteInstaller"
                    type="checkbox"
                    class="peer sr-only"
                  >
                  <div class="peer-checked:bg-indigo-500 absolute inset-0 rounded-[6px] opacity-0 peer-checked:opacity-100 transition-opacity flex items-center justify-center m-[2px]">
                    <CheckCircle2 class="w-4 h-4 text-white" />
                  </div>
                </div>
                <span class="text-sm font-bold text-slate-800 group-hover:text-slate-900 transition-colors">{{ t('install_dialog.auto_delete') }}</span>
              </label>
            </div>
          </div>
          
          <!-- Actions -->
          <div class="flex gap-4 mt-8">
            <button
              class="flex-1 py-3 px-6 rounded-2xl font-bold bg-slate-50 hover:bg-indigo-50 text-slate-600 transition-colors"
              @click="emit('close')"
            >
              {{ t('install_dialog.cancel') }}
            </button>
            
            <button
              v-if="props.isVccSelection && isCurrentToolInstalled"
              class="flex-1 py-3 px-6 rounded-2xl font-bold bg-red-500 hover:bg-red-600 text-white shadow-lg shadow-red-500/30 transition-transform hover:scale-105 active:scale-95 flex items-center justify-center gap-2"
              @click="emit('uninstall', selectedTool)"
            >
              {{ t('install_dialog.uninstall') }}
            </button>
            <button
              v-else
              class="flex-1 py-3 px-6 rounded-2xl font-bold bg-indigo-500 hover:bg-indigo-600 text-white shadow-lg shadow-indigo-500/30 transition-transform hover:scale-105 active:scale-95 flex items-center justify-center gap-2"
              @click="handleConfirm"
            >
              {{ t('install_dialog.install') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
.fade-enter-active > div:nth-child(2) {
  transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.fade-enter-from > div:nth-child(2) {
  opacity: 0;
  transform: translateY(30px) scale(0.9);
}
.fade-leave-active > div:nth-child(2) {
  transition: all 0.2s ease-in;
}
.fade-leave-to > div:nth-child(2) {
  opacity: 0;
  transform: scale(0.95);
}
</style>
