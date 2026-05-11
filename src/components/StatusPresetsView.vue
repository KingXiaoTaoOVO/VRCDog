<script setup lang="ts">
import CustomSelect from './CustomSelect.vue';
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
    console.warn(t('auto_4aec9eff'), err);
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
  { value: 'active', label: t('status_presets.active'), color: 'bg-green-500 shadow-green-500/20' },
  { value: 'join me', label: t('status_presets.join_me'), color: 'bg-blue-500 shadow-blue-500/20' },
  { value: 'ask me', label: t('status_presets.ask_me'), color: 'bg-orange-500 shadow-orange-500/20' },
  { value: 'busy', label: t('status_presets.busy'), color: 'bg-red-500 shadow-red-500/20' },
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
    console.warn(t('auto_2997abb6'), err);
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
    console.warn(t('auto_7f2d13ea'), err);
  }
};

const deletePreset = async (id: number) => {
  try {
    await DbApi.deletePreset({ id });
    await fetchPresets();
  } catch (err) {
    console.warn(t('auto_534a8c6e'), err);
  }
};

const getStatusColor = (status: string) => {
  return statusOptions.value.find(o => o.value === status)?.color || 'bg-surface';
};

const getLocalizedName = (name: string) => {
  if ([t('auto_fdfa550a'), 'Default Online', 'デフォルトオンライン'].includes(name)) return t('status_presets.default_online_label');
  if ([t('auto_ffba202d'), 'Focused', t('auto_1034b2cf')].includes(name)) return t('status_presets.default_busy_label');
  if ([t('auto_0e3cceec'), 'Join Me', 'Joinしてね'].includes(name)) return t('status_presets.default_join_label');
  return name;
};

const getLocalizedDesc = (desc: string) => {
  if ([t('auto_59d64766'), 'Online, you can invite me', t('auto_6a6fce8d')].includes(desc)) return t('status_presets.default_online_desc');
  if ([t('auto_71c85476'), 'Focusing on something, do not disturb', t('auto_bdb32a8e')].includes(desc)) return t('status_presets.default_busy_desc');
  if ([t('auto_ffc3e845'), 'Come play with me!', t('auto_08400ded')].includes(desc)) return t('status_presets.default_join_desc');
  return desc;
};

