<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { Image as ImageIcon, RefreshCcw, Clock, FileWarning, Eye, Download, Copy, FolderOpen } from 'lucide-vue-next';
import { GalleryApi, SysApi } from '../api';
import { convertFileSrc } from '@tauri-apps/api/core';
import BaseModal from './BaseModal.vue';
import { useI18n } from 'vue-i18n';
import type { GalleryImage } from '../types/vrc';
import { useVirtualList, useElementSize } from '@vueuse/core';

const { t } = useI18n();

const images = ref<GalleryImage[]>([]);
const loading = ref(true);
const loadingMore = ref(false);
const errorMsg = ref('');
const offset = ref(0);
const limit = 40;
const hasMore = ref(true);

const fetchImages = async (reset = false) => {
  if (reset) {
    loading.value = true;
    offset.value = 0;
    images.value = [];
    hasMore.value = true;
  } else {
    if (!hasMore.value || loadingMore.value) return;
    loadingMore.value = true;
  }
  
  errorMsg.value = '';
  try {
    const res = await GalleryApi.getImages({ limit, offset: offset.value });
    
    if (res.length < limit) {
      hasMore.value = false;
    }
    
    const newImages = res.map((img: GalleryImage) => ({
      ...img,
      assetUrl: convertFileSrc(img.path),
      dateStr: new Date(img.created_at * 1000).toLocaleString()
    }));
    
    if (reset) {
      images.value = newImages;
    } else {
      images.value.push(...newImages);
    }
    
    offset.value += newImages.length;
  } catch (err: any) {
    errorMsg.value = err.message || err;
  } finally {
    loading.value = false;
    loadingMore.value = false;
  }
};

const handleScroll = () => {
  // Trigger infinite load when nearing bottom
  if (!containerProps.ref.value) return;
  const el = containerProps.ref.value;
  if (el.scrollTop + el.clientHeight >= el.scrollHeight - 600) {
    fetchImages(false);
  }
};

// --- Virtual Scroll Setup ---
const containerRef = ref<HTMLElement | null>(null);
const { width } = useElementSize(containerRef);

const cols = computed(() => {
  if (width.value >= 1280) return 4;
  if (width.value >= 768) return 3;
  return 2;
});

const rowImages = computed(() => {
  const rows = [];
  for (let i = 0; i < images.value.length; i += cols.value) {
    rows.push({ id: i, items: images.value.slice(i, i + cols.value) });
  }
  return rows;
});

// VRChat screenshots are typically 16:9
const rowHeight = computed(() => {
  if (width.value === 0) return 200; // fallback
  const itemWidth = (width.value - (cols.value - 1) * 16) / cols.value; // 16px gap
  return (itemWidth * 9) / 16 + 16; // height + 16px bottom gap
});

const { list, containerProps, wrapperProps } = useVirtualList(rowImages, {
  itemHeight: () => rowHeight.value,
  overscan: 4
});


onMounted(() => fetchImages(true));

// Preview Modal
const previewImage = ref<GalleryImage | null>(null);

const copyPath = async (path: string) => {
  try {
    await navigator.clipboard.writeText(path);
    alert(t('gallery.copied_alert'));
  } catch (e) {
    console.warn(e);
  }
};

const openInExplorer = async (path: string) => {
  try {
    await SysApi.showInExplorer({ path });
  } catch (e) {
    console.warn(e);
  }
};
</script>

