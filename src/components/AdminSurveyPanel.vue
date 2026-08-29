<script setup lang="ts">
import { computed, onMounted, ref, shallowRef, watch } from 'vue';
import {
  CheckCircle2,
  ClipboardList,
  Download,
  FileQuestion,
  Image,
  Plus,
  RefreshCcw,
  Save,
  Send,
  Trash2,
  Video,
  X,
} from 'lucide-vue-next';
import { SysApi, VrcApi } from '../api';
import { useI18n } from 'vue-i18n';
import { buildSurveyWorkbook, surveyExportFileName } from '../utils/surveyExcel';
import type {
  Survey,
  SurveyClickEvent,
  SurveyMediaType,
  SurveyOption,
  SurveyQuestion,
  SurveyQuestionType,
  SurveySubmission,
} from '../types/survey';

const props = defineProps<{
  serverUrl: string;
  adminPassword: string;
  active: boolean;
}>();

const emit = defineEmits<{ log: [message: string] }>();
const { t } = useI18n();
const surveys = shallowRef<Survey[]>([]);
const submissions = shallowRef<SurveySubmission[]>([]);
const users = shallowRef<Array<{ user_id: string; display_name: string }>>([]);
const roles = shallowRef<Array<{ role_id: string; role_name: string; is_default: boolean }>>([]);
const selected = ref<Survey | null>(null);
const enabled = ref(false);
const loading = ref(false);
const saving = ref(false);
const exporting = ref(false);
const message = ref('');
const workspaceMode = ref<'design' | 'responses'>('design');
const selectedSubmission = ref<SurveySubmission | null>(null);

