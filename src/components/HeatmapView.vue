<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { DbApi } from "../api";
import { Flame, Users, Loader2, CalendarHeart, Zap } from 'lucide-vue-next';
import BaseModal from './BaseModal.vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

interface HeatmapCell {
  day: number;
  hour: number;
  count: number;
}

const cells = ref<HeatmapCell[]>([]);
const loading = ref(true);
const maxCount = ref(1);

const dayLabels = computed(() => {
  const days = t('dashboard.days');
  if (Array.isArray(days)) return days;
  // fallback if i18n is not an array
  return [t('auto_7941da94'), t('auto_2d8be272'), t('auto_e662ff59'), t('auto_21716cf3'), t('auto_1fcc29d0'), t('auto_61b45352'), t('auto_3edddd85')];
});

const fetchHeatmap = async () => {
  loading.value = true;
  try {
    const res: HeatmapCell[] = await DbApi.getHeatmap();
    cells.value = res;
    maxCount.value = Math.max(1, ...res.map(c => c.count));
  } catch (err) {
    console.warn(t('auto_94e7b6c1'), err);
    cells.value = [];
  } finally {
    loading.value = false;
  }
};

const getCell = (day: number, hour: number) => {
  return cells.value.find(c => c.day === day && c.hour === hour) || { day, hour, count: 0 };
};

const getCellColor = (count: number) => {
  if (count === 0) return 'bg-surface border-transparent';
  const ratio = count / maxCount.value;
  if (ratio < 0.25) return 'bg-primary/10 border-primary';
  if (ratio < 0.5) return 'bg-primary/10 border-primary shadow-sm';
  if (ratio < 0.75) return 'bg-primary text-white shadow-md border-primary';
  return 'bg-primary text-white shadow-lg animate-pulse border-primary';
};

const peakHour = computed(() => {
  if (cells.value.length === 0) return t('heatmap.no_data_yet');
  const peak = cells.value.reduce((a, b) => a.count > b.count ? a : b);
  if (peak.count === 0) return t('heatmap.no_records');
  return `${dayLabels.value[peak.day]} ${peak.hour}:00 (${t('heatmap.count_unit', { count: peak.count })})`;
});

const totalActivity = computed(() => cells.value.reduce((sum, c) => sum + c.count, 0));

onMounted(() => fetchHeatmap());

// Detail Modal
const showDetails = ref(false);
const detailLoading = ref(false);
const detailTitle = ref('');
const detailData = ref<{displayName: string; count: number}[]>([]);

