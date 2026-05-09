<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { VrcApi, DbApi, SysApi, GamelogApi } from "../api";
import { StickyNote, Save } from 'lucide-vue-next';
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
    console.warn('加载备忘录失败:', err);
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
    console.warn('保存失败:', err);
  }
};

const cancelEdit = () => {
  editingId.value = null;
  editText.value = '';
};

onMounted(() => fetchNotes());
</script>

<template>
  <div>
    <div class="flex items-center justify-between mb-6">
      <h2 class="text-2xl font-extrabold text-[#451a03] flex items-center gap-2">
        <StickyNote
          class="text-amber-500"
          :size="24"
        /> {{ t('notes.title') }}
      </h2>
      <span class="bg-amber-100 text-amber-800 px-3 py-1 rounded-full font-bold text-xs">
        {{ t('notes.count', { count: notes.length }) }}
      </span>
    </div>

    <div
      v-if="loading"
      class="text-center py-8 text-amber-500 font-bold animate-pulse"
    >
      {{ t('notes.loading') }}
    </div>

    <div
      v-else-if="notes.length === 0"
      class="bg-white/80 backdrop-blur rounded-2xl p-8 border-2 border-amber-100 text-center text-amber-600"
    >
      <StickyNote
        class="mx-auto mb-4 text-amber-300"
        :size="48"
      />
      <p class="font-bold">
        {{ t('notes.empty') }}
      </p>
      <p class="text-sm mt-1">
        {{ t('notes.empty_desc') }}
      </p>
    </div>

    <div
      v-else
      class="space-y-3"
    >
      <div
        v-for="note in notes"
        :key="note.user_id"
        class="bg-white/80 backdrop-blur rounded-2xl p-4 border-2 border-amber-50 hover:border-amber-200 transition-all"
      >
        <div class="flex items-center justify-between mb-2">
          <h3 class="font-bold text-amber-900">
            {{ note.display_name }}
          </h3>
          <span class="text-[10px] text-amber-500">{{ note.updated_at }}</span>
        </div>

        <div v-if="editingId === note.user_id">
          <textarea
            v-model="editText"
            class="w-full px-3 py-2 rounded-xl border-2 border-amber-200 focus:border-amber-400 focus:ring-0 outline-none bg-amber-50/50 text-sm resize-none"
            rows="3"
          />
          <div class="flex gap-2 mt-2 justify-end">
            <button
              class="text-xs text-amber-600 hover:text-amber-800 font-bold px-3 py-1 rounded-lg hover:bg-amber-50"
              @click="cancelEdit"
            >
              {{ t('notes.cancel') }}
            </button>
            <button
              class="text-xs bg-amber-500 text-white font-bold px-3 py-1.5 rounded-lg hover:bg-amber-600 flex items-center gap-1"
              @click="saveEdit(note)"
            >
              <Save :size="12" /> {{ t('notes.save') }}
            </button>
          </div>
        </div>
        <div
          v-else
          class="text-sm text-amber-800 cursor-pointer hover:bg-amber-50 rounded-lg p-2 -m-1 transition-colors"
          @click="startEdit(note)"
        >
          {{ note.note || t('notes.click_to_add') }}
        </div>
      </div>
    </div>
  </div>
</template>