<template>
  <div class="h-full flex flex-col">
    <header class="mb-6 flex justify-between items-end">
      <div>
        <h1 class="text-3xl font-extrabold text-[#451a03] tracking-tight flex items-center gap-3">
          {{ t('gallery.title') }}
          <span class="inline-flex items-center justify-center p-1.5 bg-purple-100 rounded-xl">
            <ImageIcon class="w-6 h-6 text-purple-600" />
          </span>
        </h1>
        <p class="text-amber-700/80 font-medium mt-1">
          {{ t('gallery.subtitle') }}
        </p>
      </div>
      <button
        class="px-4 py-2 bg-white rounded-full text-purple-700 font-bold border border-purple-200 shadow-sm hover:shadow-md transition-all flex items-center gap-2"
        @click="fetchImages(true)"
      >
        <RefreshCcw
          class="w-4 h-4"
          :class="{'animate-spin': loading}"
        /> {{ t('gallery.refresh') }}
      </button>
    </header>

    <div
      ref="containerRef"
      class="flex-1 bg-white/60 backdrop-blur-md border-2 border-white rounded-3xl p-6 shadow-lg overflow-hidden flex flex-col"
    >
      <div
        v-if="loading && images.length === 0"
        class="h-full flex flex-col items-center justify-center text-purple-500 opacity-70"
      >
        <ImageIcon
          class="animate-bounce mb-4"
          :size="48"
        />
        <p class="font-bold">
          {{ t('gallery.scanning') }}
        </p>
      </div>

      <div
        v-else-if="errorMsg"
        class="bg-red-50 text-red-600 p-4 rounded-2xl border border-red-200 text-center font-bold text-sm"
      >
        {{ errorMsg }}
      </div>

      <div
        v-else-if="images.length === 0"
        class="h-full flex flex-col items-center justify-center text-purple-900/50"
      >
        <FileWarning
          class="mb-4"
          :size="48"
        />
        <p class="font-bold text-lg">
          {{ t('gallery.no_images') }}
        </p>
        <p class="text-sm mt-1">
          {{ t('gallery.no_images_desc') }}
        </p>
      </div>

      <!-- Virtualized Grid -->
      <div
        v-else
        v-bind="containerProps"
        class="flex-1 overflow-y-auto custom-scrollbar pr-2"
        @scroll="handleScroll"
      >
        <div v-bind="wrapperProps">
          <div
            v-for="row in list"
            :key="row.data.id"
            class="grid gap-4 mb-4"
            :style="{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }"
          >
            <div
              v-for="img in row.data.items"
              :key="img.path"
              class="relative group rounded-2xl overflow-hidden shadow-sm hover:shadow-xl transition-all border-4 border-white bg-gray-100 cursor-pointer h-full"
              @click="previewImage = img"
            >
              <img
                :src="img.assetUrl"
                loading="lazy"
                class="w-full h-full object-cover transform group-hover:scale-105 transition-transform duration-500"
              >
              
              <div class="absolute inset-0 bg-gradient-to-t from-black/80 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity flex flex-col justify-end p-4">
                <h4 class="text-white font-bold text-xs truncate drop-shadow-md mb-1">
                  {{ img.name }}
                </h4>
                <div class="flex items-center gap-3 text-white/80 text-[10px]">
                  <span class="flex items-center gap-1"><Clock :size="10" /> {{ img.dateStr }}</span>
                  <span class="flex items-center gap-1 font-mono">{{ (img.size / 1024 / 1024).toFixed(1) }} MB</span>
                </div>
              </div>

              <div class="absolute top-3 right-3 opacity-0 group-hover:opacity-100 transition-opacity">
                <button class="p-2 bg-white/20 backdrop-blur-md hover:bg-white/40 text-white rounded-full transition-colors">
                  <Eye :size="14" />
                </button>
              </div>
            </div>
          </div>
        </div>

        <div
          v-if="loadingMore"
          class="py-6 flex justify-center text-purple-500 w-full"
        >
          <RefreshCcw
            class="animate-spin"
            :size="24"
          />
        </div>
        <div
          v-else-if="!hasMore"
          class="py-12 text-center text-purple-400 font-bold text-sm w-full"
        >
          {{ t('gallery.end_of_list', { count: images.length }) }}
        </div>
      </div>
    </div>

    <!-- 图片大图预览 Modal -->
    <BaseModal
      :show="!!previewImage"
      @close="previewImage = null"
    >
      <template v-if="previewImage">
        <div class="bg-black relative rounded-t-3xl overflow-hidden flex items-center justify-center min-h-[50vh]">
          <img
            :src="previewImage.assetUrl"
            class="max-w-full max-h-[70vh] object-contain"
          >
          <button
            class="absolute top-4 right-4 p-2 rounded-full bg-black/40 hover:bg-black/60 text-white backdrop-blur"
            @click="previewImage = null"
          >
            ✕
          </button>
        </div>
        <div class="p-6 bg-white rounded-b-3xl">
          <h2 class="text-xl font-extrabold text-[#451a03] mb-2 truncate">
            {{ previewImage.name }}
          </h2>
          <div class="flex items-center gap-4 text-sm text-gray-500 mb-6 font-bold">
            <span class="flex items-center gap-1"><Clock :size="14" /> {{ previewImage.dateStr }}</span>
            <span class="flex items-center gap-1"><FileWarning :size="14" /> {{ (previewImage.size / 1024 / 1024).toFixed(2) }} MB</span>
          </div>
          
          <div class="flex items-center justify-end gap-3">
            <button
              class="px-5 py-2.5 bg-gray-100 hover:bg-gray-200 text-gray-700 font-bold rounded-xl transition-colors flex items-center gap-2"
              @click="openInExplorer(previewImage.path)"
            >
              <FolderOpen :size="16" /> 在资源管理器中打开
            </button>
            <button
              class="px-5 py-2.5 bg-gray-100 hover:bg-gray-200 text-gray-700 font-bold rounded-xl transition-colors flex items-center gap-2"
              @click="copyPath(previewImage.path)"
            >
              <Copy :size="16" /> {{ t('gallery.copy_path') }}
            </button>
          </div>
        </div>
      </template>
    </BaseModal>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(168, 85, 247, 0.2); border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(168, 85, 247, 0.4); }
</style>
