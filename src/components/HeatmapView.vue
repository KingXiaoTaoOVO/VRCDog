<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { DbApi } from "../api";
import { Flame, Users, Loader2 } from 'lucide-vue-next';
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
  return ['一', '二', '三', '四', '五', '六', '日'];
});

const fetchHeatmap = async () => {
  loading.value = true;
  try {
    const res: HeatmapCell[] = await DbApi.getHeatmap();
    cells.value = res;
    maxCount.value = Math.max(1, ...res.map(c => c.count));
  } catch (err) {
    console.warn('热力图数据加载失败:', err);
    cells.value = [];
  } finally {
    loading.value = false;
  }
};

const getCell = (day: number, hour: number) => {
  return cells.value.find(c => c.day === day && c.hour === hour) || { day, hour, count: 0 };
};

const getCellColor = (count: number) => {
  if (count === 0) return 'bg-amber-50';
  const ratio = count / maxCount.value;
  if (ratio < 0.25) return 'bg-amber-200';
  if (ratio < 0.5) return 'bg-orange-300';
  if (ratio < 0.75) return 'bg-orange-500 text-white shadow-md';
  return 'bg-red-500 text-white shadow-lg animate-pulse';
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
  <div class="h-full flex flex-col">
    <div class="flex items-center justify-between mb-6">
      <div>
        <h2 class="text-3xl font-extrabold text-[#451a03] flex items-center gap-3">
          <span class="inline-flex items-center justify-center p-1.5 bg-orange-100 rounded-xl">
            <Flame
              class="text-orange-600"
              :size="24"
            /> 
          </span>
          {{ t('heatmap.title') }}
        </h2>
        <p class="text-amber-700/80 font-medium mt-1">
          {{ t('heatmap.subtitle') }}
        </p>
      </div>
      <div class="flex flex-col items-end gap-2 text-sm">
        <span class="text-amber-700 font-bold bg-amber-50 px-3 py-1.5 rounded-xl border border-amber-100">🔥 {{ t('heatmap.peak_hour') }}: {{ peakHour }}</span>
        <span class="bg-orange-100 text-orange-800 px-3 py-1.5 rounded-xl font-bold">
          ⚡ {{ t('heatmap.total_records') }}: {{ totalActivity }}
        </span>
      </div>
    </div>

    <div class="bg-white/60 backdrop-blur-md rounded-3xl p-6 border-2 border-white shadow-lg overflow-y-auto custom-scrollbar flex-1 flex flex-col">
      <!-- 图例 -->
      <div class="flex items-center justify-end gap-2 mb-6 text-xs text-amber-700 font-bold bg-white/50 py-2 px-4 rounded-full self-end border border-amber-50 shadow-sm">
        <span>{{ t('heatmap.legend_quiet') }}</span>
        <div class="w-4 h-4 rounded bg-amber-50 border border-amber-200" />
        <div class="w-4 h-4 rounded bg-amber-200" />
        <div class="w-4 h-4 rounded bg-orange-300" />
        <div class="w-4 h-4 rounded bg-orange-500" />
        <div class="w-4 h-4 rounded bg-red-500" />
        <span>{{ t('heatmap.legend_busy') }}</span>
      </div>

      <!-- 热力图网格 -->
      <div class="flex-1 overflow-x-auto pb-4">
        <div
          v-if="loading"
          class="h-full flex flex-col items-center justify-center text-orange-500 font-bold opacity-70"
        >
          <Flame
            class="animate-bounce mb-4"
            :size="48"
          />
          <p>{{ t('heatmap.analyzing') }}</p>
        </div>
        <table
          v-else
          class="w-full border-collapse min-w-[700px]"
        >
          <thead>
            <tr>
              <th class="w-16" />
              <th
                v-for="h in 24"
                :key="h"
                class="text-xs text-amber-500 font-bold px-1 pb-3 text-center"
              >
                {{ h - 1 }}
              </th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="(dayLabel, dayIdx) in dayLabels"
              :key="dayIdx"
              class="hover:bg-amber-50/50 transition-colors"
            >
              <td class="text-sm text-amber-900 font-extrabold pr-4 py-2 whitespace-nowrap text-right">
                {{ dayLabel }}
              </td>
              <td
                v-for="hour in 24"
                :key="hour"
                class="p-1"
              >
                <div
                  class="w-full aspect-square rounded-lg transition-all hover:scale-125 hover:z-10 hover:shadow-lg min-w-[20px] flex items-center justify-center"
                  :class="[getCellColor(getCell(dayIdx, hour - 1).count), getCell(dayIdx, hour - 1).count > 0 ? 'cursor-pointer' : 'cursor-default']"
                  :title="`${dayLabel} ${hour - 1}:00 — ${t('heatmap.count_unit', { count: getCell(dayIdx, hour - 1).count })}`"
                  @click="openDetails(dayIdx, hour - 1)"
                >
                  <span
                    v-if="getCell(dayIdx, hour - 1).count >= maxCount * 0.75"
                    class="text-[8px] text-white font-bold opacity-80 pointer-events-none"
                  >🔥</span>
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
          <h3 class="text-xl font-extrabold text-[#451a03] flex items-center gap-2 mb-4 pb-4 border-b border-amber-100">
            <Users class="text-orange-500" />
            {{ detailTitle }}
          </h3>
          
          <div
            v-if="detailLoading"
            class="py-12 flex justify-center text-orange-500"
          >
            <Loader2
              class="animate-spin"
              :size="32"
            />
          </div>
          
          <div
            v-else-if="detailData.length === 0"
            class="py-12 text-center text-amber-600 font-bold text-sm"
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
              class="flex items-center justify-between p-3 bg-amber-50 rounded-xl hover:bg-amber-100 transition-colors border border-amber-100"
            >
              <div class="flex items-center gap-3">
                <div class="w-8 h-8 rounded-full bg-orange-200 text-orange-800 font-extrabold flex items-center justify-center text-xs">
                  #{{ idx + 1 }}
                </div>
                <span class="font-bold text-amber-900">{{ item.displayName }}</span>
              </div>
              <span class="bg-orange-100 text-orange-800 px-3 py-1 rounded-full text-xs font-bold">{{ t('heatmap.count_unit', { count: item.count }) }}</span>
            </div>
          </div>
        </div>
      </template>
    </BaseModal>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { height: 8px; width: 8px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(245, 158, 11, 0.3); border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(245, 158, 11, 0.5); }
</style>
