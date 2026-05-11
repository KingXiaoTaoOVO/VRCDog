<script setup lang="ts">
import { useToast } from "../composables/useToast";

const toast = useToast();
import { ref, onMounted, computed } from 'vue';
import { Image as ImageIcon, Images, RefreshCcw, Clock, FileWarning, Eye, Download, Copy, FolderOpen, Trash2 } from 'lucide-vue-next';
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
    toast.info(t('gallery.copied_alert'));
  } catch (e) {
    console.warn(e);
  }
};

const deleteImage = async (img: GalleryImage) => {
  if (!confirm(t('auto_8259e21e'))) return;
  try {
    await GalleryApi.deleteImage({ path: img.path });
    previewImage.value = null;
    images.value = images.value.filter(i => i.path !== img.path);
  } catch (err) {
    console.error('Failed to delete image:', err);
    toast.error(t('auto_282947e8'));
  }
};

const openInExplorer = async (path: string) => {
  try {
    await SysApi.showInExplorer({ path });
  } catch (e) {
    console.warn(e);
  }
};

import { VrcApi } from '../api';

const uploadingToVrcPlus = ref(false);

const uploadToVrcPlus = async () => {
  if (!previewImage.value || !previewImage.value.assetUrl || uploadingToVrcPlus.value) return;
  uploadingToVrcPlus.value = true;
  try {
    const res = await fetch(previewImage.value.assetUrl);
    const blob = await res.blob();
    const reader = new FileReader();
    reader.readAsDataURL(blob);
    reader.onloadend = async () => {
      try {
        const base64data = reader.result as string;
        await VrcApi.uploadVrcPlusImage(base64data, 'gallery');
        toast.info(t('auto_e1afe2ef'));
      } catch (err: any) {
        toast.error(t('auto_706254d1') + (err.message || err));
      } finally {
        uploadingToVrcPlus.value = false;
      }
    };
  } catch (err: any) {
    toast.error(t('auto_4c368c6a') + (err.message || err));
    uploadingToVrcPlus.value = false;
  }
};
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-surface-hover rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-indigo-500/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <header class="mb-8 flex justify-between items-end shrink-0 z-10">
      <div>
        <h1 class="text-3xl font-extrabold text-text tracking-tight flex items-center gap-3">
          <span class="inline-flex items-center justify-center p-2 bg-indigo-100 rounded-2xl shadow-sm border border-indigo-200/50">
            <Images class="w-6 h-6 text-indigo-600" />
          </span>
          {{ t('gallery.title') }}
        </h1>
      </div>
      <button
        class="px-5 py-2.5 bg-surface rounded-xl text-text-muted font-bold border border-border-soft shadow-sm hover:shadow-md hover:text-indigo-600 hover:border-indigo-200 transition-all flex items-center gap-2"
        @click="fetchImages(true)"
      >
        <RefreshCcw
          class="w-5 h-5"
          :class="{'animate-spin text-indigo-600': loading}"
        /> {{ t('gallery.refresh') }}
      </button>
    </header>

    <div
      ref="containerRef"
      class="flex-1 bg-surface backdrop-blur-xl border border-border-strong rounded-3xl p-6 shadow-lg shadow-slate-200/40 overflow-hidden flex flex-col z-10 relative"
    >
      <div
        v-if="loading && images.length === 0"
        class="h-full flex flex-col items-center justify-center text-indigo-500/80"
      >
        <Images
          class="animate-bounce mb-4"
          :size="48"
        />
        <p class="font-extrabold text-lg tracking-wide">
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
        class="h-full flex flex-col items-center justify-center text-border-strong"
      >
        <Images
          class="mb-4 opacity-30"
          :size="64"
        />
        <p class="font-bold text-xl text-text-muted">
          {{ t('gallery.no_images') }}
        </p>
        <p class="text-sm mt-2 font-medium">
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
              class="relative group rounded-2xl overflow-hidden shadow-sm hover:shadow-xl transition-all border border-border-soft hover:border-indigo-300 bg-background/10 cursor-pointer h-full"
              @click="previewImage = img"
            >
              <img
                :src="img.assetUrl"
                loading="lazy"
                class="w-full h-full object-cover transform group-hover:scale-[1.03] transition-transform duration-500"
              >
              
              <div class="absolute inset-0 bg-gradient-to-t from-slate-900/90 via-slate-900/20 to-transparent opacity-0 group-hover:opacity-100 transition-opacity flex flex-col justify-end p-4">
                <h4 class="text-white font-bold text-xs truncate drop-shadow-md mb-1.5">
                  {{ img.name }}
                </h4>
                <div class="flex items-center gap-3 text-text-muted text-[10px]">
                  <span class="flex items-center gap-1.5"><Clock :size="12" /> {{ img.dateStr }}</span>
                  <span class="flex items-center gap-1.5 font-mono"><FileWarning :size="12" /> {{ (img.size / 1024 / 1024).toFixed(1) }} MB</span>
                </div>
              </div>

              <div class="absolute top-3 right-3 opacity-0 group-hover:opacity-100 transition-opacity">
                <button class="p-2 bg-black/40 backdrop-blur-md hover:bg-black/60 text-white rounded-xl transition-colors">
                  <Eye :size="16" />
                </button>
              </div>
            </div>
          </div>
        </div>

        <div
          v-if="loadingMore"
          class="py-6 flex justify-center text-indigo-500 w-full"
        >
          <RefreshCcw
            class="animate-spin"
            :size="24"
          />
        </div>
        <div
          v-else-if="!hasMore"
          class="py-12 text-center text-border-strong font-bold text-sm w-full"
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
        <div class="bg-surface relative rounded-t-2xl overflow-hidden flex items-center justify-center min-h-[50vh]">
          <img
            :src="previewImage.assetUrl"
            class="max-w-full max-h-[70vh] object-contain"
          >
          <button
            class="absolute top-4 right-4 p-2 rounded-xl bg-black/40 hover:bg-black/60 text-white backdrop-blur transition-colors"
            @click="previewImage = null"
          >
            ✕
          </button>
        </div>
        <div class="p-6 bg-surface rounded-b-2xl">
          <h2 class="text-xl font-black text-text mb-2 truncate">
            {{ previewImage.name }}
          </h2>
          <div class="flex items-center gap-4 text-sm text-text-muted mb-6 font-bold">
            <span class="flex items-center gap-1"><Clock :size="14" /> {{ previewImage.dateStr }}</span>
            <span class="flex items-center gap-1"><FileWarning :size="14" /> {{ (previewImage.size / 1024 / 1024).toFixed(2) }} MB</span>
          </div>
          
          <div class="flex items-center justify-end gap-3 flex-wrap">
            <button
              class="px-5 py-2.5 bg-indigo-600 hover:bg-indigo-700 text-white font-bold rounded-xl transition-colors flex items-center gap-2 shadow-sm"
              :disabled="uploadingToVrcPlus"
              @click="uploadToVrcPlus"
            >
              <RefreshCcw
                v-if="uploadingToVrcPlus"
                class="animate-spin w-4 h-4"
              />
              <Images
                v-else
                class="w-4 h-4"
              />
              {{ uploadingToVrcPlus ? t('global.gallery.uploading') : t('global.gallery.upload_btn') }}
            </button>
            <button
              class="px-5 py-2.5 bg-background/10 hover:bg-background/20 text-text-muted font-bold rounded-xl transition-colors flex items-center gap-2"
              @click="openInExplorer(previewImage.path)"
            >
              <FolderOpen :size="16" /> 在资源管理器中打开
            </button>
            <button
              class="px-5 py-2.5 bg-background/10 hover:bg-background/20 text-text-muted font-bold rounded-xl transition-colors flex items-center gap-2"
              @click="copyPath(previewImage.path)"
            >
              <Copy :size="16" /> {{ t('gallery.copy_path') }}
            </button>
            <button
              class="px-5 py-2.5 bg-red-50 hover:bg-red-100 text-red-500 font-bold rounded-xl transition-colors flex items-center gap-2 ml-auto"
              @click="deleteImage(previewImage)"
            >
              <Trash2 :size="16" /> 删除 (Delete)
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
.custom-scrollbar::-webkit-scrollbar-thumb { background: #cbd5e1; border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: #94a3b8; }
</style>
