<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { VrcApi, DbApi, SysApi, GamelogApi } from "../api";
import { Sparkles, Plus, Trash2, Zap, Send, Loader2, Check } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const props = defineProps<{ userId?: string }>();
const applyingId = ref<number | null>(null);
const appliedId = ref<number | null>(null);

const applyPreset = async (preset: StatusPreset) => {
  if (!props.userId || !preset.id) return;
  applyingId.value = preset.id;
  try {
    await VrcApi.updateStatus({
      userId: props.userId,
      status: preset.status,
      statusDescription: preset.status_description,
    });
    appliedId.value = preset.id;
    setTimeout(() => { appliedId.value = null; }, 2000);
  } catch (err) {
    console.warn('应用状态失败:', err);
  } finally {
    applyingId.value = null;
  }
};

interface StatusPreset {
  id: number | null;
  status: string;
  status_description: string;
  name: string;
}

const presets = ref<StatusPreset[]>([]);
const loading = ref(true);
const showForm = ref(false);
const form = ref({ status: 'active', status_description: '', name: '' });

const statusOptions = computed(() => [
  { value: 'active', label: t('status_presets.active'), color: 'bg-green-500' },
  { value: 'join me', label: t('status_presets.join_me'), color: 'bg-blue-500' },
  { value: 'ask me', label: t('status_presets.ask_me'), color: 'bg-orange-500' },
  { value: 'busy', label: t('status_presets.busy'), color: 'bg-red-500' },
]);

const fetchPresets = async () => {
  loading.value = true;
  try {
    const data = await DbApi.getPresets();
    if (data.length === 0) {
      // 初始化默认预设
      await DbApi.savePreset({ status: 'active', statusDescription: t('status_presets.default_online_desc'), name: t('status_presets.default_online_label') });
      await DbApi.savePreset({ status: 'busy', statusDescription: t('status_presets.default_busy_desc'), name: t('status_presets.default_busy_label') });
      await DbApi.savePreset({ status: 'join me', statusDescription: t('status_presets.default_join_desc'), name: t('status_presets.default_join_label') });
      presets.value = await DbApi.getPresets();
    } else {
      presets.value = data;
    }
  } catch (err) {
    console.warn('加载预设失败:', err);
  } finally {
    loading.value = false;
  }
};

const addPreset = async () => {
  if (!form.value.name || !form.value.status_description) return;
  try {
    await DbApi.savePreset({
      status: form.value.status,
      statusDescription: form.value.status_description,
      name: form.value.name,
    });
    showForm.value = false;
    form.value = { status: 'active', status_description: '', name: '' };
    await fetchPresets();
  } catch (err) {
    console.warn('保存预设失败:', err);
  }
};

const deletePreset = async (id: number) => {
  try {
    await DbApi.deletePreset({ id });
    await fetchPresets();
  } catch (err) {
    console.warn('删除预设失败:', err);
  }
};

const getStatusColor = (status: string) => {
  return statusOptions.value.find(o => o.value === status)?.color || 'bg-gray-400';
};

const getLocalizedName = (name: string) => {
  if (['默认在线', 'Default Online', 'デフォルトオンライン'].includes(name)) return t('status_presets.default_online_label');
  if (['专心致志', 'Focused', '作業中'].includes(name)) return t('status_presets.default_busy_label');
  if (['随便进', 'Join Me', 'Joinしてね'].includes(name)) return t('status_presets.default_join_label');
  return name;
};

const getLocalizedDesc = (desc: string) => {
  if (['在线，可邀请我', 'Online, you can invite me', 'オンライン、招待可能です'].includes(desc)) return t('status_presets.default_online_desc');
  if (['正在专心做某事，请勿打扰', 'Focusing on something, do not disturb', '集中しています、邪魔しないでください'].includes(desc)) return t('status_presets.default_busy_desc');
  if (['来找我玩！', 'Come play with me!', '一緒に遊ぼう！'].includes(desc)) return t('status_presets.default_join_desc');
  return desc;
};

onMounted(() => fetchPresets());
</script>

