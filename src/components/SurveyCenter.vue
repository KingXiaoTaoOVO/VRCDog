<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import {
  CheckCircle2,
  ChevronLeft,
  ClipboardList,
  Clock3,
  History,
  Loader2,
  Send,
  Trash2,
  X,
  XCircle,
} from 'lucide-vue-next';
import { VrcApi } from '../api';
import type { Survey, SurveySubmission } from '../types/survey';

const props = withDefaults(defineProps<{
  serverUrl: string;
  userId: string;
  forced?: boolean;
  initialTab?: 'pending' | 'history';
}>(), {
  forced: false,
  initialTab: 'pending',
});

const emit = defineEmits<{
  close: [];
  resolved: [pendingCount: number, required: boolean];
}>();

const activeTab = ref<'pending' | 'history'>(props.initialTab);
const surveys = ref<Survey[]>([]);
const history = ref<SurveySubmission[]>([]);
const answers = ref<Record<string, string | string[]>>({});
const failedQuestionIds = ref<string[]>([]);
const loading = ref(true);
const submitting = ref(false);
const message = ref('');
const error = ref('');

const currentSurvey = computed(() => surveys.value[0] || null);
const currentIsBlocking = computed(() => Boolean(currentSurvey.value?.required_for_access));
const remainingRequired = computed(() => surveys.value.some((survey) => survey.required_for_access));
const endpoint = (path: string) => `${props.serverUrl.replace(/\/+$/, '')}${path}`;

const resetAnswers = () => {
  const next: Record<string, string | string[]> = {};
  for (const question of currentSurvey.value?.questions || []) {
    next[question.question_id] = question.question_type === 'multiple_choice' ? [] : '';
  }
  answers.value = next;
  failedQuestionIds.value = [];
  message.value = '';
  error.value = '';
};

watch(() => currentSurvey.value?.survey_id, resetAnswers);

const fetchPending = async () => {
  const data = await VrcApi.request(endpoint(`/api/client/surveys/${encodeURIComponent(props.userId)}`), {
    method: 'GET',
  });
  surveys.value = data?.surveys || [];
  emit('resolved', surveys.value.length, surveys.value.some((survey: Survey) => survey.required_for_access));
};

const fetchHistory = async () => {
  const data = await VrcApi.request(endpoint(`/api/client/survey-history/${encodeURIComponent(props.userId)}`), {
    method: 'GET',
  });
  history.value = data?.submissions || [];
};

const load = async () => {
  loading.value = true;
  error.value = '';
  try {
    await Promise.all([fetchPending(), fetchHistory()]);
    if (surveys.value.length === 0 && activeTab.value === 'pending' && props.initialTab === 'history') {
      activeTab.value = 'history';
    }
  } catch (loadError: any) {
    error.value = `问卷加载失败：${loadError?.message || loadError}`;
  } finally {
    loading.value = false;
  }
};

const toggleMultiple = (questionId: string, optionId: string) => {
  const current = Array.isArray(answers.value[questionId])
    ? answers.value[questionId] as string[]
    : [];
  answers.value[questionId] = current.includes(optionId)
    ? current.filter((value) => value !== optionId)
    : [...current, optionId];
};

const isMultipleChecked = (questionId: string, optionId: string) => {
  const value = answers.value[questionId];
  return Array.isArray(value) && value.includes(optionId);
};