onMounted(() => fetchPresets());
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-surface-hover rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-primary/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <div class="flex items-center justify-between mb-8 shrink-0 z-10">
      <h2 class="text-3xl font-extrabold text-text tracking-tight flex items-center gap-3">
        <span class="inline-flex items-center justify-center p-2 bg-primary/10 rounded-2xl shadow-sm border-primary">
          <Sparkles class="w-6 h-6 text-primary" />
        </span>
        {{ t('status_presets.title') }}
      </h2>
      <button
        class="bg-primary hover:bg-primary text-white font-bold text-sm px-4 py-2.5 rounded-xl shadow-sm shadow-indigo-500/30 flex items-center gap-2 transition-all hover:scale-105 active:scale-95"
        @click="showForm = !showForm"
      >
        <Plus :size="16" /> {{ t('status_presets.add_preset') }}
      </button>
    </div>

    <!-- 新增表单 -->
    <div
      v-if="showForm"
      class="bg-surface backdrop-blur-xl rounded-2xl p-6 border-border-soft mb-6 shadow-md z-10 relative"
    >
      <div class="grid grid-cols-2 gap-5 mb-5">
        <div>
          <label class="block text-xs font-extrabold text-text-muted mb-2 tracking-wide">{{ t('status_presets.preset_name') }}</label>
          <input
            v-model="form.name"
            type="text"
            :placeholder="t('status_presets.preset_name_placeholder')"
            class="w-full px-4 py-2.5 rounded-xl border-border-soft  focus:ring-4 focus:ring-indigo-500/10 outline-none text-sm bg-surface-hover transition-all font-medium text-text"
          >
        </div>
        <div>
          <label class="block text-xs font-extrabold text-text-muted mb-2 tracking-wide">{{ t('status_presets.status_type') }}</label>
          <CustomSelect v-model="form.status" :options="statusOptions" />
        </div>
      </div>
      <div class="mb-5">
        <label class="block text-xs font-extrabold text-text-muted mb-2 tracking-wide">{{ t('status_presets.status_desc') }}</label>
        <input
          v-model="form.status_description"
          type="text"
          :placeholder="t('status_presets.status_desc_placeholder')"
          class="w-full px-4 py-2.5 rounded-xl border-border-soft  focus:ring-4 focus:ring-indigo-500/10 outline-none text-sm bg-surface-hover transition-all font-medium text-text"
        >
      </div>
      <div class="flex gap-3 justify-end pt-2 border-border-soft">
        <button
          class="text-sm text-text-muted hover:text-text-muted font-bold px-4 py-2 rounded-xl hover:bg-surface transition-colors"
          @click="showForm = false"
        >
          {{ t('status_presets.cancel') }}
        </button>
        <button
          class="text-sm bg-primary text-white font-bold px-5 py-2 rounded-xl hover:bg-primary/10 shadow-sm shadow-indigo-500/30 flex items-center gap-2 transition-colors"
          @click="addPreset"
        >
          <Zap :size="16" /> {{ t('status_presets.save') }}
        </button>
      </div>
    </div>

    <!-- 预设列表 -->
    <div class="flex-1 overflow-y-auto pr-2 custom-scrollbar z-10 relative">
      <div
        v-if="loading"
        class="absolute inset-0 flex flex-col items-center justify-center text-primary bg-surface-hover backdrop-blur-sm z-10"
      >
        <Loader2
          class="animate-spin mb-4"
          :size="48"
        />
        <span class="font-extrabold text-lg tracking-wide">{{ t('status_presets.loading') }}</span>
      </div>

      <div
        v-else-if="presets.length === 0 && !showForm"
        class="h-full flex flex-col items-center justify-center text-border-strong"
      >
        <Sparkles
          class="mb-4 opacity-30"
          :size="64"
        />
        <p class="font-bold text-xl text-text-muted">
          {{ t('status_presets.no_presets') }}
        </p>
        <p class="text-sm mt-2 font-medium">
          {{ t('status_presets.no_presets_desc') }}
        </p>
      </div>

      <div
        v-else
        class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 pb-10"
      >
        <div
          v-for="(preset, idx) in presets"
          :key="preset.id ?? idx"
          class="bg-surface backdrop-blur-xl rounded-2xl p-5 border-border-soft shadow-sm hover:shadow-md hover:border-primary transition-all flex flex-col gap-4 group relative"
        >
          <div class="flex items-start gap-3">
            <div
              class="w-3.5 h-3.5 rounded-full mt-1 flex-shrink-0 shadow-sm"
              :class="getStatusColor(preset.status)"
            />
            <div class="flex-1 min-w-0">
              <h3 class="font-bold text-text text-base truncate pr-6">
                {{ getLocalizedName(preset.name) }}
              </h3>
              <p class="text-sm text-text-muted mt-1 line-clamp-2 leading-relaxed">
                {{ getLocalizedDesc(preset.status_description) }}
              </p>
            </div>
          </div>
          
          <div class="flex items-center justify-end gap-2 mt-auto border-border-soft pt-3">
            <button
              v-if="props.userId"
              :disabled="applyingId === preset.id"
              class="flex-1 flex justify-center items-center gap-1.5 py-2 rounded-xl text-sm font-bold transition-all"
              :class="appliedId === preset.id ? 'bg-green-100 text-green-700' : 'bg-primary/10 hover:bg-primary/10 text-primary hover:text-primary'"
              @click="applyPreset(preset)"
            >
              <Check
                v-if="appliedId === preset.id"
                :size="16"
              />
              <Loader2
                v-else-if="applyingId === preset.id"
                :size="16"
                class="animate-spin"
              />
              <Send
                v-else
                :size="16"
              />
              <span v-if="appliedId === preset.id">{{ t('status_presets.applied') === 'status_presets.applied' ? '已应用' : t('status_presets.applied') }}</span>
              <span v-else>{{ t('status_presets.apply') === 'status_presets.apply' ? '应用预设' : t('status_presets.apply') }}</span>
            </button>
            <button
              class="p-2 rounded-xl hover:bg-red-50 text-border-strong hover:text-red-500 transition-colors"
              :title="t('global.auto_8767ff08')"
              @click="deletePreset(preset.id!)"
            >
              <Trash2 :size="18" />
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: #cbd5e1; border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: #94a3b8; }
</style>
