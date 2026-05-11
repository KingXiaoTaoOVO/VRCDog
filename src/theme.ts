import { ref, computed } from 'vue';

// Define theme configs
export const themes = {
  dog: {
    id: 'dog',
    name: '橙黄色小狗风格',
    logo: new URL('./assets/dog.jpg', import.meta.url).href,
    appTitle: 'VrcDog',
    colors: {
      bgMain: '#fffbeb', // amber-50
      blob1: 'rgba(253, 230, 138, 0.6)', // amber-200
      blob2: 'rgba(254, 215, 170, 0.6)', // orange-200
      borderSoft: 'rgba(255, 255, 255, 0.4)', // glass soft
      borderStrong: 'rgba(255, 255, 255, 0.8)', // glass strong
      textStrong: '#78350f', // amber-900
      textSoft: '#92400e', // amber-800
      activeBg: 'rgba(254, 243, 199, 0.5)',
      primaryBtnBg: '#f59e0b', // amber-500
      primaryBtnHover: '#d97706', // amber-600
    }
  },
  cat: {
    id: 'cat',
    name: '黄绿色小猫风格',
    logo: new URL('./assets/main.png', import.meta.url).href,
    appTitle: 'VrcCat',
    colors: {
      bgMain: '#f7fee7', // lime-50
      blob1: 'rgba(217, 249, 157, 0.6)', // lime-200
      blob2: 'rgba(167, 243, 208, 0.6)', // emerald-200
      borderSoft: 'rgba(255, 255, 255, 0.4)',
      borderStrong: 'rgba(255, 255, 255, 0.8)',
      textStrong: '#365314', // lime-900
      textSoft: '#4d7c0f', // lime-700
      activeBg: 'rgba(236, 252, 203, 0.5)',
      primaryBtnBg: '#84cc16', // lime-500
      primaryBtnHover: '#65a30d', // lime-600
    }
  },
  helmet: {
    id: 'helmet',
    name: '粉红色头盔风格',
    logo: new URL('./assets/helmet.jpeg', import.meta.url).href,
    appTitle: 'VrcArai',
    colors: {
      bgMain: '#fdf2f8', // pink-50
      blob1: 'rgba(251, 207, 232, 0.6)', // pink-200
      blob2: 'rgba(254, 205, 211, 0.6)', // rose-200
      borderSoft: 'rgba(255, 255, 255, 0.4)',
      borderStrong: 'rgba(255, 255, 255, 0.8)',
      textStrong: '#831843', // pink-900
      textSoft: '#9d174d', // pink-800
      activeBg: 'rgba(252, 231, 243, 0.5)',
      primaryBtnBg: '#ec4899', // pink-500
      primaryBtnHover: '#db2777', // pink-600
    }
  },
  mono: {
    id: 'mono',
    name: '简约黑白风格',
    logo: new URL('./assets/mono.jpeg', import.meta.url).href,
    appTitle: 'VrcMono',
    colors: {
      bgMain: '#f9fafb', // gray-50
      blob1: 'rgba(229, 231, 235, 0.6)', // gray-200
      blob2: 'rgba(209, 213, 219, 0.6)', // gray-300
      borderSoft: 'rgba(255, 255, 255, 0.4)',
      borderStrong: 'rgba(255, 255, 255, 0.8)',
      textStrong: '#111827', // gray-900
      textSoft: '#4b5563', // gray-600
      activeBg: 'rgba(243, 244, 246, 0.5)',
      primaryBtnBg: '#374151',
      primaryBtnHover: '#1f2937',
    }
  }
};

export type ThemeId = keyof typeof themes;

import { useStorage } from '@vueuse/core';

export const currentThemeId = useStorage<ThemeId>('app-theme', 'dog');

// Ensure the loaded theme actually exists, fallback if not
if (!themes[currentThemeId.value]) {
  currentThemeId.value = 'dog';
}

export const currentTheme = computed(() => themes[currentThemeId.value]);

export const setTheme = (id: ThemeId) => {
  if (themes[id]) {
    currentThemeId.value = id;
  }
};