const submit = async () => {
  if (!currentSurvey.value) return;
  submitting.value = true;
  message.value = '';
  error.value = '';
  try {
    const data = await VrcApi.request(endpoint('/api/client/surveys/submit'), {
      method: 'POST',
      params: {
        user_id: props.userId,
        survey_id: currentSurvey.value.survey_id,
        survey_revision: currentSurvey.value.revision,
        answers: answers.value,
      },
    });
    if (!data?.success) throw new Error(data?.message || '提交失败');
    failedQuestionIds.value = data.failed_question_ids || [];
    const reward = data.reward as
      | { role_id: string; role_name?: string | null; permanent?: boolean; expires_at?: string | null }
      | undefined;
    if (!data.passed) {
      error.value = currentIsBlocking.value
        ? '尚未通过要求：请检查标红题目或答案后重新提交，本次无奖励。'
        : '问卷已记录，但未满足奖励要求，本次无奖励。可修改后重试或跳过。';
      await fetchHistory();
      return;
    }
    if (reward) {
      const roleName = reward.role_name || reward.role_id;
      const validity = reward.permanent ? '永久有效' : `有效期至 ${reward.expires_at ?? '未知'}`;
      message.value = `提交成功，已发放奖励：角色「${roleName}」（${validity}）`;
    } else {
      message.value = '提交成功';
    }
    await Promise.all([fetchPending(), fetchHistory()]);
    if (surveys.value.length === 0) {
      window.setTimeout(() => emit('close'), 500);
    }
  } catch (submitError: any) {
    error.value = submitError?.message || String(submitError);
  } finally {
    submitting.value = false;
  }
};

const dismiss = async () => {
  if (!currentSurvey.value || currentSurvey.value.required_for_access) return;
  submitting.value = true;
  error.value = '';
  try {
    const data = await VrcApi.request(endpoint('/api/client/surveys/dismiss'), {
      method: 'POST',
      params: {
        user_id: props.userId,
        survey_id: currentSurvey.value.survey_id,
        survey_revision: currentSurvey.value.revision,
      },
    });
    if (!data?.success) throw new Error(data?.message || '跳过失败');
    await Promise.all([fetchPending(), fetchHistory()]);
    if (surveys.value.length === 0) emit('close');
  } catch (dismissError: any) {
    error.value = dismissError?.message || String(dismissError);
  } finally {
    submitting.value = false;
  }
};

const deleteSubmission = async (submissionId: string) => {
  if (!confirm('删除记录后，如果它对应当前问卷，问卷可能会再次提示。继续吗？')) return;
  try {
    const data = await VrcApi.request(endpoint('/api/client/survey-history/delete'), {
      method: 'POST',
      params: { user_id: props.userId, submission_id: submissionId },
    });
    if (!data?.success) throw new Error(data?.message || '删除失败');
    await Promise.all([fetchPending(), fetchHistory()]);
  } catch (deleteError: any) {
    error.value = deleteError?.message || String(deleteError);
  }
};

const close = () => {
  if (props.forced && remainingRequired.value) return;
  emit('close');
};

onMounted(load);
</script>

