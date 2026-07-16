<template>
  <img
    :src="imgSrc"
    :class="customClass"
    referrerpolicy="no-referrer"
    @error="handleImgError"
  >
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { isTauri, invoke } from '@tauri-apps/api/core';
import vrchatImg from '../assets/vrchat.png';
import { isDebugLogEnabled } from '../api/debugConfig';

const props = withDefaults(defineProps<{
  user?: any;
  url?: string;
  customClass?: string;
}>(), {
  user: null,
  url: '',
  customClass: 'w-full h-full object-cover'
});

const imgSrc = ref<string>(vrchatImg);

// 内存缓存：同一 URL 不重复请求
const cache = new Map<string, string>();
const failedAt = new Map<string, number>();
const FAILED_RETRY_MS = 60000;

const isRecentFailure = (url: string) => {
  const ts = failedAt.get(url);
  if (!ts) return false;
  if (Date.now() - ts > FAILED_RETRY_MS) {
    failedAt.delete(url);
    return false;
  }
  return true;
};

const errorText = (err: unknown) => String((err as any)?.message || err || 'unknown error');

const getAvatarUrl = (user: any) => {
  if (!user) return '';
  return user.userIcon
    || user.profilePicOverrideThumbnail
    || user.profilePicOverride
    || user.thumbnailUrl
    || user.currentAvatarThumbnailImageUrl
    || user.currentAvatarImageUrl
    || user.imageUrl
    || user.thumbnailImageUrl
    || user.fallbackAvatar
    || '';
};

const loadImage = async () => {
  const targetUrl = props.url || getAvatarUrl(props.user);
  if (!targetUrl) {
    imgSrc.value = vrchatImg;
    return;
  }

  // 本地资源或 data URI 直接用
  if (targetUrl.startsWith('data:') || targetUrl.startsWith('/') || targetUrl.startsWith('./')) {
    imgSrc.value = targetUrl;
    return;
  }

  // 命中缓存直接返回
  if (cache.has(targetUrl)) {
    imgSrc.value = cache.get(targetUrl)!;
    return;
  }

  if (isRecentFailure(targetUrl)) {
    imgSrc.value = vrchatImg;
    return;
  }

  // 转换 file 路径为 image 路径 (参考 VrcDog 实现)
  let finalUrl = targetUrl;
  const pattern = /file\/file_([a-f0-9-]+)\/(\d+)(\/file)?\/?$/;
  const match = targetUrl.match(pattern);
  if (match) {
    const fileId = match[1];
    const version = match[2];
    
    let endpointDomain = 'https://api.vrchat.cloud/api/1';
    if (targetUrl.startsWith('https://')) {
      const urlObj = new URL(targetUrl);
      endpointDomain = `${urlObj.protocol}//${urlObj.host}${urlObj.pathname.includes('/api/1/') ? '/api/1' : ''}`;
    }
    
    finalUrl = `${endpointDomain}/image/file_${fileId}/${version}/256`;
  }

  try {
    if (isTauri()) {
      // 使用 Tauri 后端代理解析图片，解决 VRChat 防盗链和跨域问题
      const { DbApi } = await import('../api');
      const authCookie = await DbApi.getAuth();
      const base64Data = await invoke<string>('vrc_get_image_bytes', { 
        url: finalUrl,
        authCookie: authCookie || null 
      });
      imgSrc.value = base64Data;
      cache.set(targetUrl, base64Data);
      failedAt.delete(targetUrl);
    } else {
      imgSrc.value = finalUrl;
      cache.set(targetUrl, finalUrl);
      failedAt.delete(targetUrl);
    }
  } catch (err) {
    failedAt.set(targetUrl, Date.now());
    if (isDebugLogEnabled()) {
      console.warn('[VrcAvatar] Image fallback:', errorText(err));
    }
    imgSrc.value = vrchatImg;
  }
};

onMounted(() => loadImage());
watch(() => [props.user, props.url], () => loadImage(), { deep: true });

const handleImgError = (e: any) => {
  if (e.target.src !== vrchatImg) e.target.src = vrchatImg;
};
</script>
