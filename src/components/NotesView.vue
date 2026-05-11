<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { VrcApi, DbApi, SysApi, GamelogApi } from "../api";
import { StickyNote, Save, Loader2, Edit3, X } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

interface FriendNote {
  user_id: string;
  display_name: string;
  note: string;
  updated_at: string;
}

const notes = ref<FriendNote[]>([]);
const loading = ref(true);
const editingId = ref<string | null>(null);
const editText = ref('');

const fetchNotes = async () => {
  loading.value = true;
  try {
    notes.value = await DbApi.getNotes();
  } catch (err) {
    console.warn(t('auto_857820f3'), err);
  } finally {
    loading.value = false;
  }
};

const startEdit = (note: FriendNote) => {
  editingId.value = note.user_id;
  editText.value = note.note;
};

const saveEdit = async (note: FriendNote) => {
  try {
    await DbApi.saveNote({
      userId: note.user_id,
      displayName: note.display_name,
      note: editText.value,
    });
    note.note = editText.value;
    editingId.value = null;
  } catch (err) {
    console.warn(t('auto_4d57314a'), err);
  }
};

const cancelEdit = () => {
  editingId.value = null;
  editText.value = '';
};

onMounted(() => fetchNotes());
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-surface-hover rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-primary/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <div class="flex items-center justify-between mb-8 shrink-0 z-10">
      <h2 class="text-3xl font-extrabold text-text tracking-tight flex items-center gap-3">
        <span class="inline-flex items-center justify-center p-2 bg-primary/10 rounded-2xl shadow-sm border-primary">
          <StickyNote class="w-6 h-6 text-primary" />
        </span>
        {{ t('notes.title') }}
      </h2>
      <span class="bg-surface border-border-soft text-text-muted px-4 py-2 rounded-xl font-bold text-sm shadow-sm flex items-center gap-2">
        {{ t('notes.count', { count: notes.length }) }}
      </span>
    </div>

    <div class="flex-1 overflow-y-auto pr-2 custom-scrollbar z-10 relative">
      <div
        v-if="loading"
        class="absolute inset-0 flex flex-col items-center justify-center text-primary bg-surface-hover backdrop-blur-sm z-10"
      >
        <Loader2
          class="animate-spin mb-4"
          :size="48"
        />
        <span class="font-extrabold text-lg tracking-wide">{{ t('notes.loading') }}</span>
      </div>

      <div
        v-else-if="notes.length === 0"
        class="h-full flex flex-col items-center justify-center text-border-strong"
      >
        <StickyNote
          class="mb-4 opacity-30"
          :size="64"
        />
        <p class="font-bold text-xl text-text-muted">
          {{ t('notes.empty') }}
        </p>
        <p class="text-sm mt-2 font-medium">
          {{ t('notes.empty_desc') }}
        </p>
      </div>

      <div
        v-else
        class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-5 pb-10"
      >
        <div
          v-for="note in notes"
          :key="note.user_id"
          class="bg-surface backdrop-blur-xl rounded-2xl p-5 border-border-soft shadow-sm hover:shadow-md hover:border-primary transition-all flex flex-col group relative"
        >
          <div class="flex items-start justify-between mb-3 border-border-soft pb-3">
            <h3 class="font-bold text-text text-base truncate pr-2">
              {{ note.display_name }}
            </h3>
            <span class="text-[10px] text-border-strong font-mono tracking-tighter whitespace-nowrap pt-1 flex-shrink-0">{{ note.updated_at }}</span>
          </div>

          <div
            v-if="editingId === note.user_id"
            class="flex-1 flex flex-col"
          >
            <textarea
              v-model="editText"
              class="w-full flex-1 min-h-[100px] px-3 py-2 rounded-xl border-primary  focus:ring-4 focus:ring-indigo-500/10 outline-none bg-primary/10 text-sm resize-none transition-all custom-scrollbar"
              :placeholder="t('global.auto_b351a1e4')"
            />
            <div class="flex gap-2 mt-3 justify-end">
              <button
                class="text-xs text-text-muted hover:text-text-muted font-bold px-3 py-2 rounded-lg hover:bg-surface transition-colors flex items-center gap-1"
                @click="cancelEdit"
              >
                <X :size="14" /> {{ t('notes.cancel') }}
              </button>
              <button
                class="text-xs bg-primary/10 text-white font-bold px-4 py-2 rounded-lg hover:bg-primary/10 shadow-sm shadow-indigo-500/30 transition-colors flex items-center gap-1.5"
                @click="saveEdit(note)"
              >
                <Save :size="14" /> {{ t('notes.save') }}
              </button>
            </div>
          </div>
          <div
            v-else
            class="flex-1 text-sm text-text-muted leading-relaxed cursor-pointer hover:bg-surface-hover rounded-xl p-3 -mx-3 transition-colors relative"
            @click="startEdit(note)"
          >
            <div class="whitespace-pre-wrap line-clamp-5">
              {{ note.note || t('notes.click_to_add') }}
            </div>
            <div class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity bg-surface p-1.5 rounded-lg shadow-sm border-border-soft text-primary">
              <Edit3 :size="14" />
            </div>
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
