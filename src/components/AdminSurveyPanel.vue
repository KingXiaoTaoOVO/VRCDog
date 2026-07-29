<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
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
} from 'lucide-vue-next';
import { SysApi, VrcApi } from '../api';
import { buildSurveyWorkbook, surveyExportFileName } from '../utils/surveyExcel';
import type {
  Survey,
  SurveyMediaType,
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
const surveys = ref<Survey[]>([]);
const submissions = ref<SurveySubmission[]>([]);
const users = ref<Array<{ user_id: string; display_name: string }>>([]);
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
      { option_id: newId('option'), label: '' },
      { option_id: newId('option'), label: '' },
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
    const [settingsData, surveyData, submissionData, userData] = await Promise.all([
      request('/api/admin/survey-settings'),
      request('/api/admin/surveys'),
      request('/api/admin/survey-submissions'),
      request('/api/admin/users'),
    ]);
    enabled.value = Boolean(settingsData?.enabled);
    surveys.value = surveyData?.surveys || [];
    submissions.value = submissionData?.submissions || [];
    users.value = userData?.users || [];
    if (selected.value) {
      const fresh = surveys.value.find((item) => item.survey_id === selected.value?.survey_id);
      // Same fix: use JSON round-trip instead of structuredClone to handle proxies.
      if (fresh) selected.value = JSON.parse(JSON.stringify(fresh));
    }
  } catch (error: any) {
    notify(`问卷数据加载失败：${error?.message || error}`, true);
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
    notify(enabled.value ? '客户端问卷提示已开启' : '客户端问卷提示已关闭');
  } catch (error: any) {
    enabled.value = !enabled.value;
    notify(`设置保存失败：${error?.message || error}`, true);
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
    if (!data?.success) throw new Error(data?.message || '保存失败');
    notify(selected.value.status === 'published' ? '修改已保存，并作为新版本发送' : '问卷草稿已保存');
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
    if (!data?.success) throw new Error(data?.message || '发布失败');
    notify('问卷已发布，客户端将在登录或下一次心跳时收到提示');
    await fetchData();
  } catch (error: any) {
    notify(error?.message || String(error), true);
  }
};

const resendSurvey = async () => {
  if (!selected.value || !confirm('重新发送会生成新版本，所有用户都需要再次填写。继续吗？')) return;
  try {
    const data = await request('/api/admin/surveys/resend', {
      method: 'POST',
      params: { survey_id: selected.value.survey_id },
    });
    if (!data?.success) throw new Error(data?.message || '重新发送失败');
    notify(`已重新发送问卷（版本 ${data.revision}）`);
    await fetchData();
  } catch (error: any) {
    notify(error?.message || String(error), true);
  }
};

const deleteSurvey = async () => {
  if (!selected.value || !confirm('删除问卷会同时删除该问卷的所有提交记录，且无法恢复。继续吗？')) return;
  try {
    const data = await request('/api/admin/surveys/delete', {
      method: 'POST',
      params: { survey_id: selected.value.survey_id },
    });
    if (!data?.success) throw new Error(data?.message || '删除失败');
    selected.value = null;
    notify('问卷及其提交记录已删除');
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
      { option_id: newId('option'), label: '' },
      { option_id: newId('option'), label: '' },
    ]
    : [];
};

