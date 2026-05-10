<script setup lang="ts">
import { useToast } from "../composables/useToast";

const toast = useToast();
import { ref, onMounted, computed } from "vue";
import { isTauri, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { SysApi } from "../api/index";
import StatusCard, { type ComponentStatus } from "./StatusCard.vue";
import InstallDialog from "./InstallDialog.vue";
import { Settings, RefreshCcw, Bone, X, Heart, AlertTriangle } from "lucide-vue-next";
import dogImg from '../assets/dog.jpg';
import unityImg from '../assets/unity.png';
import vrchatImg from '../assets/vrchat.png';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const hubStatus = ref<ComponentStatus>('checking');
const hubProgress = ref(0);
const hubProgressMsg = ref('');
const unityStatus = ref<ComponentStatus>('checking');
const unityProgress = ref(0);
const unityProgressMsg = ref('');
const toolStatus = ref<ComponentStatus>('checking');
const toolProgress = ref(0);
const toolProgressMsg = ref('');
const vccStatus = ref(false);
const alcomStatus = ref(false);
const showInstallDialog = ref(false);
const showSettings = ref(false);
const dialogConfig = ref({ title: '', target: '', isVccSelection: false });

const checkEnvironment = async () => {
  hubStatus.value = 'checking'; unityStatus.value = 'checking'; toolStatus.value = 'checking';
  try {
    if (!isTauri()) throw new Error(t('env.browser_mode_error'));
    const result = await SysApi.checkSystemStatus();
    hubStatus.value = result.hub_installed ? 'installed' : 'not_installed';
    unityStatus.value = result.unity_installed ? 'installed' : 'not_installed';
    toolStatus.value = result.tool_installed ? 'installed' : 'not_installed';
    vccStatus.value = result.vcc_installed;
    alcomStatus.value = result.alcom_installed;
  } catch {
    setTimeout(() => { hubStatus.value = 'not_installed'; unityStatus.value = 'not_installed'; toolStatus.value = 'not_installed'; }, 1000);
  }
};

const handleInstallClick = (target: string) => {
  if (target === 'hub') dialogConfig.value = { title: t('env.install_hub_title'), target: 'hub', isVccSelection: false };
  else if (target === 'unity') dialogConfig.value = { title: t('env.install_unity_title'), target: 'unity', isVccSelection: false };
  else if (target === 'tool') dialogConfig.value = { title: t('env.install_tool_title'), target: 'tool', isVccSelection: true };
  showInstallDialog.value = true;
};

const confirmUninstallTarget = ref('');
const showUninstallConfirm = ref(false);
const showUninstallToolSelection = ref(false);

const requestUninstall = (target: string) => {
  if (target === 'tool') {
    if (vccStatus.value && alcomStatus.value) {
      showUninstallToolSelection.value = true;
      return;
    }
    target = vccStatus.value ? 'vcc' : 'alcom';
  }
  confirmUninstallTarget.value = target;
  showUninstallConfirm.value = true;
};

const handleUninstallSelection = (target: string) => {
  showUninstallToolSelection.value = false;
  confirmUninstallTarget.value = target;
  showUninstallConfirm.value = true;
};

const executeUninstall = async () => {
  showUninstallConfirm.value = false;
  const target = confirmUninstallTarget.value;
  if (!target) return;

  if (target === 'hub') hubStatus.value = 'checking';
  if (target === 'unity') unityStatus.value = 'checking';
  if (target === 'tool' || target === 'vcc' || target === 'alcom') toolStatus.value = 'checking';
  
  try {
    if (!isTauri()) throw new Error(t('env.uninstall_browser_error'));
    await SysApi.uninstallSoftware({ target });
    // Add a timeout because uninstallers run asynchronously
    setTimeout(async () => {
      await checkEnvironment();
    }, 3000);
  } catch (err: any) { 
    toast.error(err.message || err); 
    await checkEnvironment(); 
  }
};

const showLaunchToolConfirm = ref(false);

const handleLaunch = async (target: string) => {
  if (target === 'tool') {
    if (vccStatus.value && alcomStatus.value) {
      showLaunchToolConfirm.value = true;
      return;
    }
    target = vccStatus.value ? 'vcc' : 'alcom';
  }
  await executeLaunch(target);
};

const executeLaunch = async (target: string) => {
  showLaunchToolConfirm.value = false;
  try {
    if (!isTauri()) throw new Error(t('env.launch_browser_error'));
    await SysApi.launchSoftware({ target });
  } catch (err: any) {
    toast.error(err.message || err);
  }
};

const handleDialogConfirm = async (config: any) => {
  showInstallDialog.value = false;
  const target = dialogConfig.value.target;
  if (target === 'hub') hubStatus.value = 'installing';
  if (target === 'unity') unityStatus.value = 'installing';
  if (target === 'tool') toolStatus.value = 'installing';
  try {
    if (!('__TAURI_INTERNALS__' in window)) {
      setTimeout(() => {
        if (target === 'hub') { hubStatus.value = 'installed'; }
        if (target === 'unity') { unityStatus.value = 'installed'; }
        if (target === 'tool') { toolStatus.value = 'installed'; }
      }, 2000);
      return;
    }
    await SysApi.installSoftware({ target, path: config.path, tool: config.tool, autoDelete: config.autoDelete });
    await checkEnvironment();
  } catch (error: any) {
    toast.error(error.message || error);
    await checkEnvironment();
  }
};

onMounted(() => {
  checkEnvironment();
  
  // Register progress listeners
  listen('install-progress', (event: any) => {
    const payload = event.payload;
    if (payload.target === 'hub') {
      hubProgress.value = payload.progress;
      hubProgressMsg.value = payload.message;
    } else if (payload.target === 'unity') {
      unityProgress.value = payload.progress;
      unityProgressMsg.value = payload.message;
    } else if (payload.target === 'tool') {
      toolProgress.value = payload.progress;
      toolProgressMsg.value = payload.message;
    }
  });
});

// VPM Listing feature
import { useStorage } from '@vueuse/core';
import { Copy, HelpCircle, Search, Info, Plus } from "lucide-vue-next";
const vpmRepoUrl = useStorage('ovo_vpm_repo_url', 'https://vpm.pimaker.at/index.json');
const vpmPackages = ref<any[]>([]);
const vpmSearch = ref('');
const isVpmLoading = ref(false);

const loadVpmRepo = async () => {
  if (!vpmRepoUrl.value) return;
  isVpmLoading.value = true;
  try {
    const res = await fetch(vpmRepoUrl.value);
    const data = await res.json();
    if (data && data.packages) {
      vpmPackages.value = Object.values(data.packages).map((pkgGroup: any) => {
        // Packages are grouped by version, get the latest version (first key or highest semver)
        const versions = Object.keys(pkgGroup.versions);
        const latestVer = versions.sort().reverse()[0];
        return pkgGroup.versions[latestVer];
      });
    } else {
      vpmPackages.value = [];
    }
  } catch (err: any) {
    console.error("Failed to load VPM Repo:", err);
    toast.error(t('env.parse_err') + err.message);
  } finally {
    isVpmLoading.value = false;
  }
};

const filteredVpmPackages = computed(() => {
  if (!vpmSearch.value) return vpmPackages.value;
  const q = vpmSearch.value.toLowerCase();
  return vpmPackages.value.filter(p => p.name.toLowerCase().includes(q) || (p.displayName && p.displayName.toLowerCase().includes(q)));
});

const scanLocalDeps = async () => {
  if (!isTauri()) return;
  isVpmLoading.value = true;
  try {
    const localDeps = await invoke<any[]>("scan_local_project_dependencies");
    if (Array.isArray(localDeps)) {
      vpmPackages.value = localDeps.map(d => ({
        name: d.name,
        displayName: d.name,
        version: d.version,
        description: t('env.local_pkg_desc'),
        author: { name: t('env.local_user') },
        isLocal: true,
      }));
    }
  } catch (err: any) {
    console.error("Failed to scan local dependencies:", err);
    toast.error(t('env.scan_err') + (err.message || err));
  } finally {
    isVpmLoading.value = false;
  }
};

const addToVcc = (url: string) => {
  window.location.assign(`vcc://vpm/addRepo?url=${encodeURIComponent(url)}`);
};

const addToAlcom = (url: string) => {
  window.location.assign(`alcom://vpm/addRepo?url=${encodeURIComponent(url)}`);
};

const copyVpmUrl = () => {
  navigator.clipboard.writeText(vpmRepoUrl.value);
  toast.info(t('gallery.copied_alert'));
};

onMounted(() => {
  scanLocalDeps();
});
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-slate-50/50 rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-indigo-500/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <header class="flex items-center justify-between mb-8 shrink-0 z-10">
      <div class="flex items-center gap-4">
        <div class="w-14 h-14 rounded-2xl overflow-hidden border border-slate-200 shadow-lg shadow-indigo-200/50 flex-shrink-0 bg-white">
          <img
            :src="dogImg"
            class="w-full h-full object-cover"
            alt="VrcDog Logo"
          >
        </div>
        <div>
          <h1 class="text-3xl font-extrabold text-slate-900 tracking-tight mb-1 flex items-center gap-2">
            {{ t('env.title') }} <Bone class="w-6 h-6 text-indigo-500 inline-block animate-bounce" />
          </h1>
          <p class="text-slate-500 font-bold text-sm">
            {{ t('env.subtitle') }}
          </p>
        </div>
      </div>
      <div class="flex gap-2">
        <button
          class="flex items-center gap-2 px-5 py-2.5 rounded-xl bg-white hover:bg-slate-50 text-slate-700 hover:text-indigo-600 shadow-sm border border-slate-200 transition-all active:scale-95 text-sm font-bold"
          @click="checkEnvironment"
        >
          <RefreshCcw
            class="w-4 h-4"
            :class="{'animate-spin': hubStatus === 'checking'}"
          />
          {{ t('env.check') }}
        </button>
        <button
          class="p-2.5 rounded-xl bg-white hover:bg-slate-50 text-slate-600 hover:text-indigo-600 shadow-sm border border-slate-200 transition-all active:scale-95"
          @click="showSettings = true"
        >
          <Settings class="w-5 h-5" />
        </button>
      </div>
    </header>

    <div class="grid grid-cols-1 md:grid-cols-3 gap-6 flex-1 overflow-y-auto pr-2 custom-scrollbar items-stretch content-start pb-6 z-10 relative">
      <!-- Status Cards -->
      <StatusCard
        title="Unity Hub"
        :description="t('env.hub_desc')"
        :status="hubStatus"
        :progress="hubProgress"
        :progress-message="hubProgressMsg"
        :icon-src="unityImg"
        :install-label="t('env.hub_install')"
        @install="handleInstallClick('hub')"
        @uninstall="requestUninstall('hub')"
        @launch="handleLaunch('hub')"
      />
      <StatusCard
        title="Unity 2022.3.22f1"
        :description="t('env.unity_desc')"
        :status="unityStatus"
        :progress="unityProgress"
        :progress-message="unityProgressMsg"
        :icon-src="unityImg"
        :install-label="t('env.unity_install')"
        @install="handleInstallClick('unity')"
        @uninstall="requestUninstall('unity')"
        @launch="handleLaunch('unity')"
      />
      <StatusCard
        title="Creator Tools"
        :description="t('env.tool_desc')"
        :status="toolStatus"
        :progress="toolProgress"
        :progress-message="toolProgressMsg"
        :icon-src="vrchatImg"
        :install-label="t('env.tool_install')"
        @install="handleInstallClick('tool')"
        @uninstall="requestUninstall('tool')"
        @launch="handleLaunch('tool')"
      />

      <!-- VCC Dependency Management Section -->
      <div class="col-span-1 md:col-span-3 mt-2 bg-white/70 backdrop-blur-xl p-6 rounded-3xl border border-white shadow-lg shadow-slate-200/40">
        <h2 class="text-xl font-extrabold text-slate-900 mb-6 flex items-center gap-3">
          <span class="p-1.5 bg-indigo-50 rounded-lg text-indigo-600">
            <Bone :size="20" />
          </span>
          {{ t('env.vcc_lib_title') }} <span class="text-[10px] uppercase font-black tracking-wider bg-indigo-100 text-indigo-700 px-2 py-0.5 rounded-full">{{ t('env.vcc_lib_beta') }}</span>
        </h2>
        
        <!-- Repo Header -->
        <div class="flex flex-col xl:flex-row items-center gap-3 mb-6">
          <div class="relative w-full flex-1">
            <input 
              v-model="vpmRepoUrl" 
              class="w-full bg-white border border-slate-200 rounded-xl pl-4 pr-10 py-2.5 text-sm font-bold text-slate-800 focus:outline-none focus:border-indigo-400 focus:ring-4 focus:ring-indigo-500/10 transition-all shadow-sm"
              placeholder="https://vpm.domain.com/index.json"
            >
            <button 
              class="absolute right-2 top-1/2 -translate-y-1/2 p-1.5 text-slate-400 hover:text-indigo-600 hover:bg-indigo-50 rounded-lg transition-colors"
              :title="t('env.copy_url')"
              @click="copyVpmUrl"
            >
              <Copy class="w-4 h-4" />
            </button>
          </div>
          
          <div class="flex w-full xl:w-auto gap-3">
            <button 
              class="flex-1 xl:flex-none px-4 py-2.5 bg-white border border-slate-200 hover:border-indigo-300 text-slate-700 hover:text-indigo-700 rounded-xl font-bold shadow-sm transition-all flex items-center justify-center gap-2 active:scale-95 text-sm"
              @click="scanLocalDeps"
            >
              <Search
                :class="{'animate-pulse': isVpmLoading}"
                class="w-4 h-4"
              /> {{ t('env.scan_local') }}
            </button>
            <button 
              class="flex-1 xl:flex-none px-4 py-2.5 bg-indigo-600 hover:bg-indigo-700 text-white rounded-xl font-bold shadow-sm shadow-indigo-500/30 transition-all flex items-center justify-center gap-2 active:scale-95 text-sm"
              @click="loadVpmRepo"
            >
              <RefreshCcw
                :class="{'animate-spin': isVpmLoading}"
                class="w-4 h-4"
              /> {{ t('env.load_net_repo') }}
            </button>
          </div>
        </div>

        <!-- Search Bar -->
        <div class="relative mb-4">
          <Search class="absolute left-4 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
          <input 
            v-model="vpmSearch"
            class="w-full bg-white border border-slate-200 rounded-xl pl-11 pr-4 py-2.5 text-sm font-bold text-slate-800 focus:outline-none focus:border-indigo-400 focus:ring-4 focus:ring-indigo-500/10 transition-all shadow-sm placeholder:text-slate-400 placeholder:font-medium"
            :placeholder="t('env.search_pkg')"
          >
        </div>

        <!-- Packages Table -->
        <div class="bg-white rounded-2xl border border-slate-200 overflow-hidden shadow-sm">
          <table class="w-full text-left text-sm text-slate-800">
            <thead class="bg-slate-50 border-b border-slate-200 font-extrabold text-slate-600">
              <tr>
                <th class="px-5 py-3.5">
                  {{ t('env.pkg_name') }}
                </th>
                <th class="px-5 py-3.5">
                  {{ t('env.pkg_version') }}
                </th>
                <th class="px-5 py-3.5">
                  {{ t('env.pkg_author') }}
                </th>
                <th class="px-5 py-3.5 text-right">
                  {{ t('env.pkg_action') }}
                </th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-100">
              <tr
                v-for="pkg in filteredVpmPackages"
                :key="pkg.name"
                class="hover:bg-indigo-50/40 transition-colors"
              >
                <td class="px-5 py-3.5">
                  <div class="font-extrabold text-slate-900">
                    {{ pkg.displayName || pkg.name }}
                  </div>
                  <div class="text-xs text-slate-500 font-medium truncate max-w-[250px] mt-0.5">
                    {{ pkg.description || pkg.name }}
                  </div>
                </td>
                <td class="px-5 py-3.5 font-mono text-xs font-bold text-slate-600">
                  {{ pkg.version }}
                </td>
                <td class="px-5 py-3.5 text-xs font-bold text-slate-600">
                  {{ pkg.author?.name || 'Unknown' }}
                </td>
                <td class="px-5 py-3.5 text-right space-x-2">
                  <template v-if="!pkg.isLocal">
                    <button 
                      class="text-[11px] font-extrabold text-slate-600 bg-white border border-slate-200 hover:border-indigo-300 hover:text-indigo-600 px-3 py-1.5 rounded-lg shadow-sm transition-colors"
                      @click="addToVcc(vpmRepoUrl)"
                    >
                      VCC Add
                    </button>
                    <button 
                      class="text-[11px] font-extrabold text-white bg-indigo-600 hover:bg-indigo-700 px-3 py-1.5 rounded-lg shadow-sm shadow-indigo-500/20 transition-colors"
                      @click="addToAlcom(vpmRepoUrl)"
                    >
                      ALCOM Add
                    </button>
                  </template>
                  <template v-else>
                    <span class="text-[11px] font-extrabold text-emerald-600 bg-emerald-50 px-2.5 py-1.5 rounded-lg border border-emerald-200/50 inline-block">
                      {{ t('env.installed') }}
                    </span>
                  </template>
                </td>
              </tr>
              <tr v-if="filteredVpmPackages.length === 0 && !isVpmLoading">
                <td
                  colspan="4"
                  class="px-5 py-12 text-center text-slate-400 font-bold"
                >
                  {{ t('env.no_pkg_found') }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <InstallDialog
      :show="showInstallDialog"
      :title="dialogConfig.title"
      :is-vcc-selection="dialogConfig.isVccSelection"
      :vcc-installed="vccStatus"
      :alcom-installed="alcomStatus"
      @close="showInstallDialog = false"
      @confirm="handleDialogConfirm"
      @uninstall="requestUninstall"
    />

    <!-- Uninstall Confirm Modal -->
    <Teleport to="body">
      <Transition name="fade">
        <div
          v-if="showUninstallConfirm"
          class="fixed inset-0 z-[60] flex items-center justify-center p-4"
        >
          <div
            class="absolute inset-0 bg-slate-900/40 backdrop-blur-sm"
            @click="showUninstallConfirm = false"
          />
          <div class="bg-white/90 backdrop-blur-xl w-full max-w-sm rounded-3xl shadow-2xl relative z-10 p-6 border border-white">
            <div class="flex justify-between items-center mb-4">
              <h3 class="text-xl font-extrabold text-red-600 flex items-center gap-2">
                <AlertTriangle class="w-6 h-6" /> 确认卸载?
              </h3>
              <button
                class="p-1.5 rounded-xl hover:bg-slate-100 text-slate-400 hover:text-slate-600 transition-colors"
                @click="showUninstallConfirm = false"
              >
                <X class="w-5 h-5" />
              </button>
            </div>
            <p class="text-slate-600 text-sm mb-6 font-bold leading-relaxed">
              {{ t('env.uninstall_desc_1') }} <span class="text-slate-900">{{ confirmUninstallTarget === 'hub' ? 'Unity Hub' : (confirmUninstallTarget === 'unity' ? 'Unity 2022.3.22f1' : (confirmUninstallTarget === 'vcc' ? 'VCC' : (confirmUninstallTarget === 'alcom' ? 'ALCOM' : 'Creator Tools'))) }}</span> {{ t('env.uninstall_desc_2') }}
            </p>
            <div class="flex gap-3">
              <button
                class="flex-1 py-2.5 rounded-xl bg-slate-100 hover:bg-slate-200 text-slate-700 font-bold transition-colors"
                @click="showUninstallConfirm = false"
              >
                取消
              </button>
              <button
                class="flex-1 py-2.5 rounded-xl bg-red-500 hover:bg-red-600 text-white font-bold shadow-lg shadow-red-500/30 transition-colors"
                @click="executeUninstall"
              >
                确认卸载
              </button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- Tool Launch Confirm Modal -->
    <Teleport to="body">
      <Transition name="fade">
        <div
          v-if="showLaunchToolConfirm"
          class="fixed inset-0 z-[60] flex items-center justify-center p-4"
        >
          <div
            class="absolute inset-0 bg-slate-900/40 backdrop-blur-sm"
            @click="showLaunchToolConfirm = false"
          />
          <div class="bg-white/90 backdrop-blur-xl w-full max-w-sm rounded-3xl shadow-2xl relative z-10 p-6 border border-white">
            <div class="flex justify-between items-center mb-4">
              <h3 class="text-xl font-extrabold text-indigo-600 flex items-center gap-2">
                <AlertTriangle class="w-6 h-6" /> 选择启动工具
              </h3>
              <button
                class="p-1.5 rounded-xl hover:bg-slate-100 text-slate-400 hover:text-slate-600 transition-colors"
                @click="showLaunchToolConfirm = false"
              >
                <X class="w-5 h-5" />
              </button>
            </div>
            <p class="text-slate-600 text-sm mb-6 font-bold leading-relaxed">
              {{ t('env.detect_multiple_tools') }} <span class="text-slate-900">VCC</span> {{ t('env.and') }} <span class="text-slate-900">ALCOM</span>{{ t('env.please_select_launch') }}
            </p>
            <div class="flex gap-3">
              <button
                class="flex-1 py-2.5 rounded-xl bg-indigo-500 hover:bg-indigo-600 text-white font-bold shadow-lg shadow-indigo-500/30 transition-colors"
                @click="executeLaunch('vcc')"
              >
                启动 VCC
              </button>
              <button
                class="flex-1 py-2.5 rounded-xl bg-slate-800 hover:bg-slate-900 text-white font-bold shadow-lg shadow-slate-900/30 transition-colors"
                @click="executeLaunch('alcom')"
              >
                启动 ALCOM
              </button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- Tool Uninstall Selection Modal -->
    <Teleport to="body">
      <Transition name="fade">
        <div
          v-if="showUninstallToolSelection"
          class="fixed inset-0 z-[60] flex items-center justify-center p-4"
        >
          <div
            class="absolute inset-0 bg-slate-900/40 backdrop-blur-sm"
            @click="showUninstallToolSelection = false"
          />
          <div class="bg-white/90 backdrop-blur-xl w-full max-w-sm rounded-3xl shadow-2xl relative z-10 p-6 border border-white">
            <div class="flex justify-between items-center mb-4">
              <h3 class="text-xl font-extrabold text-red-600 flex items-center gap-2">
                <AlertTriangle class="w-6 h-6" /> 选择卸载工具
              </h3>
              <button
                class="p-1.5 rounded-xl hover:bg-slate-100 text-slate-400 hover:text-slate-600 transition-colors"
                @click="showUninstallToolSelection = false"
              >
                <X class="w-5 h-5" />
              </button>
            </div>
            <p class="text-slate-600 text-sm mb-6 font-bold leading-relaxed">
              {{ t('env.detect_multiple_tools') }} <span class="text-slate-900">VCC</span> {{ t('env.and') }} <span class="text-slate-900">ALCOM</span>{{ t('env.please_select_uninstall') }}
            </p>
            <div class="flex gap-3">
              <button
                class="flex-1 py-2.5 rounded-xl bg-red-500 hover:bg-red-600 text-white font-bold shadow-lg shadow-red-500/30 transition-colors"
                @click="handleUninstallSelection('vcc')"
              >
                卸载 VCC
              </button>
              <button
                class="flex-1 py-2.5 rounded-xl bg-orange-500 hover:bg-orange-600 text-white font-bold shadow-lg shadow-orange-500/30 transition-colors"
                @click="handleUninstallSelection('alcom')"
              >
                卸载 ALCOM
              </button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- Settings Modal -->
    <Teleport to="body">
      <Transition name="fade">
        <div
          v-if="showSettings"
          class="fixed inset-0 z-[70] flex items-center justify-center p-4"
        >
          <div
            class="absolute inset-0 bg-slate-900/40 backdrop-blur-sm"
            @click="showSettings = false"
          />
          <div class="bg-white/95 backdrop-blur-xl w-full max-w-sm rounded-3xl shadow-2xl relative z-10 p-8 border border-white">
            <div class="flex justify-between items-center mb-6">
              <h2 class="text-2xl font-extrabold text-slate-900 flex items-center gap-2">
                {{ t('env.settings') }} <Settings class="w-5 h-5 text-indigo-500" />
              </h2>
              <button
                class="p-2 rounded-xl hover:bg-slate-100 text-slate-400 hover:text-slate-600 transition-colors"
                @click="showSettings = false"
              >
                <X class="w-6 h-6" />
              </button>
            </div>
            <div class="space-y-4 text-center">
              <div class="w-24 h-24 mx-auto rounded-full overflow-hidden border-4 border-indigo-100 shadow-md mb-4 bg-white">
                <img
                  :src="dogImg"
                  class="w-full h-full object-cover"
                >
              </div>
              <h3 class="text-xl font-black text-slate-900">
                VrcDog {{ t('env.dog_manager') }}
              </h3>
              <p class="text-slate-500 font-bold text-sm">
                v1.1.0 {{ t('env.dog_social') }}
              </p>
              <div class="pt-6 border-t border-slate-100">
                <p class="text-sm font-bold text-slate-400 flex items-center justify-center gap-1.5">
                  Made with <Heart class="w-4 h-4 text-pink-500 fill-pink-500 animate-pulse" /> for VRChat
                </p>
              </div>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.fade-enter-active, .fade-leave-active { transition: opacity 0.3s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
.fade-enter-active > div:nth-child(2) { transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1); }
.fade-enter-from > div:nth-child(2) { opacity: 0; transform: translateY(30px) scale(0.9); }
.fade-leave-active > div:nth-child(2) { transition: all 0.2s ease-in; }
.fade-leave-to > div:nth-child(2) { opacity: 0; transform: scale(0.95); }
</style>
