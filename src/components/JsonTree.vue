<script setup lang="ts">
import { ref, computed } from 'vue';

const props = defineProps<{
  data: any;
  depth: number;
  keyName?: string;
}>();

const collapsed = ref(props.depth > 1); // 默认展开前2层

const isObject = computed(() => props.data !== null && typeof props.data === 'object' && !Array.isArray(props.data));
const isArray = computed(() => Array.isArray(props.data));
const isExpandable = computed(() => isObject.value || isArray.value);
const entries = computed(() => {
  if (isObject.value) return Object.entries(props.data);
  if (isArray.value) return props.data.map((v: any, i: number) => [String(i), v]);
  return [];
});
const bracketOpen = computed(() => isArray.value ? '[' : '{');
const bracketClose = computed(() => isArray.value ? ']' : '}');
const itemCount = computed(() => entries.value.length);

const toggle = () => { collapsed.value = !collapsed.value; };

const valueColor = (val: any) => {
  if (val === null) return 'var(--theme-text-muted)';
  if (typeof val === 'string') return '#22c55e';
  if (typeof val === 'number') return '#3b82f6';
  if (typeof val === 'boolean') return '#f59e0b';
  return 'var(--theme-text)';
};

const formatValue = (val: any) => {
  if (val === null) return 'null';
  if (typeof val === 'string') return `"${val}"`;
  return String(val);
};
</script>

<template>
  <div class="json-tree" :style="{ paddingLeft: depth > 0 ? '16px' : '0' }">
    <template v-if="isExpandable">
      <span class="cursor-pointer select-none inline-flex items-center gap-1 hover:opacity-70" @click="toggle">
        <span class="text-[10px] w-3 inline-block" style="color: var(--theme-text-muted);">{{ collapsed ? '▶' : '▼' }}</span>
        <span v-if="keyName !== undefined" style="color: var(--theme-primary);">"{{ keyName }}"</span>
        <span v-if="keyName !== undefined" style="color: var(--theme-text-muted);">: </span>
        <span style="color: var(--theme-text-muted);">{{ bracketOpen }}</span>
        <span v-if="collapsed" class="text-[10px]" style="color: var(--theme-text-muted);"> {{ itemCount }} items {{ bracketClose }}</span>
      </span>
      <div v-if="!collapsed">
        <JsonTree
          v-for="[key, val] in entries"
          :key="key"
          :data="val"
          :depth="depth + 1"
          :key-name="key"
        />
        <div :style="{ paddingLeft: '0px' }">
          <span style="color: var(--theme-text-muted);">{{ bracketClose }}</span>
        </div>
      </div>
    </template>
    <template v-else>
      <div class="inline leading-relaxed">
        <span class="w-3 inline-block"></span>
        <span v-if="keyName !== undefined" style="color: var(--theme-primary);">"{{ keyName }}"</span>
        <span v-if="keyName !== undefined" style="color: var(--theme-text-muted);">: </span>
        <span :style="{ color: valueColor(data) }">{{ formatValue(data) }}</span>
      </div>
    </template>
  </div>
</template>