<template>
  <div class="fixed inset-0 z-[10020] bg-background flex flex-col text-text">
    <header class="h-16 shrink-0 border-b border-border-soft bg-surface px-6 flex items-center justify-between gap-4">
      <div class="flex items-center gap-3 min-w-0">
        <div class="w-9 h-9 rounded-md bg-primary text-white grid place-items-center shrink-0">
          <ClipboardList :size="19" />
        </div>
        <div class="min-w-0">
          <h1 class="text-base font-black text-text-strong">问卷中心</h1>
          <p class="text-[11px] text-text-muted truncate">
            {{ remainingRequired ? '完成并通过必填问卷后方可继续使用' : '查看待填写问卷与历史记录' }}
          </p>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <button
          class="h-9 px-3 rounded-md text-xs font-bold flex items-center gap-2"
          :class="activeTab === 'pending' ? 'bg-primary text-white' : 'text-text-muted hover:bg-surface-hover'"
          @click="activeTab = 'pending'"
        >
          <Clock3 :size="15" /> 待填写 <span v-if="surveys.length">{{ surveys.length }}</span>
        </button>
        <button
          class="h-9 px-3 rounded-md text-xs font-bold flex items-center gap-2"
          :class="activeTab === 'history' ? 'bg-primary text-white' : 'text-text-muted hover:bg-surface-hover'"
          @click="activeTab = 'history'"
        >
          <History :size="15" /> 我的记录
        </button>
        <button
          v-if="!forced || !remainingRequired"
          class="w-9 h-9 rounded-md border border-border-soft grid place-items-center text-text-muted hover:text-text"
          title="关闭"
          @click="close"
        >
          <X :size="17" />
        </button>
      </div>
    </header>

    <div v-if="loading" class="flex-1 grid place-items-center">
      <Loader2 :size="28" class="animate-spin text-primary" />
    </div>

    <main v-else-if="activeTab === 'pending'" class="flex-1 min-h-0 overflow-y-auto">
      <div v-if="currentSurvey" class="max-w-3xl mx-auto px-6 py-8">
        <button v-if="surveys.length > 1" class="mb-4 text-xs text-text-muted flex items-center gap-1"><ChevronLeft :size="14" /> 当前第 1 份，共 {{ surveys.length }} 份</button>
        <div class="flex items-start justify-between gap-6 border-b border-border-soft pb-6">
          <div>
            <div class="flex items-center gap-2 mb-2">
              <span class="px-2 py-1 rounded text-[10px] font-bold" :class="currentSurvey.required_for_access ? 'bg-red-500/10 text-red-500' : 'bg-blue-500/10 text-blue-500'">
                {{ currentSurvey.required_for_access ? '使用门禁' : '可选问卷' }}
              </span>
              <span class="text-[10px] text-text-muted">版本 {{ currentSurvey.revision }}</span>
            </div>
            <h2 class="text-2xl font-black text-text-strong">{{ currentSurvey.title }}</h2>
            <p v-if="currentSurvey.description" class="mt-3 text-sm leading-6 text-text-muted whitespace-pre-wrap">{{ currentSurvey.description }}</p>
          </div>
        </div>

        <form class="py-6 space-y-6" @submit.prevent="submit">
          <section
            v-for="(question, index) in currentSurvey.questions"
            :key="question.question_id"
            class="pb-6 border-b border-border-soft last:border-0"
            :class="failedQuestionIds.includes(question.question_id) ? 'text-red-500' : ''"
          >
            <h3 class="text-sm font-bold text-text-strong" :class="failedQuestionIds.includes(question.question_id) ? '!text-red-500' : ''">
              <span class="text-text-muted mr-2">{{ index + 1 }}.</span>{{ question.title }}
              <span v-if="question.required" class="text-red-500 ml-1">*</span>
            </h3>
            <p v-if="question.description" class="mt-1.5 text-xs leading-5 text-text-muted">{{ question.description }}</p>

            <div v-for="(media, mediaIndex) in question.media" :key="mediaIndex" class="mt-4">
              <img v-if="media.media_type === 'image'" :src="media.url" :alt="media.caption || question.title" class="max-h-80 max-w-full rounded-md object-contain border border-border-soft bg-black/5">
              <video v-else :src="media.url" controls preload="metadata" class="max-h-96 w-full rounded-md bg-black" />
              <p v-if="media.caption" class="mt-1.5 text-[11px] text-text-muted">{{ media.caption }}</p>
            </div>

            <div v-if="question.question_type === 'single_choice'" class="mt-4 grid gap-2">
              <label v-for="option in question.options" :key="option.option_id" class="min-h-10 px-3 border border-border-soft rounded-md flex items-center gap-3 cursor-pointer hover:bg-surface-hover">
                <input v-model="answers[question.question_id]" type="radio" :name="question.question_id" :value="option.option_id" class="w-4 h-4 accent-primary">
                <span class="text-sm text-text">{{ option.label }}</span>
              </label>
            </div>

            <div v-else-if="question.question_type === 'multiple_choice'" class="mt-4 grid gap-2">
              <label v-for="option in question.options" :key="option.option_id" class="min-h-10 px-3 border border-border-soft rounded-md flex items-center gap-3 cursor-pointer hover:bg-surface-hover">
                <input
                  type="checkbox"
                  class="w-4 h-4 accent-primary"
                  :checked="isMultipleChecked(question.question_id, option.option_id)"
                  @change="toggleMultiple(question.question_id, option.option_id)"
                >
                <span class="text-sm text-text">{{ option.label }}</span>
              </label>
            </div>

            <input
              v-else-if="question.question_type === 'short_text'"
              :value="answers[question.question_id]"
              class="mt-4 w-full h-10 px-3 rounded-md bg-surface border border-border-soft text-sm text-text outline-none focus:border-primary"
              placeholder="请输入答案"
              @input="answers[question.question_id] = ($event.target as HTMLInputElement).value"
            >

            <textarea
              v-else
              :value="answers[question.question_id]"
              rows="5"
              class="mt-4 w-full px-3 py-2 rounded-md bg-surface border border-border-soft text-sm text-text outline-none resize-y focus:border-primary"
              placeholder="请输入你的意见"
              @input="answers[question.question_id] = ($event.target as HTMLTextAreaElement).value"
            />
          </section>

          <p v-if="error" class="p-3 rounded-md bg-red-500/10 text-red-500 text-xs">{{ error }}</p>
          <p v-if="message" class="p-3 rounded-md bg-green-500/10 text-green-500 text-xs flex items-center gap-2"><CheckCircle2 :size="15" /> {{ message }}</p>

          <div class="flex items-center justify-between gap-3 pt-2">
            <button
              v-if="!currentIsBlocking"
              type="button"
              class="h-10 px-4 rounded-md border border-border-soft text-xs font-bold text-text-muted hover:bg-surface-hover"
              :disabled="submitting"
              @click="dismiss"
            >
              跳过并不再提示
            </button>
            <span v-else class="text-[11px] text-text-muted">带 * 的题目为必答题</span>
            <button type="submit" class="h-10 px-6 rounded-md bg-primary hover:bg-primary-hover text-white text-sm font-bold flex items-center gap-2 disabled:opacity-60" :disabled="submitting">
              <Loader2 v-if="submitting" :size="16" class="animate-spin" />
              <Send v-else :size="16" />
              提交问卷
            </button>
          </div>
        </form>
      </div>

      <div v-else class="h-full grid place-items-center text-center px-6">
        <div>
          <CheckCircle2 :size="48" class="mx-auto text-green-500" />
          <h2 class="mt-4 text-lg font-black text-text-strong">暂无待填写问卷</h2>
          <p class="mt-2 text-sm text-text-muted">管理员发布新问卷后会在这里提示。</p>
          <button class="mt-5 h-9 px-4 rounded-md bg-primary text-white text-xs font-bold" @click="close">继续使用</button>
        </div>
      </div>
    </main>

    <main v-else class="flex-1 min-h-0 overflow-y-auto">
      <div class="max-w-4xl mx-auto px-6 py-8">
        <div class="mb-5">
          <h2 class="text-xl font-black text-text-strong">我的问卷记录</h2>
          <p class="mt-1 text-xs text-text-muted">删除通过记录后，对应的当前版本可能重新出现在待填写列表。</p>
        </div>
        <div v-if="history.length" class="border border-border-soft rounded-md overflow-hidden">
          <div v-for="submission in history" :key="submission.submission_id" class="min-h-16 px-4 py-3 border-b border-border-soft last:border-0 flex items-center gap-4">
            <CheckCircle2 v-if="submission.status === 'passed'" :size="19" class="text-green-500 shrink-0" />
            <XCircle v-else-if="submission.status === 'failed'" :size="19" class="text-red-500 shrink-0" />
            <Clock3 v-else :size="19" class="text-text-muted shrink-0" />
            <div class="flex-1 min-w-0">
              <div class="text-sm font-bold text-text-strong truncate">{{ submission.survey_title }}</div>
              <div class="mt-1 text-[11px] text-text-muted">版本 {{ submission.survey_revision }} · {{ submission.submitted_at }} · {{ submission.status === 'passed' ? '已通过' : submission.status === 'failed' ? '未通过' : '已跳过' }}</div>
            </div>
            <button class="w-8 h-8 rounded-md grid place-items-center text-text-muted hover:text-red-500 hover:bg-red-500/10" title="删除记录" @click="deleteSubmission(submission.submission_id)"><Trash2 :size="15" /></button>
          </div>
        </div>
        <div v-else class="py-20 text-center text-sm text-text-muted">暂无问卷记录</div>
        <p v-if="error" class="mt-4 p-3 rounded-md bg-red-500/10 text-red-500 text-xs">{{ error }}</p>
      </div>
    </main>
  </div>
</template>