const headers = computed(() => ({ 'x-vrcdog-admin-password': props.adminPassword }));
const newId = (prefix: string) => `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;

const request = (path: string, options: Record<string, unknown> = {}) => VrcApi.request(
  `${props.serverUrl}${path}`,
  { ...options, headers: headers.value },
);

const notify = (text: string, error = false) => {
  message.value = text;
  emit('log', `${error ? '[ERROR]' : '[INFO]'} ${text}`);
  window.setTimeout(() => {
    if (message.value === text) message.value = '';
  }, 3500);
};

const blankQuestion = (questionType: SurveyQuestionType = 'single_choice'): SurveyQuestion => ({
  question_id: newId('question'),
  question_type: questionType,
  title: '',
  description: '',
  required: true,
  require_correct: false,
  options: questionType.includes('choice')
    ? [
      { option_id: newId('option'), label: '', media: [] },
      { option_id: newId('option'), label: '', media: [] },
    ]
    : [],
  correct_answers: [],
  media: [],
});

const createSurvey = () => {
  selected.value = {
    survey_id: newId('survey'),
    title: '',
    description: '',
    required_for_access: true,
    status: 'draft',
    revision: 1,
    created_at: '',
    updated_at: '',
    published_at: null,
    questions: [blankQuestion()],
    reward: null,
  };
};

const selectSurvey = (survey: Survey) => {
  // Use JSON round-trip to deep-clone plain data and strip Vue reactive proxies.
  // structuredClone fails on proxied objects from API responses.
  selected.value = JSON.parse(JSON.stringify(survey));
  selectedSubmission.value = submissions.value.find((item) => item.survey_id === survey.survey_id) || null;
};

const fetchData = async () => {
  if (!props.active) return;
  loading.value = true;
  try {
    const [settingsData, surveyData, submissionData, userData, roleData] = await Promise.all([
      request('/api/admin/survey-settings'),
      request('/api/admin/surveys'),
      request('/api/admin/survey-submissions'),
      request('/api/admin/users'),
      request('/api/admin/roles'),
    ]);
    enabled.value = Boolean(settingsData?.enabled);
    surveys.value = surveyData?.surveys || [];
    submissions.value = submissionData?.submissions || [];
    users.value = userData?.users || [];
    roles.value = roleData?.roles || [];
    if (selected.value) {
      const fresh = surveys.value.find((item) => item.survey_id === selected.value?.survey_id);
      // Same fix: use JSON round-trip instead of structuredClone to handle proxies.
      if (fresh) selected.value = JSON.parse(JSON.stringify(fresh));
    }
  } catch (error: any) {
    notify(t('survey_admin.load_failed', { error: error?.message || error }), true);
  } finally {
    loading.value = false;
  }
};

const saveSettings = async () => {
  try {
    await request('/api/admin/survey-settings', {
      method: 'POST',
      params: { enabled: enabled.value },
    });
    notify(enabled.value ? t('survey_admin.enabled_on') : t('survey_admin.enabled_off'));
  } catch (error: any) {
    enabled.value = !enabled.value;
    notify(t('survey_admin.settings_save_failed', { error: error?.message || error }), true);
  }
};

const saveSurvey = async () => {
  if (!selected.value) return false;
  saving.value = true;
  try {
    const data = await request('/api/admin/surveys', {
      method: 'POST',
      params: selected.value,
    });
    if (!data?.success) throw new Error(data?.message || t('survey_admin.save_failed'));
    notify(selected.value.status === 'published' ? t('survey_admin.saved_as_new_version') : t('survey_admin.draft_saved'));
    await fetchData();
    return true;
  } catch (error: any) {
    notify(error?.message || String(error), true);
    return false;
  } finally {
    saving.value = false;
  }
};

const publishSurvey = async () => {
  if (!selected.value || !(await saveSurvey())) return;
  try {
    const data = await request('/api/admin/surveys/publish', {
      method: 'POST',
      params: { survey_id: selected.value.survey_id },
    });
    if (!data?.success) throw new Error(data?.message || t('survey_admin.publish_failed'));
    notify(t('survey_admin.published'));
    await fetchData();
  } catch (error: any) {
    notify(error?.message || String(error), true);
  }
};

const resendSurvey = async () => {
  if (!selected.value || !confirm(t('survey_admin.resend_confirm'))) return;
  try {
    const data = await request('/api/admin/surveys/resend', {
      method: 'POST',
      params: { survey_id: selected.value.survey_id },
    });
    if (!data?.success) throw new Error(data?.message || t('survey_admin.resend_failed'));
    notify(t('survey_admin.resent', { revision: data.revision }));
    await fetchData();
  } catch (error: any) {
    notify(error?.message || String(error), true);
  }
};

const deleteSurvey = async () => {
  if (!selected.value || !confirm(t('survey_admin.delete_survey_confirm'))) return;
  try {
    const data = await request('/api/admin/surveys/delete', {
      method: 'POST',
      params: { survey_id: selected.value.survey_id },
    });
    if (!data?.success) throw new Error(data?.message || t('survey_admin.delete_failed'));
    selected.value = null;
    notify(t('survey_admin.survey_deleted'));
    await fetchData();
  } catch (error: any) {
    notify(error?.message || String(error), true);
  }
};

const deleteSubmission = async (submission: SurveySubmission) => {
  if (!submission) return;
  if (!confirm(t('survey_admin.delete_submission_confirm', { name: userName(submission.user_id) }))) return;
  try {
    const data = await request('/api/admin/survey-submissions/delete', {
      method: 'POST',
      params: { submission_id: submission.submission_id },
    });
    if (!data?.success) throw new Error(data?.message || t('survey_admin.delete_failed'));
    notify(t('survey_admin.submission_deleted'));
    if (selectedSubmission.value?.submission_id === submission.submission_id) {
      selectedSubmission.value = null;
    }
    await fetchData();
  } catch (error: any) {
    notify(error?.message || String(error), true);
  }
};

const setQuestionType = (question: SurveyQuestion, type: SurveyQuestionType) => {
  question.question_type = type;
  question.correct_answers = [];
  question.options = type.includes('choice')
    ? [
      { option_id: newId('option'), label: '', media: [] },
      { option_id: newId('option'), label: '', media: [] },
    ]
    : [];
};

const addOption = (question: SurveyQuestion) => {
  question.options.push({ option_id: newId('option'), label: '', media: [] });
};

const removeOption = (question: SurveyQuestion, optionId: string) => {
  if (question.options.length <= 2) return;
  question.options = question.options.filter((option) => option.option_id !== optionId);
  question.correct_answers = question.correct_answers.filter((answer) => answer !== optionId);
};

const toggleCorrect = (question: SurveyQuestion, optionId: string) => {
  if (question.question_type === 'single_choice') {
    question.correct_answers = [optionId];
    return;
  }
  question.correct_answers = question.correct_answers.includes(optionId)
    ? question.correct_answers.filter((answer) => answer !== optionId)
    : [...question.correct_answers, optionId];
};

const setTextAnswers = (question: SurveyQuestion, value: string) => {
  question.correct_answers = value.split('\n').map((item) => item.trim()).filter(Boolean);
};

const addMedia = (question: SurveyQuestion, mediaType: SurveyMediaType) => {
  question.media.push({ media_type: mediaType, url: '', caption: '' });
};

const surveySubmissions = computed(() => {
  if (!selected.value) return [];
  return submissions.value
    .filter((s) => s.survey_id === selected.value!.survey_id)
    .sort((a, b) => b.submitted_at.localeCompare(a.submitted_at));
});

// Per-survey submission counts, memoized so the survey list doesn't re-filter
// the whole submissions array on every render.
const submissionCounts = computed<Record<string, number>>(() => {
  const map: Record<string, number> = {};
  for (const item of submissions.value) {
    map[item.survey_id] = (map[item.survey_id] || 0) + 1;
  }
  return map;
});

// Paginate the (potentially huge) submission list so we never render thousands
// of DOM rows at once.
const SUBMISSIONS_PAGE_SIZE = 50;
const submissionsPage = ref(1);
const totalSubmissionPages = computed(() =>
  Math.max(1, Math.ceil(surveySubmissions.value.length / SUBMISSIONS_PAGE_SIZE)),
);
const pagedSubmissions = computed(() => {
  const page = Math.min(submissionsPage.value, totalSubmissionPages.value);
  const start = (page - 1) * SUBMISSIONS_PAGE_SIZE;
  return surveySubmissions.value.slice(start, start + SUBMISSIONS_PAGE_SIZE);
});
watch(
  () => selected.value?.survey_id,
  () => { submissionsPage.value = 1; },
);
watch(totalSubmissionPages, (pages) => {
  if (submissionsPage.value > pages) submissionsPage.value = pages;
});

const userName = (userId: string) => {
  const displayName = users.value.find((user) => user.user_id === userId)?.display_name?.trim();
  return displayName || userId;
};

const hasUserName = (userId: string) => userName(userId) !== userId;

const openSubmissionDetail = (submission: SurveySubmission) => {
  selectedSubmission.value = submission;
};

const setWorkspaceMode = (mode: 'design' | 'responses') => {
  workspaceMode.value = mode;
  if (mode === 'responses') {
    if (!selected.value && surveys.value.length > 0) selectSurvey(surveys.value[0]);
    selectedSubmission.value = surveySubmissions.value[0] || null;
  }
};

// ─── 通过奖励编辑器 ───
// rewardEnabled drives whether a survey grants a role on a passing submission.
const rewardEnabled = computed<boolean>({
  get: () => Boolean(selected.value?.reward),
  set: (on) => {
    if (!selected.value) return;
    if (on) {
      if (!selected.value.reward) {
        selected.value.reward = { role_id: roles.value[0]?.role_id || '', duration_value: null, duration_unit: 'hour' };
      }
    } else {
      selected.value.reward = null;
    }
  },
});

// A reward is "permanent" when duration_value is null.
const rewardPermanent = computed<boolean>({
  get: () => {
    const reward = selected.value?.reward;
    return !reward || reward.duration_value === null;
  },
  set: (permanent) => {
    if (!selected.value?.reward) return;
    selected.value.reward.duration_value = permanent ? null : 24;
  },
});

const setRewardRole = (roleId: string) => {
  if (selected.value?.reward) selected.value.reward.role_id = roleId;
};

const setRewardDuration = (event: Event) => {
  if (!selected.value?.reward) return;
  const raw = (event.target as HTMLInputElement).value;
  const parsed = Number(raw);
  selected.value.reward.duration_value = raw === '' || !Number.isFinite(parsed) ? null : parsed;
};

const setRewardUnit = (event: Event) => {
  if (!selected.value?.reward) return;
  selected.value.reward.duration_unit = (event.target as HTMLSelectElement).value;
};

const addOptionMedia = (option: SurveyOption) => {
  if (!option.media) option.media = [];
  option.media.push({ media_type: 'image', url: '', caption: '' });
};

// Resolve a question_id to its title for display in the detail modal
const questionTitle = (questionId: string) => {
  if (!selected.value) return questionId;
  const q = selected.value.questions.find((q) => q.question_id === questionId);
  return q?.title || questionId;
};

const formatAnswer = (answer: string | string[] | undefined, questionId?: string) => {
  if (answer === undefined || answer === null) return t('survey_admin.not_answered');
  const question = questionId
    ? selected.value?.questions.find((item) => item.question_id === questionId)
    : undefined;
  const values = Array.isArray(answer) ? answer : [answer];
  const labels = values.map((value) => {
    const text = String(value).trim();
    if (!text) return '';
    const option = question?.options.find((option) => option.option_id === text);
    if (option) return option.label || text;
    // 选项已被删除或问卷已改版：展示原始值并注明，避免整题显示为空白
    return question ? t('survey_admin.option_deleted', { text }) : text;
  }).filter(Boolean);
  if (labels.length === 0) return Array.isArray(answer) ? t('survey_admin.not_selected') : t('survey_admin.blank');
  return labels.join('、');
};

// ─── 答题点击记录 ───
// 点击事件在服务端保存了题干/选项文案快照，即使问卷被修改或删除也能完整展示。
const submissionClicks = computed<SurveyClickEvent[]>(() => {
  const clicks = selectedSubmission.value?.click_events;
  if (!clicks || clicks.length === 0) return [];
  return [...clicks].sort((a, b) => a.clicked_at.localeCompare(b.clicked_at));
});

const clickActionLabel = (event: SurveyClickEvent) => {
  if (event.action === 'input') return event.text_value?.trim() ? t('survey_admin.input_label', { value: event.text_value }) : t('survey_admin.input_empty');
  if (event.action === 'deselect') return t('survey_admin.deselect_label', { option: event.option_label || event.option_id || t('survey_admin.unknown_option') });
  return t('survey_admin.select_label', { option: event.option_label || event.option_id || t('survey_admin.unknown_option') });
};

const downloadWorkbookFallback = (content: Uint8Array, fileName: string) => {
  const blob = new Blob([content], {
    type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
  });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = fileName;
  link.click();
  window.setTimeout(() => URL.revokeObjectURL(url), 1000);
};

const exportCurrentSurvey = async () => {
  if (!selected.value || surveySubmissions.value.length === 0 || exporting.value) return;
  exporting.value = true;
  try {
    const fileName = surveyExportFileName(selected.value.title);
    const content = await buildSurveyWorkbook(selected.value, surveySubmissions.value, users.value);
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const filePath = await save({
        filters: [{ name: t('survey_admin.excel_filter'), extensions: ['xlsx'] }],
        defaultPath: fileName,
      });
      if (!filePath) {
        notify(t('survey_admin.export_cancelled'));
        return;
      }
      await SysApi.saveBinaryFile({ path: filePath, content: Array.from(content) });
      notify(t('survey_admin.exported', { count: surveySubmissions.value.length }));
    } catch (error) {
      console.warn('Tauri Excel save failed, using browser download fallback:', error);
      downloadWorkbookFallback(content, fileName);
      notify(t('survey_admin.exported', { count: surveySubmissions.value.length }));
    }
  } catch (error: any) {
    notify(t('survey_admin.excel_export_failed', { error: error?.message || error }), true);
  } finally {
    exporting.value = false;
  }
};

watch(() => props.active, (active) => {
  if (active) fetchData();
});
onMounted(fetchData);
</script>

<template>
  <div class="h-full min-h-0 flex flex-col gap-3">
    <nav class="shrink-0 flex items-center justify-between gap-4 p-1 border border-border-soft rounded-lg bg-surface">
      <div class="flex items-center gap-1">
        <button
          class="h-9 px-4 rounded-md text-xs font-black flex items-center gap-2 transition-colors"
          :class="workspaceMode === 'design' ? 'bg-primary text-white shadow-sm' : 'text-text-muted hover:bg-surface-hover hover:text-text'"
          @click="setWorkspaceMode('design')"
        >
            <FileQuestion :size="15" /> {{ t('survey_admin.tab_design') }}
        </button>
        <button
          class="h-9 px-4 rounded-md text-xs font-black flex items-center gap-2 transition-colors"
          :class="workspaceMode === 'responses' ? 'bg-primary text-white shadow-sm' : 'text-text-muted hover:bg-surface-hover hover:text-text'"
          @click="setWorkspaceMode('responses')"
        >
            <ClipboardList :size="15" /> {{ t('survey_admin.tab_responses') }}
          <span class="px-1.5 py-0.5 rounded bg-black/10 text-[9px]">{{ submissions.length }}</span>
        </button>
      </div>
      <div class="pr-3 text-[10px] text-text-muted">
        {{ workspaceMode === 'design' ? t('survey_admin.desc_design') : t('survey_admin.desc_responses') }}
      </div>
    </nav>

    <div v-if="workspaceMode === 'design'" class="flex-1 min-h-0 flex gap-4">
      <aside class="w-64 shrink-0 border border-border-soft rounded-lg bg-surface overflow-hidden flex flex-col">
      <div class="p-3 border-b border-border-soft flex items-center justify-between gap-2">
        <div>
            <div class="text-xs font-black text-text-strong">{{ t('survey_admin.survey_list') }}</div>
          <div class="text-[10px] text-text-muted mt-1">{{ t('survey_admin.list_summary', { surveyCount: surveys.length, submissionCount: submissions.length }) }}</div>
        </div>
        <button class="w-8 h-8 grid place-items-center rounded-md bg-primary text-white" :title="t('survey_admin.new_survey')" @click="createSurvey">
          <Plus :size="16" />
        </button>
      </div>

      <label class="m-3 p-3 border border-border-soft rounded-md flex items-center justify-between gap-3 cursor-pointer">
        <span>
            <span class="block text-xs font-bold text-text">{{ t('survey_admin.client_survey') }}</span>
            <span class="block text-[10px] text-text-muted mt-1">{{ t('survey_admin.client_survey_desc') }}</span>
        </span>
        <input v-model="enabled" type="checkbox" class="w-4 h-4 accent-primary" @change="saveSettings">
      </label>

      <div class="flex-1 min-h-0 overflow-y-auto px-2 pb-2 space-y-1">
        <button
          v-for="survey in surveys"
          :key="survey.survey_id"
          class="w-full text-left p-3 rounded-md border transition-colors"
          :class="selected?.survey_id === survey.survey_id ? 'bg-primary/10 border-primary/40' : 'border-transparent hover:bg-surface-hover'"
          @click="selectSurvey(survey)"
        >
          <span class="flex items-center gap-2">
            <span class="w-2 h-2 rounded-full" :class="survey.status === 'published' ? 'bg-green-500' : 'bg-amber-500'" />
            <span class="text-xs font-bold text-text-strong truncate">{{ survey.title }}</span>
          </span>
            <span class="block text-[10px] text-text-muted mt-1.5">{{ t('survey_admin.revision_submissions', { revision: survey.revision, count: submissionCounts[survey.survey_id] || 0 }) }}</span>
        </button>
        <div v-if="!loading && surveys.length === 0" class="py-12 text-center text-xs text-text-muted">
          {{ t('survey_admin.no_surveys') }}
        </div>
      </div>
    </aside>

    <main v-if="selected" class="flex-1 min-w-0 border border-border-soft rounded-lg bg-surface overflow-hidden flex flex-col">
      <header class="px-5 py-3 border-b border-border-soft flex items-center justify-between gap-4">
        <div class="min-w-0">
          <div class="flex items-center gap-2">
            <FileQuestion :size="17" class="text-primary shrink-0" />
            <h2 class="text-sm font-black text-text-strong truncate">{{ selected.title || t('survey_admin.unnamed_survey') }}</h2>
            <span class="px-2 py-0.5 rounded text-[10px] font-bold" :class="selected.status === 'published' ? 'bg-green-500/15 text-green-500' : 'bg-amber-500/15 text-amber-500'">
              {{ selected.status === 'published' ? t('survey_admin.published_badge', { revision: selected.revision }) : t('survey_admin.draft_badge') }}
            </span>
          </div>
            <p v-if="selected.status === 'published'" class="text-[10px] text-amber-500 mt-1">{{ t('survey_admin.published_note') }}</p>
        </div>
        <div class="flex items-center gap-2 shrink-0">
          <button class="h-8 px-3 rounded-md border border-border-soft text-xs font-bold text-text hover:bg-surface-hover flex items-center gap-1.5" :disabled="saving" @click="saveSurvey">
            <Save :size="14" /> {{ t('survey_admin.save') }}
          </button>
          <button v-if="selected.status === 'draft'" class="h-8 px-3 rounded-md bg-primary text-white text-xs font-bold flex items-center gap-1.5" @click="publishSurvey">
            <Send :size="14" /> {{ t('survey_admin.publish') }}
          </button>
          <button v-else class="h-8 px-3 rounded-md bg-primary text-white text-xs font-bold flex items-center gap-1.5" @click="resendSurvey">
            <RefreshCcw :size="14" /> {{ t('survey_admin.resend') }}
          </button>
          <button class="w-8 h-8 grid place-items-center rounded-md border border-red-500/30 text-red-500 hover:bg-red-500/10" :title="t('survey_admin.delete_survey')" @click="deleteSurvey">
            <Trash2 :size="15" />
          </button>
        </div>
      </header>

      <div class="flex-1 min-h-0 overflow-y-auto p-5 space-y-5">
        <section class="grid grid-cols-[1fr_220px] gap-4 border-b border-border-soft pb-5">
          <div class="space-y-3">
            <label class="block">
              <span class="block text-[11px] font-bold text-text-muted mb-1">{{ t('survey_admin.survey_title') }}</span>
              <input v-model="selected.title" class="w-full h-9 px-3 rounded-md bg-background border border-border-soft text-sm text-text outline-none focus:border-primary" :placeholder="t('survey_admin.title_placeholder')">
            </label>
            <label class="block">
              <span class="block text-[11px] font-bold text-text-muted mb-1">{{ t('survey_admin.description_label') }}</span>
              <textarea v-model="selected.description" rows="2" class="w-full px-3 py-2 rounded-md bg-background border border-border-soft text-xs text-text outline-none resize-y focus:border-primary" :placeholder="t('survey_admin.description_placeholder')" />
            </label>
          </div>
          <label class="self-end min-h-20 p-3 rounded-md border border-border-soft flex items-start gap-3 cursor-pointer">
            <input v-model="selected.required_for_access" type="checkbox" class="mt-0.5 w-4 h-4 accent-primary">
            <span>
              <span class="block text-xs font-bold text-text-strong">{{ t('survey_admin.access_gate') }}</span>
              <span class="block text-[10px] leading-4 text-text-muted mt-1">{{ t('survey_admin.access_gate_desc') }}</span>
            </span>
          </label>
        </section>

        <section class="border-b border-border-soft pb-5">
          <div class="flex items-start justify-between gap-4">
            <div class="min-w-0">
              <div class="text-xs font-black text-text-strong">{{ t('survey_admin.reward_optional') }}</div>
              <p class="text-[10px] text-text-muted mt-1 leading-4">{{ t('survey_admin.reward_desc') }}</p>
            </div>
            <label class="flex items-center gap-2 text-xs text-text cursor-pointer shrink-0">
              <input v-model="rewardEnabled" type="checkbox" class="w-4 h-4 accent-primary"> {{ t('survey_admin.enable_reward') }}
            </label>
          </div>
          <div v-if="rewardEnabled && selected" class="mt-4 grid grid-cols-2 gap-4">
            <label class="block">
                <span class="block text-[11px] font-bold text-text-muted mb-1">{{ t('survey_admin.reward_role') }}</span>
              <select
                :value="selected.reward?.role_id || ''"
                class="w-full h-9 px-3 rounded-md bg-background border border-border-soft text-sm text-text"
                @change="setRewardRole(($event.target as HTMLSelectElement).value)"
              >
                <option v-for="role in roles" :key="role.role_id" :value="role.role_id">{{ role.role_name }}</option>
              </select>
              <span v-if="roles.length === 0" class="block text-[10px] text-amber-500 mt-1">{{ t('survey_admin.no_roles') }}</span>
            </label>
            <div class="flex items-end gap-3 flex-wrap">
              <label class="flex items-center gap-2 text-xs text-text cursor-pointer pb-2 shrink-0">
                <input v-model="rewardPermanent" type="checkbox" class="w-4 h-4 accent-primary"> {{ t('survey_admin.permanent') }}
              </label>
              <template v-if="!rewardPermanent">
                <label class="block">
                  <span class="block text-[10px] font-bold text-text-muted mb-1">{{ t('survey_admin.duration') }}</span>
                  <input
                    :value="selected.reward?.duration_value ?? ''"
                    type="number"
                    min="0"
                    step="1"
                    class="w-24 h-9 px-2 rounded-md bg-background border border-border-soft text-sm text-text"
                    :placeholder="t('survey_admin.duration_placeholder')"
                    @input="setRewardDuration"
                  >
                </label>
                <label class="block">
                  <span class="block text-[10px] font-bold text-text-muted mb-1">{{ t('survey_admin.unit') }}</span>
                  <select
                    :value="selected.reward?.duration_unit || 'hour'"
                    class="h-9 px-2 rounded-md bg-background border border-border-soft text-sm text-text"
                    @change="setRewardUnit"
                  >
                    <option value="hour">{{ t('survey_admin.unit_hour') }}</option>
                    <option value="day">{{ t('survey_admin.unit_day') }}</option>
                    <option value="month">{{ t('survey_admin.unit_month') }}</option>
                    <option value="year">{{ t('survey_admin.unit_year') }}</option>
                  </select>
                </label>
              </template>
            </div>
          </div>
        </section>

        <section v-for="(question, questionIndex) in selected.questions" :key="question.question_id" class="border border-border-soft rounded-lg overflow-hidden">
          <div class="p-3 bg-surface-hover/60 border-b border-border-soft flex items-center gap-3">
            <span class="w-7 h-7 rounded-md bg-primary/10 text-primary text-xs font-black grid place-items-center shrink-0">{{ questionIndex + 1 }}</span>
            <input v-model="question.title" class="flex-1 min-w-0 bg-transparent text-sm font-bold text-text-strong outline-none" :placeholder="t('survey_admin.question_title_placeholder')">
            <select :value="question.question_type" class="h-8 px-2 rounded-md bg-surface border border-border-soft text-xs text-text" @change="setQuestionType(question, ($event.target as HTMLSelectElement).value as SurveyQuestionType)">
              <option value="single_choice">{{ t('survey_admin.q_single') }}</option>
              <option value="multiple_choice">{{ t('survey_admin.q_multiple') }}</option>
              <option value="short_text">{{ t('survey_admin.q_short_text') }}</option>
              <option value="long_text">{{ t('survey_admin.q_long_text') }}</option>
            </select>
            <button class="w-8 h-8 grid place-items-center text-red-500 rounded-md hover:bg-red-500/10" :title="t('survey_admin.delete_question')" @click="selected.questions.splice(questionIndex, 1)">
              <Trash2 :size="14" />
            </button>
          </div>

          <div class="p-4 space-y-4">
            <input v-model="question.description" class="w-full h-8 px-3 rounded-md bg-background border border-border-soft text-xs text-text outline-none" :placeholder="t('survey_admin.question_desc_placeholder')">

            <div v-if="question.question_type.includes('choice')" class="space-y-3">
              <div v-for="(option, optionIndex) in question.options" :key="option.option_id" class="space-y-2">
                <div class="flex items-center gap-2">
                  <button
                    class="w-5 h-5 shrink-0 border grid place-items-center"
                    :class="[
                      question.question_type === 'single_choice' ? 'rounded-full' : 'rounded',
                      question.correct_answers.includes(option.option_id) ? 'bg-green-500 border-green-500 text-white' : 'border-border-strong text-transparent',
                    ]"
                    :title="question.require_correct ? t('survey_admin.set_correct_answer') : t('survey_admin.enable_block_first')"
                    @click="toggleCorrect(question, option.option_id)"
                  >
                    <CheckCircle2 :size="12" />
                  </button>
                  <input v-model="option.label" class="flex-1 h-8 px-3 rounded-md bg-background border border-border-soft text-xs text-text outline-none" :placeholder="t('survey_admin.option_placeholder', { index: optionIndex + 1 })">
                  <button class="w-7 h-7 grid place-items-center text-text-muted hover:text-red-500 disabled:opacity-30" :disabled="question.options.length <= 2" :title="t('survey_admin.delete_option')" @click="removeOption(question, option.option_id)">
                    <Trash2 :size="13" />
                  </button>
                </div>
                <div class="pl-7 flex items-center gap-2 flex-wrap">
                  <button class="text-[11px] text-text-muted hover:text-primary flex items-center gap-1" @click="addOptionMedia(option)"><Image :size="12" /> {{ t('survey_admin.option_image') }}</button>
                  <div v-if="option.media && option.media.length" class="flex flex-wrap gap-2">
                    <div v-for="(m, mi) in option.media" :key="mi" class="flex items-center gap-1 bg-background border border-border-soft rounded-md px-1.5 py-1">
                      <span class="text-[10px] text-text-muted truncate max-w-[120px]">{{ m.url || t('survey_admin.link_empty') }}</span>
                      <button class="w-4 h-4 grid place-items-center text-red-500" :title="t('survey_admin.delete_image')" @click="option.media!.splice(mi, 1)"><X :size="11" /></button>
                    </div>
                  </div>
                </div>
                <div v-if="option.media && option.media.length" class="pl-7 space-y-1">
                  <div v-for="(m, mi) in option.media" :key="mi" class="grid grid-cols-[80px_1fr_1fr_28px] gap-1 items-center">
                    <select v-model="m.media_type" class="h-7 px-1 rounded-md bg-background border border-border-soft text-[11px] text-text">
                      <option value="image">{{ t('survey_admin.media_image') }}</option>
                      <option value="video">{{ t('survey_admin.media_video') }}</option>
                    </select>
                    <input v-model="m.url" type="url" class="h-7 px-2 rounded-md bg-background border border-border-soft text-[11px] text-text outline-none" :placeholder="t('survey_admin.url_placeholder')">
                     <input v-model="m.caption" class="h-7 px-2 rounded-md bg-background border border-border-soft text-[11px] text-text outline-none" :placeholder="t('survey_admin.caption_placeholder')">
                    <button class="w-7 h-7 grid place-items-center text-red-500" :title="t('survey_admin.delete_image')" @click="option.media!.splice(mi, 1)"><Trash2 :size="12" /></button>
                  </div>
                  <div v-if="option.media.some((mm) => mm.url)" class="flex flex-wrap gap-2 pt-1">
                    <figure v-for="(pm, previewIndex) in option.media.filter((mm) => mm.url)" :key="'opv' + previewIndex" class="relative">
                      <img v-if="pm.media_type === 'image'" :src="pm.url" referrerpolicy="no-referrer" class="h-20 w-20 object-cover rounded-md border border-border-soft bg-black/5">
                      <video v-else :src="pm.url" controls preload="metadata" class="h-20 rounded-md border border-border-soft bg-black"></video>
                      <figcaption v-if="pm.caption" class="mt-1 text-[10px] text-text-muted max-w-20 truncate">{{ pm.caption }}</figcaption>
                    </figure>
                  </div>
                </div>
              </div>
                <button class="h-8 px-3 rounded-md border border-dashed border-border-strong text-xs text-text-muted hover:text-primary flex items-center gap-1.5" @click="addOption(question)">
                <Plus :size="13" /> {{ t('survey_admin.add_option') }}
              </button>
            </div>

            <label v-else-if="question.require_correct" class="block">
              <span class="block text-[10px] text-text-muted mb-1">{{ t('survey_admin.correct_answers_hint') }}</span>
              <textarea :value="question.correct_answers.join('\n')" rows="2" class="w-full px-3 py-2 rounded-md bg-background border border-border-soft text-xs text-text outline-none" @input="setTextAnswers(question, ($event.target as HTMLTextAreaElement).value)" />
            </label>

            <div v-for="(media, mediaIndex) in question.media" :key="mediaIndex" class="grid grid-cols-[92px_1fr_1fr_32px] gap-2 items-center">
              <select v-model="media.media_type" class="h-8 px-2 rounded-md bg-background border border-border-soft text-xs text-text">
                <option value="image">{{ t('survey_admin.media_image') }}</option>
                <option value="video">{{ t('survey_admin.media_video') }}</option>
              </select>
              <input v-model="media.url" type="url" class="h-8 px-3 rounded-md bg-background border border-border-soft text-xs text-text outline-none" :placeholder="t('survey_admin.url_placeholder')">
              <input v-model="media.caption" class="h-8 px-3 rounded-md bg-background border border-border-soft text-xs text-text outline-none" :placeholder="t('survey_admin.media_caption_placeholder')">
              <button class="w-8 h-8 grid place-items-center text-red-500" :title="t('survey_admin.delete_media')" @click="question.media.splice(mediaIndex, 1)"><Trash2 :size="13" /></button>
            </div>

            <div v-if="question.media.some((m) => m.url)" class="flex flex-wrap gap-3 pt-1">
              <figure v-for="(media, previewIndex) in question.media.filter((m) => m.url)" :key="'pv' + previewIndex" class="relative">
                <img v-if="media.media_type === 'image'" :src="media.url" referrerpolicy="no-referrer" class="h-28 w-28 object-cover rounded-md border border-border-soft bg-black/5">
                <video v-else :src="media.url" controls preload="metadata" class="h-28 rounded-md border border-border-soft bg-black"></video>
                <figcaption v-if="media.caption" class="mt-1 text-[10px] text-text-muted max-w-28 truncate">{{ media.caption }}</figcaption>
              </figure>
            </div>

            <div class="flex flex-wrap items-center gap-4 pt-1">
              <label class="flex items-center gap-2 text-xs text-text cursor-pointer">
                <input v-model="question.required" type="checkbox" class="w-4 h-4 accent-primary"> {{ t('survey_admin.required') }}
              </label>
              <label class="flex items-center gap-2 text-xs text-text cursor-pointer">
                <input v-model="question.require_correct" type="checkbox" class="w-4 h-4 accent-green-500"> {{ t('survey_admin.block_on_wrong') }}
              </label>
              <button class="text-xs text-text-muted hover:text-primary flex items-center gap-1.5" @click="addMedia(question, 'image')"><Image :size="14" /> {{ t('survey_admin.add_image') }}</button>
              <button class="text-xs text-text-muted hover:text-primary flex items-center gap-1.5" @click="addMedia(question, 'video')"><Video :size="14" /> {{ t('survey_admin.add_video') }}</button>
            </div>
          </div>
        </section>

        <button class="w-full h-10 border border-dashed border-border-strong rounded-lg text-xs font-bold text-text-muted hover:text-primary hover:border-primary flex items-center justify-center gap-2" @click="selected.questions.push(blankQuestion())">
          <Plus :size="15" /> {{ t('survey_admin.add_question') }}
        </button>
      </div>

      <div v-if="message" class="absolute bottom-6 right-6 px-4 py-2 rounded-md bg-text-strong text-background text-xs font-bold shadow-xl">
        {{ message }}
      </div>

    </main>

    <div v-else class="flex-1 border border-border-soft rounded-lg bg-surface grid place-items-center text-center">
      <div>
        <FileQuestion :size="44" class="mx-auto text-text-muted opacity-40" />
        <p class="mt-3 text-sm font-bold text-text-strong">{{ t('survey_admin.select_or_create') }}</p>
        <p class="mt-1 text-xs text-text-muted">{{ t('survey_admin.publish_hint') }}</p>
      </div>
    </div>
    </div>

    <!-- ═══ 独立答卷记录工作区 ═══ -->
    <div v-else class="flex-1 min-h-0 grid grid-cols-[220px_280px_minmax(0,1fr)] gap-3">
      <aside class="min-h-0 border border-border-soft rounded-lg bg-surface overflow-hidden flex flex-col">
        <header class="p-3 border-b border-border-soft">
          <div class="text-xs font-black text-text-strong">{{ t('survey_admin.filter_by_survey') }}</div>
          <div class="text-[10px] text-text-muted mt-1">{{ t('survey_admin.list_summary', { surveyCount: surveys.length, submissionCount: submissions.length }) }}</div>
        </header>
        <div class="flex-1 min-h-0 overflow-y-auto p-2 space-y-1">
          <button
            v-for="survey in surveys"
            :key="survey.survey_id"
            class="w-full text-left p-3 rounded-md border transition-colors"
            :class="selected?.survey_id === survey.survey_id ? 'bg-primary/10 border-primary/40' : 'border-transparent hover:bg-surface-hover'"
            @click="selectSurvey(survey)"
          >
            <div class="text-xs font-bold text-text-strong truncate">{{ survey.title }}</div>
            <div class="text-[10px] text-text-muted mt-1">{{ t('survey_admin.revision_answers', { revision: survey.revision, count: submissionCounts[survey.survey_id] || 0 }) }}</div>
          </button>
          <div v-if="surveys.length === 0" class="py-10 text-center text-xs text-text-muted">{{ t('survey_admin.no_surveys') }}</div>
        </div>
      </aside>

      <section class="min-h-0 border border-border-soft rounded-lg bg-surface overflow-hidden flex flex-col">
        <header class="p-3 border-b border-border-soft flex items-center justify-between gap-3">
          <div class="min-w-0">
            <div class="text-xs font-black text-text-strong">{{ t('survey_admin.submission_list') }}</div>
            <div class="text-[10px] text-text-muted mt-1 truncate">{{ t('survey_admin.list_header', { title: selected?.title || t('survey_admin.please_select_survey'), count: surveySubmissions.length }) }}</div>
          </div>
          <button
            class="h-8 px-3 rounded-md bg-primary text-white text-[10px] font-bold flex items-center gap-1.5 shrink-0 disabled:opacity-40 disabled:cursor-not-allowed"
            :disabled="!selected || surveySubmissions.length === 0 || exporting"
            :title="t('survey_admin.export_current_tooltip')"
            @click="exportCurrentSurvey"
          >
            <RefreshCcw v-if="exporting" :size="13" class="animate-spin" />
            <Download v-else :size="13" />
            {{ exporting ? t('survey_admin.exporting') : t('survey_admin.export_excel') }}
          </button>
        </header>
        <div class="flex-1 min-h-0 overflow-y-auto p-2 space-y-2">
          <div
            v-for="submission in pagedSubmissions"
            :key="submission.submission_id"
            class="group relative w-full text-left p-3 pr-10 rounded-md border transition-colors cursor-pointer"
            :class="selectedSubmission?.submission_id === submission.submission_id ? 'bg-primary/10 border-primary/40' : 'border-border-soft bg-background hover:border-primary/30'"
            @click="openSubmissionDetail(submission)"
          >
            <div class="flex items-center justify-between gap-2">
              <span class="text-xs font-bold text-text-strong truncate">{{ userName(submission.user_id) }}</span>
              <span
                class="px-1.5 py-0.5 rounded text-[9px] font-bold shrink-0"
                :class="submission.passed ? 'bg-green-500/15 text-green-500' : submission.status === 'dismissed' ? 'bg-gray-500/15 text-gray-500' : 'bg-red-500/15 text-red-500'"
              >
                {{ submission.passed ? t('survey_admin.passed') : submission.status === 'dismissed' ? t('survey_admin.dismissed') : t('survey_admin.failed') }}
              </span>
            </div>
            <div v-if="hasUserName(submission.user_id)" class="text-[9px] text-text-muted mt-1 font-mono truncate">{{ submission.user_id }}</div>
            <div class="text-[10px] text-text-muted mt-1">{{ submission.submitted_at }}</div>
              <div class="text-[10px] text-text-muted mt-0.5">{{ t('survey_admin.submission_version', { revision: submission.survey_revision }) }}</div>
            <button
              class="absolute right-2 top-2 w-7 h-7 grid place-items-center rounded-md text-text-muted opacity-0 group-hover:opacity-100 hover:bg-red-500/10 hover:text-red-500 transition-colors"
              :title="t('survey_admin.delete_this_submission')"
              @click.stop="deleteSubmission(submission)"
            >
              <Trash2 :size="14" />
            </button>
          </div>
          <div v-if="selected && surveySubmissions.length === 0" class="py-10 text-center text-xs text-text-muted">{{ t('survey_admin.no_submissions') }}</div>
          <div v-else-if="!selected" class="py-10 text-center text-xs text-text-muted">{{ t('survey_admin.select_survey_left') }}</div>
        </div>
        <div v-if="surveySubmissions.length > SUBMISSIONS_PAGE_SIZE" class="shrink-0 flex items-center justify-between px-3 py-2 border-t border-border-soft text-[10px] text-text-muted">
          <span>{{ t('survey_admin.page_info', { count: surveySubmissions.length, page: Math.min(submissionsPage, totalSubmissionPages), total: totalSubmissionPages }) }}</span>
          <div class="flex gap-2">
            <button
              class="px-2.5 py-1 rounded border border-border-soft text-text-muted hover:text-primary hover:border-primary/40 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
              :disabled="submissionsPage <= 1"
              @click="submissionsPage--"
            >{{ t('survey_admin.prev_page') }}</button>
            <button
              class="px-2.5 py-1 rounded border border-border-soft text-text-muted hover:text-primary hover:border-primary/40 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
              :disabled="submissionsPage >= totalSubmissionPages"
              @click="submissionsPage++"
            >{{ t('survey_admin.next_page') }}</button>
          </div>
        </div>
      </section>

      <article class="min-h-0 border border-border-soft rounded-lg bg-surface overflow-hidden flex flex-col">
        <template v-if="selectedSubmission">
          <header class="px-5 py-4 border-b border-border-soft flex items-center justify-between gap-3 shrink-0">
            <div class="min-w-0">
              <h2 class="text-sm font-black text-text-strong truncate">{{ userName(selectedSubmission.user_id) }}</h2>
              <p class="text-[10px] text-text-muted mt-1 truncate">
                <span v-if="hasUserName(selectedSubmission.user_id)" class="font-mono">{{ selectedSubmission.user_id }} · </span>{{ selectedSubmission.submitted_at }}
              </p>
            </div>
            <span
              class="px-2 py-1 rounded text-[10px] font-bold shrink-0"
              :class="selectedSubmission.passed ? 'bg-green-500/15 text-green-500' : selectedSubmission.status === 'dismissed' ? 'bg-gray-500/15 text-gray-500' : 'bg-red-500/15 text-red-500'"
            >
              {{ selectedSubmission.passed ? t('survey_admin.passed') : selectedSubmission.status === 'dismissed' ? t('survey_admin.dismissed') : t('survey_admin.failed') }}
            </span>
          </header>
          <div class="flex-1 min-h-0 overflow-y-auto p-5 space-y-3">
            <div
              v-for="(answer, questionId) in selectedSubmission.answers"
              :key="questionId"
              class="p-3 rounded-md border border-border-soft bg-background"
              :class="{ 'border-red-500/40 bg-red-500/5': selectedSubmission.failed_question_ids.includes(questionId) }"
            >
              <div class="flex items-start justify-between gap-3 mb-2">
                <div class="text-xs font-bold text-text-strong">{{ questionTitle(questionId) }}</div>
                <span v-if="selectedSubmission.failed_question_ids.includes(questionId)" class="px-1.5 py-0.5 rounded text-[9px] font-bold bg-red-500/15 text-red-500 shrink-0">{{ t('survey_admin.wrong') }}</span>
              </div>
              <div class="text-xs text-text leading-relaxed whitespace-pre-wrap break-words">{{ formatAnswer(answer, questionId) }}</div>
              <div v-if="selectedSubmission.answer_files[questionId] && selectedSubmission.answer_files[questionId].length" class="mt-2 flex flex-wrap gap-2">
                <a
                  v-for="file in selectedSubmission.answer_files[questionId]"
                  :key="file.file_id"
                  :href="props.serverUrl + file.url"
                  target="_blank"
                  rel="noopener"
                  class="relative shrink-0"
                >
                  <img v-if="file.mime_type.startsWith('image/')" :src="props.serverUrl + file.url" class="h-24 w-24 object-cover rounded border border-border-soft" :alt="file.file_name">
                  <span v-else class="h-24 px-2 grid place-items-center rounded border border-border-soft text-[10px] text-text-muted max-w-[96px] truncate">{{ file.file_name }}</span>
                </a>
              </div>
            </div>
            <div v-if="Object.keys(selectedSubmission.answers).length === 0 && submissionClicks.length === 0" class="py-10 text-center text-xs text-text-muted">{{ t('survey_admin.no_answers') }}</div>

            <!-- 答题点击记录：展示用户答题过程中的每次选择/取消/输入 -->
            <div v-if="submissionClicks.length" class="mt-4">
              <div class="text-xs font-black text-text-strong mb-2">{{ t('survey_admin.click_records', { count: submissionClicks.length }) }}</div>
              <div class="border border-border-soft rounded-md bg-background overflow-hidden">
                <div
                  v-for="event in submissionClicks"
                  :key="event.event_id"
                  class="px-3 py-2 border-b border-border-soft last:border-0 flex items-start gap-3"
                >
                  <span
                    class="shrink-0 mt-0.5 px-1.5 py-0.5 rounded text-[9px] font-bold"
                    :class="event.action === 'deselect' ? 'bg-gray-500/15 text-gray-500' : event.action === 'input' ? 'bg-blue-500/15 text-blue-500' : 'bg-green-500/15 text-green-500'"
                  >
                    {{ event.action === 'deselect' ? t('survey_admin.action_deselect') : event.action === 'input' ? t('survey_admin.action_input') : t('survey_admin.action_click') }}
                  </span>
                  <div class="min-w-0 flex-1">
                    <div class="text-[11px] font-bold text-text-strong truncate">{{ event.question_title || event.question_id }}</div>
                    <div class="text-[11px] text-text mt-0.5 break-words">{{ clickActionLabel(event) }}</div>
                  </div>
                  <span class="shrink-0 text-[9px] text-text-muted font-mono">{{ event.clicked_at }}</span>
                </div>
              </div>
            </div>
          </div>
          <footer class="px-5 py-3 border-t border-border-soft text-[10px] text-text-muted flex items-center justify-between gap-3 shrink-0">
            <span class="truncate">{{ t('survey_admin.submission_id') }}{{ selectedSubmission.submission_id }}</span>
            <div class="flex items-center gap-3 shrink-0">
              <span v-if="selectedSubmission.failed_question_ids.length > 0" class="text-red-500">{{ t('survey_admin.questions_wrong', { count: selectedSubmission.failed_question_ids.length }) }}</span>
              <button
                class="flex items-center gap-1 px-2 py-1 rounded-md border border-red-500/30 text-red-500 hover:bg-red-500/10 transition-colors"
                @click="deleteSubmission(selectedSubmission)"
              >
                <Trash2 :size="13" /> {{ t('survey_admin.delete_this_submission') }}
              </button>
            </div>
          </footer>
        </template>
        <div v-else class="h-full grid place-items-center text-center p-6">
          <div>
            <ClipboardList :size="44" class="mx-auto text-text-muted opacity-40" />
            <p class="mt-3 text-sm font-bold text-text-strong">{{ t('survey_admin.select_submission') }}</p>
            <p class="mt-1 text-xs text-text-muted">{{ t('survey_admin.submission_detail_hint') }}</p>
          </div>
        </div>
      </article>
    </div>

    <div v-if="message" class="absolute bottom-6 right-6 px-4 py-2 rounded-md bg-text-strong text-background text-xs font-bold shadow-xl z-50">
      {{ message }}
    </div>
  </div>
</template>