const openDetails = async (day: number, hour: number) => {
  const cell = getCell(day, hour);
  if (cell.count === 0) return;
  
  detailTitle.value = `${dayLabels.value[day]} ${hour}:00 - ${hour+1}:00 ${t('heatmap.active_friends')}`;
  showDetails.value = true;
  detailLoading.value = true;
  detailData.value = [];
  try {
    const res = await DbApi.getHeatmapDetails({ day, hour });
    detailData.value = res;
  } catch (err) {
    console.error(err);
  } finally {
    detailLoading.value = false;
  }
};
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-surface-hover rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-primary/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-primary/10 rounded-full blur-[120px] pointer-events-none -z-10" />

    <div class="flex items-center justify-between mb-8 shrink-0 z-10">
      <div>
        <h2 class="text-3xl font-extrabold text-text tracking-tight flex items-center gap-3">
          <span class="inline-flex items-center justify-center p-2 bg-primary/10 rounded-2xl shadow-sm border-primary">
            <CalendarHeart class="w-6 h-6 text-primary" />
          </span>
          {{ t('heatmap.title') }}
        </h2>
      </div>
      <div class="flex flex-col items-end gap-2 text-xs">
        <span class="text-primary font-bold bg-surface px-3 py-1.5 rounded-xl border-border-soft shadow-sm flex items-center gap-1.5">
          <Flame
            :size="14"
            class="text-orange-500"
          /> {{ t('heatmap.peak_hour') }}: {{ peakHour }}
        </span>
        <span class="text-text-muted font-bold bg-surface px-3 py-1.5 rounded-xl border-border-soft shadow-sm flex items-center gap-1.5">
          <Zap
            :size="14"
            class="text-primary"
          /> {{ t('heatmap.total_records') }}: {{ totalActivity }}
        </span>
      </div>
    </div>

    <div class="bg-surface backdrop-blur-xl rounded-3xl p-6 border-border-strong shadow-lg shadow-slate-200/40 overflow-y-auto custom-scrollbar flex-1 flex flex-col z-10 relative">
      <!-- 图例 -->
      <div class="flex items-center justify-end gap-2.5 mb-6 text-xs text-text-muted font-bold bg-surface-hover backdrop-blur py-2 px-5 rounded-full self-end border-border-soft shadow-sm">
        <span>{{ t('heatmap.legend_quiet') }}</span>
        <div class="w-4 h-4 rounded-md bg-surface" />
        <div class="w-4 h-4 rounded-md bg-primary/10" />
        <div class="w-4 h-4 rounded-md bg-primary/10" />
        <div class="w-4 h-4 rounded-md bg-primary/10" />
        <div class="w-4 h-4 rounded-md bg-primary/10" />
        <span>{{ t('heatmap.legend_busy') }}</span>
      </div>

      <!-- 热力图网格 -->
      <div class="flex-1 overflow-x-auto pb-4">
        <div
          v-if="loading"
          class="h-full flex flex-col items-center justify-center text-primary font-bold opacity-70"
        >
          <CalendarHeart
            class="animate-bounce mb-4"
            :size="48"
          />
          <p class="font-extrabold text-lg tracking-wide">
            {{ t('heatmap.analyzing') }}
          </p>
        </div>
        <table
          v-else
          class="w-full border-collapse min-w-[800px]"
        >
          <thead>
            <tr>
              <th class="w-16" />
              <th
                v-for="h in 24"
                :key="h"
                class="text-xs text-border-strong font-bold px-1 pb-4 text-center font-mono"
              >
                {{ h - 1 }}
              </th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="(dayLabel, dayIdx) in dayLabels"
              :key="dayIdx"
              class="hover:bg-surface-hover transition-colors"
            >
              <td class="text-sm text-text-muted font-extrabold pr-5 py-2.5 whitespace-nowrap text-right">
                {{ dayLabel }}
              </td>
              <td
                v-for="hour in 24"
                :key="hour"
                class="p-1"
              >
                <div
                  class="w-full aspect-square rounded-[10px] transition-all hover:scale-125 hover:z-10 hover:shadow-xl min-w-[24px] flex items-center justify-center relative group"
                  :class="[getCellColor(getCell(dayIdx, hour - 1).count), getCell(dayIdx, hour - 1).count > 0 ? 'cursor-pointer' : 'cursor-default']"
                  @click="openDetails(dayIdx, hour - 1)"
                >
                  <span
                    v-if="getCell(dayIdx, hour - 1).count >= maxCount * 0.75"
                    class="text-[10px] text-white font-bold opacity-90 pointer-events-none drop-shadow-sm"
                  />
                  
                  <div class="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 px-3 py-1.5 bg-surface text-text-inverse text-xs font-bold rounded-lg opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none whitespace-nowrap z-20 drop-shadow-lg">
                    {{ dayLabel }} {{ hour - 1 }}:00 — {{ t('heatmap.count_unit', { count: getCell(dayIdx, hour - 1).count }) }}
                  </div>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- 活跃详情弹窗 -->
    <BaseModal
      :show="showDetails"
      :loading="detailLoading"
      @close="showDetails = false"
    >
      <template v-if="showDetails">
        <div class="p-6">
          <h3 class="text-xl font-black text-text flex items-center gap-3 mb-5 pb-5 border-border-soft">
            <Users class="text-primary" />
            {{ detailTitle }}
          </h3>
          
          <div
            v-if="detailLoading"
            class="py-12 flex justify-center text-primary"
          >
            <Loader2
              class="animate-spin"
              :size="32"
            />
          </div>
          
          <div
            v-else-if="detailData.length === 0"
            class="py-12 text-center text-text-muted font-bold text-sm"
          >
            {{ t('heatmap.no_details') }}
          </div>
          
          <div
            v-else
            class="space-y-3"
          >
            <div
              v-for="(item, idx) in detailData"
              :key="idx"
              class="flex items-center justify-between p-4 bg-surface-hover rounded-2xl hover:bg-surface transition-colors border-border-soft shadow-sm"
            >
              <div class="flex items-center gap-4">
                <div class="w-8 h-8 rounded-full bg-primary/10 text-primary font-black flex items-center justify-center text-xs">
                  #{{ idx + 1 }}
                </div>
                <span class="font-bold text-text text-base">{{ item.displayName }}</span>
              </div>
              <span class="bg-primary/10 text-primary border-primary px-3 py-1 rounded-xl text-xs font-bold">{{ t('heatmap.count_unit', { count: item.count }) }}</span>
            </div>
          </div>
        </div>
      </template>
    </BaseModal>
  </div>
</template>


