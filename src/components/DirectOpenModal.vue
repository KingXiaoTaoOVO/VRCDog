<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseModal from './BaseModal.vue';
import { VrcApi } from '../api';

const { t } = useI18n();

const show = ref(false);
const inputUrl = ref('');
const loading = ref(false);
const errorMsg = ref('');

const toggleModal = () => {
  show.value = !show.value;
  if (show.value) {
    inputUrl.value = '';
    errorMsg.value = '';
    loading.value = false;
    setTimeout(() => {
      const input = document.getElementById('direct-open-input');
      if (input) input.focus();
    }, 100);
  }
};

const handleGlobalKeyDown = (e: KeyboardEvent) => {
  if (e.ctrlKey && e.key.toLowerCase() === 'd') {
    e.preventDefault();
    toggleModal();
  }
};

onMounted(() => {
  window.addEventListener('keydown', handleGlobalKeyDown);
  window.addEventListener('vrc-direct-open-toggle', toggleModal);
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleGlobalKeyDown);
  window.removeEventListener('vrc-direct-open-toggle', toggleModal);
});

const submit = async () => {
  const q = inputUrl.value.trim();
  if (!q) return;

  loading.value = true;
  errorMsg.value = '';

  try {
    // Determine the type from the prefix
    let id = q;
    // Extract UUID if it's a full URL
    const match = q.match(/(usr|wrld|avtr|grp)_[a-fA-F0-9\-]+/);
    if (match) {
      id = match[0];
    } else {
      throw new Error(t('direct_open.invalid_id') || '无效的链接或 UUID 格式');
    }

    let detailData: any = null;
    let type = '';
    
    if (id.startsWith('usr_')) {
      detailData = await VrcApi.getUser({ userId: id });
      type = 'user';
    } else if (id.startsWith('wrld_')) {
      detailData = await VrcApi.getWorld({ worldId: id });
      type = 'world';
    } else if (id.startsWith('avtr_')) {
      detailData = await VrcApi.getAvatar({ avatarId: id });
      type = 'avatar';
    } else if (id.startsWith('grp_')) {
      throw new Error(t('direct_open.group_not_supported') || '群组解析暂不支持');
    }

    if (detailData) {
      // Emit an event to be picked up by SearchView or Dashboard
      window.dispatchEvent(new CustomEvent('vrc-open-detail', { 
        detail: { type, data: detailData }
      }));
      show.value = false;
    }
  } catch (err: any) {
    errorMsg.value = err.message || '获取信息失败';
  } finally {
    loading.value = false;
  }
};
</script>

<template>
  <BaseModal
    :show="show"
    :loading="loading"
    @close="show = false"
  >
    <div class="p-6 bg-gray-900 rounded-xl text-white w-96 max-w-full">
      <h2 class="text-xl font-bold mb-2">{{ t('direct_open.title') || '直接打开' }}</h2>
      <p class="text-xs text-gray-400 mb-4">{{ t('direct_open.desc') || '输入玩家 / 世界 / 房间 / 模型链接 或 UUID' }}</p>
      
      <input
        id="direct-open-input"
        v-model="inputUrl"
        type="text"
        class="w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm text-white focus:outline-none focus:border-white transition-colors mb-4"
        @keyup.enter="submit"
      >
      
      <p v-if="errorMsg" class="text-red-400 text-xs mb-4">{{ errorMsg }}</p>

      <div class="flex justify-end gap-3">
        <button
          class="px-5 py-2 bg-gray-800 hover:bg-gray-700 rounded-lg text-sm font-medium transition-colors"
          @click="show = false"
        >
          {{ t('direct_open.cancel') || '取消' }}
        </button>
        <button
          class="px-5 py-2 bg-white text-black hover:bg-gray-200 rounded-lg text-sm font-bold transition-colors disabled:opacity-50"
          :disabled="!inputUrl.trim() || loading"
          @click="submit"
        >
          {{ t('direct_open.confirm') || '确定' }}
        </button>
      </div>
    </div>
  </BaseModal>
</template>
