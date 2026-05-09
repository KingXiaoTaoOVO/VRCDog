import { ref, computed } from 'vue';

// Define theme configs
export const themes = {
  dog: {
    id: 'dog',
    name: '小狗风格',
    logo: new URL('./assets/dog.jpg', import.meta.url).href,
    appTitle: 'VrcDog',
    colors: {
      bgMain: '#fffbeb', // amber-50
      blob1: 'rgba(251, 207, 232, 0.4)', // pink-200/40
      blob2: 'rgba(253, 230, 138, 0.4)', // amber-200/40
      borderSoft: 'rgba(254, 243, 199, 1)', // amber-100
      borderStrong: 'rgba(253, 230, 138, 1)', // amber-200
      textStrong: '#78350f', // amber-900
      textSoft: '#b45309', // amber-700
      activeBg: 'rgba(254, 243, 199, 0.5)', // amber-100/50
      primaryBtnBg: '#6366f1', // indigo-500
      primaryBtnHover: '#4f46e5', // indigo-600
    }
  },
  cat: {
    id: 'cat',
    name: '小猫风格',
    logo: new URL('./assets/main.png', import.meta.url).href,
    appTitle: 'VrcCat',
    colors: {
      bgMain: '#f7fee7', // lime-50
      blob1: 'rgba(217, 249, 157, 0.4)', // lime-200/40
      blob2: 'rgba(167, 243, 208, 0.4)', // emerald-200/40
      borderSoft: 'rgba(236, 252, 203, 1)', // lime-100
      borderStrong: 'rgba(217, 249, 157, 1)', // lime-200
      textStrong: '#365314', // lime-900
      textSoft: '#4d7c0f', // lime-700
      activeBg: 'rgba(236, 252, 203, 0.5)', // lime-100/50
      primaryBtnBg: '#10b981', // emerald-500
      primaryBtnHover: '#059669', // emerald-600
    }
  },
  helmet: {
    id: 'helmet',
    name: '头盔风格',
    logo: new URL('./assets/helmet.jpeg', import.meta.url).href,
    appTitle: 'VrcArai',
    colors: {
      bgMain: '#fef2f2', // red-50
      blob1: 'rgba(254, 202, 202, 0.4)', // red-200/40
      blob2: 'rgba(226, 232, 240, 0.4)', // slate-200/40
      borderSoft: 'rgba(254, 226, 226, 1)', // red-100
      borderStrong: 'rgba(254, 202, 202, 1)', // red-200
      textStrong: '#7f1d1d', // red-900
      textSoft: '#b91c1c', // red-700
      activeBg: 'rgba(254, 226, 226, 0.5)', // red-100/50
      primaryBtnBg: '#ef4444', // red-500
      primaryBtnHover: '#dc2626', // red-600
    }
  },
  mono: {
    id: 'mono',
    name: '极简黑白',
    logo: new URL('./assets/mono.jpeg', import.meta.url).href,
    appTitle: 'VrcMono',
    colors: {
      bgMain: '#f3f4f6', // gray-100
      blob1: 'rgba(209, 213, 219, 0.4)', // gray-300/40
      blob2: 'rgba(229, 231, 235, 0.4)', // gray-200/40
      borderSoft: 'rgba(229, 231, 235, 1)', // gray-200
      borderStrong: 'rgba(156, 163, 175, 1)', // gray-400
      textStrong: '#111827', // gray-900
      textSoft: '#4b5563', // gray-600
      activeBg: 'rgba(209, 213, 219, 0.5)', // gray-300/50
      primaryBtnBg: '#374151', // gray-700
      primaryBtnHover: '#111827', // gray-900
    }
  }
};

export type ThemeId = keyof typeof themes;

import { useStorage } from '@vueuse/core';

export const currentThemeId = useStorage<ThemeId>('app-theme', 'helmet');

// Ensure the loaded theme actually exists, fallback if not
if (!themes[currentThemeId.value]) {
  currentThemeId.value = 'helmet';
}

export const currentTheme = computed(() => themes[currentThemeId.value]);

export const setTheme = (id: ThemeId) => {
  if (themes[id]) {
    currentThemeId.value = id;
  }
};