const addOption = (question: SurveyQuestion) => {
  question.options.push({ option_id: newId('option'), label: '' });
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

const submissionCount = (surveyId: string) => submissions.value.filter(
  (submission) => submission.survey_id === surveyId,
).length;

const surveySubmissions = computed(() => {
  if (!selected.value) return [];
  return submissions.value
    .filter((s) => s.survey_id === selected.value!.survey_id)
    .sort((a, b) => b.submitted_at.localeCompare(a.submitted_at));
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

// Resolve a question_id to its title for display in the detail modal
const questionTitle = (questionId: string) => {
  if (!selected.value) return questionId;
  const q = selected.value.questions.find((q) => q.question_id === questionId);
  return q?.title || questionId;
};

const formatAnswer = (answer: string | string[] | undefined, questionId?: string) => {
  if (answer === undefined || answer === null) return '(未作答)';
  const question = questionId
    ? selected.value?.questions.find((item) => item.question_id === questionId)
    : undefined;
  const values = Array.isArray(answer) ? answer : [answer];
  const labels = values.map((value) => {
    const text = String(value).trim();
    if (!text) return '';
    return question?.options.find((option) => option.option_id === text)?.label || text;
  }).filter(Boolean);
  if (labels.length === 0) return Array.isArray(answer) ? '(未选择)' : '(空白)';
  return labels.join('、');
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
        filters: [{ name: 'Excel 工作簿', extensions: ['xlsx'] }],
        defaultPath: fileName,
      });
      if (!filePath) {
        notify('已取消导出');
        return;
      }
      await SysApi.saveBinaryFile({ path: filePath, content: Array.from(content) });
      notify(`已导出 ${surveySubmissions.value.length} 份答卷`);
    } catch (error) {
      console.warn('Tauri Excel save failed, using browser download fallback:', error);
      downloadWorkbookFallback(content, fileName);
      notify(`已导出 ${surveySubmissions.value.length} 份答卷`);
    }
  } catch (error: any) {
    notify(`Excel 导出失败：${error?.message || error}`, true);
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
          <FileQuestion :size="15" /> 问卷设计
        </button>
        <button
          class="h-9 px-4 rounded-md text-xs font-black flex items-center gap-2 transition-colors"
          :class="workspaceMode === 'responses' ? 'bg-primary text-white shadow-sm' : 'text-text-muted hover:bg-surface-hover hover:text-text'"
          @click="setWorkspaceMode('responses')"
        >
          <ClipboardList :size="15" /> 答卷记录
          <span class="px-1.5 py-0.5 rounded bg-black/10 text-[9px]">{{ submissions.length }}</span>
        </button>
      </div>
      <div class="pr-3 text-[10px] text-text-muted">
        {{ workspaceMode === 'design' ? '创建、编辑和发布客户端问卷' : '独立查看用户提交内容和答题结果' }}
      </div>
    </nav>

    <div v-if="workspaceMode === 'design'" class="flex-1 min-h-0 flex gap-4">
      <aside class="w-64 shrink-0 border border-border-soft rounded-lg bg-surface overflow-hidden flex flex-col">
      <div class="p-3 border-b border-border-soft flex items-center justify-between gap-2">
        <div>
          <div class="text-xs font-black text-text-strong">问卷列表</div>
          <div class="text-[10px] text-text-muted mt-1">{{ surveys.length }} 份问卷 · {{ submissions.length }} 次提交</div>
        </div>
        <button class="w-8 h-8 grid place-items-center rounded-md bg-primary text-white" title="新建问卷" @click="createSurvey">
          <Plus :size="16" />
        </button>
      </div>

      <label class="m-3 p-3 border border-border-soft rounded-md flex items-center justify-between gap-3 cursor-pointer">
        <span>
          <span class="block text-xs font-bold text-text">客户端问卷</span>
          <span class="block text-[10px] text-text-muted mt-1">关闭后不提示也不阻断</span>
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
          <span class="block text-[10px] text-text-muted mt-1.5">v{{ survey.revision }} · {{ submissionCount(survey.survey_id) }} 次提交</span>
        </button>
        <div v-if="!loading && surveys.length === 0" class="py-12 text-center text-xs text-text-muted">
          暂无问卷
        </div>
      </div>
    </aside>

    <main v-if="selected" class="flex-1 min-w-0 border border-border-soft rounded-lg bg-surface overflow-hidden flex flex-col">
      <header class="px-5 py-3 border-b border-border-soft flex items-center justify-between gap-4">
        <div class="min-w-0">
          <div class="flex items-center gap-2">
            <FileQuestion :size="17" class="text-primary shrink-0" />
            <h2 class="text-sm font-black text-text-strong truncate">{{ selected.title || '未命名问卷' }}</h2>
            <span class="px-2 py-0.5 rounded text-[10px] font-bold" :class="selected.status === 'published' ? 'bg-green-500/15 text-green-500' : 'bg-amber-500/15 text-amber-500'">
              {{ selected.status === 'published' ? `已发布 v${selected.revision}` : '草稿' }}
            </span>
          </div>
          <p v-if="selected.status === 'published'" class="text-[10px] text-amber-500 mt-1">修改已发布问卷并保存时，会自动生成新版本并再次提示用户。</p>
        </div>
        <div class="flex items-center gap-2 shrink-0">
          <button class="h-8 px-3 rounded-md border border-border-soft text-xs font-bold text-text hover:bg-surface-hover flex items-center gap-1.5" :disabled="saving" @click="saveSurvey">
            <Save :size="14" /> 保存
          </button>
          <button v-if="selected.status === 'draft'" class="h-8 px-3 rounded-md bg-primary text-white text-xs font-bold flex items-center gap-1.5" @click="publishSurvey">
            <Send :size="14" /> 发布
          </button>
          <button v-else class="h-8 px-3 rounded-md bg-primary text-white text-xs font-bold flex items-center gap-1.5" @click="resendSurvey">
            <RefreshCcw :size="14" /> 重新发送
          </button>
          <button class="w-8 h-8 grid place-items-center rounded-md border border-red-500/30 text-red-500 hover:bg-red-500/10" title="删除问卷" @click="deleteSurvey">
            <Trash2 :size="15" />
          </button>
        </div>
      </header>

      <div class="flex-1 min-h-0 overflow-y-auto p-5 space-y-5">
        <section class="grid grid-cols-[1fr_220px] gap-4 border-b border-border-soft pb-5">
          <div class="space-y-3">
            <label class="block">
              <span class="block text-[11px] font-bold text-text-muted mb-1">问卷标题</span>
              <input v-model="selected.title" class="w-full h-9 px-3 rounded-md bg-background border border-border-soft text-sm text-text outline-none focus:border-primary" placeholder="例如：产品使用资格确认">
            </label>
            <label class="block">
              <span class="block text-[11px] font-bold text-text-muted mb-1">说明</span>
              <textarea v-model="selected.description" rows="2" class="w-full px-3 py-2 rounded-md bg-background border border-border-soft text-xs text-text outline-none resize-y focus:border-primary" placeholder="填写目的、隐私说明或注意事项" />
            </label>
          </div>
          <label class="self-end min-h-20 p-3 rounded-md border border-border-soft flex items-start gap-3 cursor-pointer">
            <input v-model="selected.required_for_access" type="checkbox" class="mt-0.5 w-4 h-4 accent-primary">
            <span>
              <span class="block text-xs font-bold text-text-strong">作为使用门禁</span>
              <span class="block text-[10px] leading-4 text-text-muted mt-1">开启后，必答题未完成或门禁题答错时不能进入产品。</span>
            </span>
          </label>
        </section>

        <section v-for="(question, questionIndex) in selected.questions" :key="question.question_id" class="border border-border-soft rounded-lg overflow-hidden">
          <div class="p-3 bg-surface-hover/60 border-b border-border-soft flex items-center gap-3">
            <span class="w-7 h-7 rounded-md bg-primary/10 text-primary text-xs font-black grid place-items-center shrink-0">{{ questionIndex + 1 }}</span>
            <input v-model="question.title" class="flex-1 min-w-0 bg-transparent text-sm font-bold text-text-strong outline-none" placeholder="请输入题目">
            <select :value="question.question_type" class="h-8 px-2 rounded-md bg-surface border border-border-soft text-xs text-text" @change="setQuestionType(question, ($event.target as HTMLSelectElement).value as SurveyQuestionType)">
              <option value="single_choice">单选题</option>
              <option value="multiple_choice">多选题</option>
              <option value="short_text">填空题</option>
              <option value="long_text">意见/长文本</option>
            </select>
            <button class="w-8 h-8 grid place-items-center text-red-500 rounded-md hover:bg-red-500/10" title="删除题目" @click="selected.questions.splice(questionIndex, 1)">
              <Trash2 :size="14" />
            </button>
          </div>

          <div class="p-4 space-y-4">
            <input v-model="question.description" class="w-full h-8 px-3 rounded-md bg-background border border-border-soft text-xs text-text outline-none" placeholder="题目补充说明（可选）">

            <div v-if="question.question_type.includes('choice')" class="space-y-2">
              <div v-for="(option, optionIndex) in question.options" :key="option.option_id" class="flex items-center gap-2">
                <button
                  class="w-5 h-5 shrink-0 border grid place-items-center"
                  :class="[
                    question.question_type === 'single_choice' ? 'rounded-full' : 'rounded',
                    question.correct_answers.includes(option.option_id) ? 'bg-green-500 border-green-500 text-white' : 'border-border-strong text-transparent',
                  ]"
                  :title="question.require_correct ? '设为正确答案' : '开启“答错阻断”后可设置正确答案'"
                  @click="toggleCorrect(question, option.option_id)"
                >
                  <CheckCircle2 :size="12" />
                </button>
                <input v-model="option.label" class="flex-1 h-8 px-3 rounded-md bg-background border border-border-soft text-xs text-text outline-none" :placeholder="`选项 ${optionIndex + 1}`">
                <button class="w-7 h-7 grid place-items-center text-text-muted hover:text-red-500 disabled:opacity-30" :disabled="question.options.length <= 2" title="删除选项" @click="removeOption(question, option.option_id)">
                  <Trash2 :size="13" />
                </button>
              </div>
              <button class="h-8 px-3 rounded-md border border-dashed border-border-strong text-xs text-text-muted hover:text-primary flex items-center gap-1.5" @click="addOption(question)">
                <Plus :size="13" /> 添加选项
              </button>
            </div>

            <label v-else-if="question.require_correct" class="block">
              <span class="block text-[10px] text-text-muted mb-1">可接受的正确答案（每行一个，不区分大小写）</span>
              <textarea :value="question.correct_answers.join('\n')" rows="2" class="w-full px-3 py-2 rounded-md bg-background border border-border-soft text-xs text-text outline-none" @input="setTextAnswers(question, ($event.target as HTMLTextAreaElement).value)" />
            </label>

            <div v-for="(media, mediaIndex) in question.media" :key="mediaIndex" class="grid grid-cols-[92px_1fr_1fr_32px] gap-2 items-center">
              <select v-model="media.media_type" class="h-8 px-2 rounded-md bg-background border border-border-soft text-xs text-text">
                <option value="image">图片</option>
                <option value="video">视频</option>
              </select>
              <input v-model="media.url" type="url" class="h-8 px-3 rounded-md bg-background border border-border-soft text-xs text-text outline-none" placeholder="https://...">
              <input v-model="media.caption" class="h-8 px-3 rounded-md bg-background border border-border-soft text-xs text-text outline-none" placeholder="媒体说明（可选）">
              <button class="w-8 h-8 grid place-items-center text-red-500" title="删除媒体" @click="question.media.splice(mediaIndex, 1)"><Trash2 :size="13" /></button>
            </div>

            <div class="flex flex-wrap items-center gap-4 pt-1">
              <label class="flex items-center gap-2 text-xs text-text cursor-pointer">
                <input v-model="question.required" type="checkbox" class="w-4 h-4 accent-primary"> 必答
              </label>
              <label class="flex items-center gap-2 text-xs text-text cursor-pointer">
                <input v-model="question.require_correct" type="checkbox" class="w-4 h-4 accent-green-500"> 答错阻断
              </label>
              <button class="text-xs text-text-muted hover:text-primary flex items-center gap-1.5" @click="addMedia(question, 'image')"><Image :size="14" /> 添加图片</button>
              <button class="text-xs text-text-muted hover:text-primary flex items-center gap-1.5" @click="addMedia(question, 'video')"><Video :size="14" /> 添加视频</button>
            </div>
          </div>
        </section>

        <button class="w-full h-10 border border-dashed border-border-strong rounded-lg text-xs font-bold text-text-muted hover:text-primary hover:border-primary flex items-center justify-center gap-2" @click="selected.questions.push(blankQuestion())">
          <Plus :size="15" /> 添加题目
        </button>
      </div>

      <div v-if="message" class="absolute bottom-6 right-6 px-4 py-2 rounded-md bg-text-strong text-background text-xs font-bold shadow-xl">
        {{ message }}
      </div>

    </main>

    <div v-else class="flex-1 border border-border-soft rounded-lg bg-surface grid place-items-center text-center">
      <div>
        <FileQuestion :size="44" class="mx-auto text-text-muted opacity-40" />
        <p class="mt-3 text-sm font-bold text-text-strong">选择一份问卷，或新建问卷</p>
        <p class="mt-1 text-xs text-text-muted">发布后客户端会按问卷版本逐次提示。</p>
      </div>
    </div>
    </div>

    <!-- ═══ 独立答卷记录工作区 ═══ -->
    <div v-else class="flex-1 min-h-0 grid grid-cols-[220px_280px_minmax(0,1fr)] gap-3">
      <aside class="min-h-0 border border-border-soft rounded-lg bg-surface overflow-hidden flex flex-col">
        <header class="p-3 border-b border-border-soft">
          <div class="text-xs font-black text-text-strong">按问卷筛选</div>
          <div class="text-[10px] text-text-muted mt-1">{{ surveys.length }} 份问卷 · {{ submissions.length }} 次提交</div>
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
            <div class="text-[10px] text-text-muted mt-1">v{{ survey.revision }} · {{ submissionCount(survey.survey_id) }} 份答卷</div>
          </button>
          <div v-if="surveys.length === 0" class="py-10 text-center text-xs text-text-muted">暂无问卷</div>
        </div>
      </aside>

      <section class="min-h-0 border border-border-soft rounded-lg bg-surface overflow-hidden flex flex-col">
        <header class="p-3 border-b border-border-soft flex items-center justify-between gap-3">
          <div class="min-w-0">
            <div class="text-xs font-black text-text-strong">提交列表</div>
            <div class="text-[10px] text-text-muted mt-1 truncate">{{ selected?.title || '请先选择问卷' }} · {{ surveySubmissions.length }} 条</div>
          </div>
          <button
            class="h-8 px-3 rounded-md bg-primary text-white text-[10px] font-bold flex items-center gap-1.5 shrink-0 disabled:opacity-40 disabled:cursor-not-allowed"
            :disabled="!selected || surveySubmissions.length === 0 || exporting"
            title="将当前问卷的全部答卷导出为 Excel"
            @click="exportCurrentSurvey"
          >
            <RefreshCcw v-if="exporting" :size="13" class="animate-spin" />
            <Download v-else :size="13" />
            {{ exporting ? '导出中' : '导出 Excel' }}
          </button>
        </header>
        <div class="flex-1 min-h-0 overflow-y-auto p-2 space-y-2">
          <button
            v-for="submission in surveySubmissions"
            :key="submission.submission_id"
            class="w-full text-left p-3 rounded-md border transition-colors"
            :class="selectedSubmission?.submission_id === submission.submission_id ? 'bg-primary/10 border-primary/40' : 'border-border-soft bg-background hover:border-primary/30'"
            @click="openSubmissionDetail(submission)"
          >
            <div class="flex items-center justify-between gap-2">
              <span class="text-xs font-bold text-text-strong truncate">{{ userName(submission.user_id) }}</span>
              <span
                class="px-1.5 py-0.5 rounded text-[9px] font-bold shrink-0"
                :class="submission.passed ? 'bg-green-500/15 text-green-500' : submission.status === 'dismissed' ? 'bg-gray-500/15 text-gray-500' : 'bg-red-500/15 text-red-500'"
              >
                {{ submission.passed ? '通过' : submission.status === 'dismissed' ? '已忽略' : '未通过' }}
              </span>
            </div>
            <div v-if="hasUserName(submission.user_id)" class="text-[9px] text-text-muted mt-1 font-mono truncate">{{ submission.user_id }}</div>
            <div class="text-[10px] text-text-muted mt-1">{{ submission.submitted_at }}</div>
            <div class="text-[10px] text-text-muted mt-0.5">问卷版本 v{{ submission.survey_revision }}</div>
          </button>
          <div v-if="selected && surveySubmissions.length === 0" class="py-10 text-center text-xs text-text-muted">该问卷暂无提交记录</div>
          <div v-else-if="!selected" class="py-10 text-center text-xs text-text-muted">请从左侧选择问卷</div>
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
              {{ selectedSubmission.passed ? '已通过' : selectedSubmission.status === 'dismissed' ? '已忽略' : '未通过' }}
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
                <span v-if="selectedSubmission.failed_question_ids.includes(questionId)" class="px-1.5 py-0.5 rounded text-[9px] font-bold bg-red-500/15 text-red-500 shrink-0">答错</span>
              </div>
              <div class="text-xs text-text leading-relaxed whitespace-pre-wrap break-words">{{ formatAnswer(answer, questionId) }}</div>
            </div>
            <div v-if="Object.keys(selectedSubmission.answers).length === 0" class="py-10 text-center text-xs text-text-muted">该提交没有任何答案记录</div>
          </div>
          <footer class="px-5 py-3 border-t border-border-soft text-[10px] text-text-muted flex items-center justify-between gap-3 shrink-0">
            <span class="truncate">提交 ID：{{ selectedSubmission.submission_id }}</span>
            <span v-if="selectedSubmission.failed_question_ids.length > 0" class="text-red-500 shrink-0">{{ selectedSubmission.failed_question_ids.length }} 题答错</span>
          </footer>
        </template>
        <div v-else class="h-full grid place-items-center text-center p-6">
          <div>
            <ClipboardList :size="44" class="mx-auto text-text-muted opacity-40" />
            <p class="mt-3 text-sm font-bold text-text-strong">选择一条提交记录</p>
            <p class="mt-1 text-xs text-text-muted">这里会显示用户的逐题答案和判定结果。</p>
          </div>
        </div>
      </article>
    </div>

    <div v-if="message" class="absolute bottom-6 right-6 px-4 py-2 rounded-md bg-text-strong text-background text-xs font-bold shadow-xl z-50">
      {{ message }}
    </div>
  </div>
</template>