<template>
  <div>
    <div class="flex items-center justify-between mb-6">
      <h2 class="text-2xl font-extrabold text-[#451a03] flex items-center gap-2">
        <Sparkles
          class="text-amber-500"
          :size="24"
        /> {{ t('status_presets.title') }}
      </h2>
      <button
        class="bg-amber-500 hover:bg-amber-600 text-white font-bold text-xs px-4 py-2 rounded-xl shadow-md flex items-center gap-1 transition-colors"
        @click="showForm = !showForm"
      >
        <Plus :size="14" /> {{ t('status_presets.add_preset') }}
      </button>
    </div>

    <!-- 新增表单 -->
    <div
      v-if="showForm"
      class="bg-white/90 backdrop-blur rounded-2xl p-5 border-2 border-amber-200 mb-6 shadow-sm"
    >
      <div class="grid grid-cols-2 gap-4 mb-4">
        <div>
          <label class="block text-xs font-bold text-amber-900 mb-1">{{ t('status_presets.preset_name') }}</label>
          <input
            v-model="form.name"
            type="text"
            :placeholder="t('status_presets.preset_name_placeholder')"
            class="w-full px-3 py-2 rounded-xl border-2 border-amber-100 focus:border-amber-400 focus:ring-0 outline-none text-sm bg-amber-50/30"
          >
        </div>
        <div>
          <label class="block text-xs font-bold text-amber-900 mb-1">{{ t('status_presets.status_type') }}</label>
          <select
            v-model="form.status"
            class="w-full px-3 py-2 rounded-xl border-2 border-amber-100 focus:border-amber-400 focus:ring-0 outline-none text-sm bg-amber-50/30"
          >
            <option
              v-for="opt in statusOptions"
              :key="opt.value"
              :value="opt.value"
            >
              {{ opt.label }}
            </option>
          </select>
        </div>
      </div>
      <div class="mb-4">
        <label class="block text-xs font-bold text-amber-900 mb-1">{{ t('status_presets.status_desc') }}</label>
        <input
          v-model="form.status_description"
          type="text"
          :placeholder="t('status_presets.status_desc_placeholder')"
          class="w-full px-3 py-2 rounded-xl border-2 border-amber-100 focus:border-amber-400 focus:ring-0 outline-none text-sm bg-amber-50/30"
        >
      </div>
      <div class="flex gap-2 justify-end">
        <button
          class="text-xs text-amber-600 hover:text-amber-800 font-bold px-4 py-2 rounded-lg"
          @click="showForm = false"
        >
          {{ t('status_presets.cancel') }}
        </button>
        <button
          class="text-xs bg-green-500 text-white font-bold px-4 py-2 rounded-xl hover:bg-green-600 flex items-center gap-1"
          @click="addPreset"
        >
          <Zap :size="12" /> {{ t('status_presets.save') }}
        </button>
      </div>
    </div>

    <!-- 预设列表 -->
    <div
      v-if="loading"
      class="text-center py-8 text-amber-500 font-bold animate-pulse"
    >
      {{ t('status_presets.loading') }}
    </div>

    <div
      v-else-if="presets.length === 0 && !showForm"
      class="bg-white/80 backdrop-blur rounded-2xl p-8 border-2 border-amber-100 text-center text-amber-600"
    >
      <Sparkles
        class="mx-auto mb-4 text-amber-300"
        :size="48"
      />
      <p class="font-bold">
        {{ t('status_presets.no_presets') }}
      </p>
      <p class="text-sm mt-1">
        {{ t('status_presets.no_presets_desc') }}
      </p>
    </div>

    <div
      v-else
      class="grid grid-cols-1 md:grid-cols-2 gap-3"
    >
      <div
        v-for="(preset, idx) in presets"
        :key="preset.id ?? idx"
        class="bg-white/80 backdrop-blur rounded-2xl p-4 border-2 border-amber-50 hover:border-amber-200 transition-all flex items-center gap-3 group"
      >
        <div
          class="w-3 h-3 rounded-full"
          :class="getStatusColor(preset.status)"
        />
        <div class="flex-1 min-w-0">
          <h3 class="font-bold text-amber-900 text-sm">
            {{ getLocalizedName(preset.name) }}
          </h3>
          <p class="text-xs text-amber-600 truncate">
            {{ getLocalizedDesc(preset.status_description) }}
          </p>
        </div>
        <button
          v-if="props.userId"
          :disabled="applyingId === preset.id"
          class="opacity-0 group-hover:opacity-100 p-1.5 rounded-lg text-xs font-bold transition-all flex items-center gap-1"
          :class="appliedId === preset.id ? 'bg-green-100 text-green-600' : 'hover:bg-blue-50 text-blue-500 hover:text-blue-700'"
          @click="applyPreset(preset)"
        >
          <Check
            v-if="appliedId === preset.id"
            :size="14"
          />
          <Loader2
            v-else-if="applyingId === preset.id"
            :size="14"
            class="animate-spin"
          />
          <Send
            v-else
            :size="14"
          />
        </button>
        <button
          class="opacity-0 group-hover:opacity-100 p-1.5 rounded-lg hover:bg-red-50 text-red-400 hover:text-red-600 transition-all"
          @click="deletePreset(preset.id!)"
        >
          <Trash2 :size="14" />
        </button>
      </div>
    </div>
  </div>
</template>
