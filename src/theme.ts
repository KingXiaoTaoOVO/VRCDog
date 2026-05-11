import { ref, computed } from 'vue';

interface Theme {
  id: string;
  name: string;
  logo?: string;
  appTitle?: string;
  colors: {
    bgMain: string;
    surface: string;
    surfaceHover: string;
    blob1: string;
    blob2: string;
    borderSoft: string;
    borderStrong: string;
    text: string;      // 基础文字颜色 (Standard text)
    textStrong: string; // 标题/强调 (Headers)
    textSoft: string;   // 柔和文字 (Subtitles)
    textMuted: string;  // 灰暗文字 (Disabled/Metadata)
    activeBg: string;
    primaryBtnBg: string;
    primaryBtnHover: string;
    terminalBg: string;
    glassEffect: string;
  };
}

export type ThemeId = 'dog' | 'cat' | 'helmet' | 'mono';

/**
 * [Aesthetics] 极简高斯模糊亚克力设计系统
 * 颜色基于 Logo 图片提取，确保视觉连贯性
 * 柔和舒适的配色，避免高频白光，保护视力
 */
export const themes: Record<ThemeId, Theme> = {
  dog: {
    id: 'dog',
    name: '小狗',
    logo: new URL('./assets/dog.jpg', import.meta.url).href,
    appTitle: 'VrcDog',
    colors: {
      bgMain: '#faf7ed', 
      surface: 'rgba(255, 252, 240, 0.5)', 
      surfaceHover: 'rgba(255, 252, 240, 0.75)',
      blob1: 'rgba(245, 158, 11, 0.35)',
      blob2: 'rgba(251, 191, 36, 0.35)',
      borderSoft: 'rgba(120, 53, 15, 0.1)',
      borderStrong: 'rgba(120, 53, 15, 0.2)',
      text: '#5d4037',      // 柔和棕黑
      textStrong: '#451a03', // 深焦糖
      textSoft: '#8d6e63',   // 浅褐
      textMuted: '#a1887f',  // 灰褐
      activeBg: 'rgba(251, 191, 36, 0.25)',
      primaryBtnBg: '#d97706',
      primaryBtnHover: '#b45309',
      terminalBg: 'rgba(20, 10, 0, 0.9)',
      glassEffect: 'blur(24px) saturate(180%)',
    }
  },
  cat: {
    id: 'cat',
    name: '小猫',
    logo: new URL('./assets/main.png', import.meta.url).href,
    appTitle: 'VrcCat',
    colors: {
      bgMain: '#f0fdf4', 
      surface: 'rgba(240, 253, 244, 0.5)', 
      surfaceHover: 'rgba(240, 253, 244, 0.75)',
      blob1: 'rgba(34, 197, 94, 0.3)',
      blob2: 'rgba(20, 184, 166, 0.3)',
      borderSoft: 'rgba(6, 78, 59, 0.1)',
      borderStrong: 'rgba(6, 78, 59, 0.2)',
      text: '#2d4a3e',      // 柔和森绿
      textStrong: '#064e3b', // 深林绿
      textSoft: '#4d7c6b',   // 浅绿
      textMuted: '#709d8d',  // 灰绿
      activeBg: 'rgba(20, 184, 166, 0.2)',
      primaryBtnBg: '#059669',
      primaryBtnHover: '#047857',
      terminalBg: 'rgba(0, 20, 10, 0.9)',
      glassEffect: 'blur(24px) saturate(180%)',
    }
  },
  helmet: {
    id: 'helmet',
    name: '头盔',
    logo: new URL('./assets/helmet.jpeg', import.meta.url).href,
    appTitle: 'VrcArai',
    colors: {
      bgMain: '#fff1f2', 
      surface: 'rgba(255, 241, 242, 0.5)', 
      surfaceHover: 'rgba(255, 241, 242, 0.75)',
      blob1: 'rgba(244, 63, 94, 0.3)',
      blob2: 'rgba(217, 70, 239, 0.3)',
      borderSoft: 'rgba(136, 19, 55, 0.1)',
      borderStrong: 'rgba(136, 19, 55, 0.2)',
      text: '#6b3e4a',      // 柔和暗玫
      textStrong: '#4c0519', // 深玫瑰
      textSoft: '#a67b86',   // 浅玫
      textMuted: '#c5aeb4',  // 灰玫
      activeBg: 'rgba(244, 114, 182, 0.2)',
      primaryBtnBg: '#e11d48',
      primaryBtnHover: '#be123c',
      terminalBg: 'rgba(20, 0, 5, 0.9)',
      glassEffect: 'blur(24px) saturate(180%)',
    }
  },
  mono: {
    id: 'mono',
    name: '黑白',
    logo: new URL('./assets/mono.jpeg', import.meta.url).href,
    appTitle: 'VrcMono',
    colors: {
      bgMain: '#f8fafc', 
      surface: 'rgba(248, 250, 252, 0.5)', 
      surfaceHover: 'rgba(248, 250, 252, 0.75)',
      blob1: 'rgba(100, 116, 139, 0.3)',
      blob2: 'rgba(148, 163, 184, 0.3)',
      borderSoft: 'rgba(15, 23, 42, 0.1)',
      borderStrong: 'rgba(15, 23, 42, 0.2)',
      text: '#475569',      // 柔和板岩灰
      textStrong: '#0f172a', // 深岩黑
      textSoft: '#64748b',   // 板岩中灰
      textMuted: '#94a3b8',  // 板岩浅灰
      activeBg: 'rgba(148, 163, 184, 0.2)',
      primaryBtnBg: '#475569',
      primaryBtnHover: '#334155',
      terminalBg: 'rgba(10, 10, 10, 0.95)',
      glassEffect: 'blur(30px) saturate(150%)',
    }
  }
};

import { useStorage } from '@vueuse/core';

export const currentThemeId = useStorage<ThemeId>('app-theme', 'dog');

if (!themes[currentThemeId.value]) {
  currentThemeId.value = 'dog';
}

export const currentTheme = computed(() => themes[currentThemeId.value]);

export const setTheme = (id: ThemeId) => {
  if (themes[id]) {
    currentThemeId.value = id;
  }
};
