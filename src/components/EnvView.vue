<script setup lang="ts">
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
    if (!isTauri()) throw new Error("浏览器模式");
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
    if (!isTauri()) throw new Error("普通浏览器不能执行卸载");
    await SysApi.uninstallSoftware({ target });
    // Add a timeout because uninstallers run asynchronously
    setTimeout(async () => {
      await checkEnvironment();
    }, 3000);
  } catch (err: any) { 
    alert(err.message || err); 
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
    if (!isTauri()) throw new Error("浏览器模式无法启动本地软件");
    await SysApi.launchSoftware({ target });
  } catch (err: any) {
    alert(err.message || err);
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
    alert(error.message || error);
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
    alert(t('env.parse_err') + err.message);
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
    alert(t('env.scan_err') + (err.message || err));
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
  alert(t('gallery.copied_alert'));
};

onMounted(() => {
  scanLocalDeps();
});
</script>

<template>
  <div class="h-full flex flex-col">
    <header class="flex items-center justify-between mb-8">
      <div class="flex items-center gap-4">
        <div class="w-14 h-14 rounded-full overflow-hidden border-4 border-white shadow-lg shadow-amber-200/50 flex-shrink-0 bg-white">
          <img
            :src="dogImg"
            class="w-full h-full object-cover"
            alt="VrcDog Logo"
          >
        </div>
        <div>
          <h1 class="text-2xl font-extrabold text-[#451a03] tracking-tight mb-0.5 flex items-center gap-2">
            {{ t('env.title') }} <Bone class="w-5 h-5 text-amber-500 inline-block animate-bounce" />
          </h1>
          <p class="text-amber-700 font-medium text-sm">
            {{ t('env.subtitle') }}
          </p>
        </div>
      </div>
      <div class="flex gap-2">
        <button
          class="flex items-center gap-2 px-4 py-2 rounded-full bg-white hover:bg-amber-50 text-amber-700 shadow-sm border border-amber-100 transition-colors text-sm font-bold"
          @click="checkEnvironment"
        >
          <RefreshCcw
            class="w-4 h-4"
            :class="{'animate-spin': hubStatus === 'checking'}"
          />
          {{ t('env.check') }}
        </button>
        <button
          class="p-2 rounded-full bg-white hover:bg-amber-50 text-amber-700 shadow-sm border border-amber-100 transition-colors"
          @click="showSettings = true"
        >
          <Settings class="w-4 h-4" />
        </button>
      </div>
    </header>

    <div class="grid grid-cols-1 md:grid-cols-3 gap-6 flex-1 overflow-y-auto pr-1 items-stretch content-start pb-6">
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
      <div class="col-span-1 md:col-span-3 mt-4 glass-panel p-6 rounded-3xl shadow-sm">
        <h2 class="text-xl font-extrabold text-[#451a03] mb-4 flex items-center gap-2">
          {{ t('env.vcc_lib_title') }} <span class="text-xs bg-amber-100 text-amber-700 px-2 py-0.5 rounded-full">{{ t('env.vcc_lib_beta') }}</span>
        </h2>
        
        <!-- Repo Header -->
        <div class="flex items-center gap-2 mb-4">
          <input 
            v-model="vpmRepoUrl" 
            class="flex-1 bg-white border-2 border-amber-100 rounded-xl px-4 py-2 text-sm text-amber-900 focus:outline-none focus:border-amber-400 font-mono shadow-inner"
            placeholder="https://vpm.domain.com/index.json"
          >
          <button 
            class="px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-xl font-bold shadow-md shadow-blue-500/20 transition-colors flex items-center gap-2"
            @click="scanLocalDeps"
          >
            <Search :class="{'animate-pulse': isVpmLoading}" class="w-4 h-4" /> {{ t('env.scan_local') }}
          </button>
          <button 
            class="px-4 py-2 bg-emerald-500 hover:bg-emerald-600 text-white rounded-xl font-bold shadow-md shadow-emerald-500/20 transition-colors flex items-center gap-2"
            @click="loadVpmRepo"
          >
            <RefreshCcw :class="{'animate-spin': isVpmLoading}" class="w-4 h-4" /> {{ t('env.load_net_repo') }}
          </button>
          <button 
            class="p-2 bg-white hover:bg-amber-50 text-amber-600 border border-amber-200 rounded-xl transition-colors"
            @click="copyVpmUrl"
            :title="t('env.copy_url')"
          >
            <Copy class="w-4 h-4" />
          </button>
        </div>

        <!-- Search Bar -->
        <div class="relative mb-4">
          <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-amber-400" />
          <input 
            v-model="vpmSearch"
            class="w-full bg-white/60 border-2 border-amber-50 rounded-xl pl-10 pr-4 py-2 text-sm text-amber-900 focus:outline-none focus:border-amber-300 transition-colors"
            :placeholder="t('env.search_pkg')"
          >
        </div>

        <!-- Packages Table -->
        <div class="bg-white/80 rounded-2xl border border-amber-100 overflow-hidden">
          <table class="w-full text-left text-sm text-amber-900">
            <thead class="bg-amber-50 border-b border-amber-100 font-bold">
              <tr>
                <th class="px-4 py-3">{{ t('env.pkg_name') }}</th>
                <th class="px-4 py-3">{{ t('env.pkg_version') }}</th>
                <th class="px-4 py-3">{{ t('env.pkg_author') }}</th>
                <th class="px-4 py-3 text-right">{{ t('env.pkg_action') }}</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-amber-50">
              <tr v-for="pkg in filteredVpmPackages" :key="pkg.name" class="hover:bg-amber-50/50 transition-colors">
                <td class="px-4 py-3">
                  <div class="font-bold">{{ pkg.displayName || pkg.name }}</div>
                  <div class="text-xs text-amber-600/70 truncate max-w-[200px]">{{ pkg.description || pkg.name }}</div>
                </td>
                <td class="px-4 py-3 font-mono text-xs">{{ pkg.version }}</td>
                <td class="px-4 py-3 text-xs">{{ pkg.author?.name || 'Unknown' }}</td>
                <td class="px-4 py-3 text-right space-x-2">
                  <template v-if="!pkg.isLocal">
                    <button 
                      class="text-xs font-bold text-white bg-[#f59e0b] hover:bg-[#d97706] px-3 py-1.5 rounded-lg shadow-sm transition-colors"
                      @click="addToVcc(vpmRepoUrl)"
                    >
                      VCC Add
                    </button>
                    <button 
                      class="text-xs font-bold text-white bg-indigo-500 hover:bg-indigo-600 px-3 py-1.5 rounded-lg shadow-sm transition-colors"
                      @click="addToAlcom(vpmRepoUrl)"
                    >
                      ALCOM Add
                    </button>
                  </template>
                  <template v-else>
                    <span class="text-xs font-bold text-emerald-600 bg-emerald-50 px-2 py-1 rounded border border-emerald-200">
                      {{ t('env.installed') }}
                    </span>
                  </template>
                </td>
              </tr>
              <tr v-if="filteredVpmPackages.length === 0 && !isVpmLoading">
                <td colspan="4" class="px-4 py-8 text-center text-amber-600/60 font-bold">
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
            class="absolute inset-0 bg-black/40 backdrop-blur-sm"
            @click="showUninstallConfirm = false"
          />
          <div class="bg-white/95 backdrop-blur-xl w-full max-w-sm rounded-3xl shadow-2xl relative z-10 p-6 border-2 border-white">
            <div class="flex justify-between items-center mb-4">
              <h3 class="text-xl font-extrabold text-red-600 flex items-center gap-2">
                <AlertTriangle class="w-6 h-6" /> 确认卸载?
              </h3>
              <button
                class="p-1 rounded-full hover:bg-black/5 text-gray-500 transition-colors"
                @click="showUninstallConfirm = false"
              >
                <X class="w-5 h-5" />
              </button>
            </div>
            <p class="text-gray-600 text-sm mb-6 font-bold">
              您确定要卸载 <span class="text-[#451a03]">{{ confirmUninstallTarget === 'hub' ? 'Unity Hub' : (confirmUninstallTarget === 'unity' ? 'Unity 2022.3.22f1' : (confirmUninstallTarget === 'vcc' ? 'VCC' : (confirmUninstallTarget === 'alcom' ? 'ALCOM' : 'Creator Tools'))) }}</span> 吗？卸载后需要重新安装。
            </p>
            <div class="flex gap-3">
              <button
                class="flex-1 py-2.5 rounded-2xl bg-gray-100 hover:bg-gray-200 text-gray-700 font-bold transition-colors"
                @click="showUninstallConfirm = false"
              >
                取消
              </button>
              <button
                class="flex-1 py-2.5 rounded-2xl bg-red-500 hover:bg-red-600 text-white font-bold shadow-lg shadow-red-500/30 border-2 border-red-400 transition-colors"
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
            class="absolute inset-0 bg-black/40 backdrop-blur-sm"
            @click="showLaunchToolConfirm = false"
          />
          <div class="bg-white/95 backdrop-blur-xl w-full max-w-sm rounded-3xl shadow-2xl relative z-10 p-6 border-2 border-white">
            <div class="flex justify-between items-center mb-4">
              <h3 class="text-xl font-extrabold text-amber-600 flex items-center gap-2">
                <AlertTriangle class="w-6 h-6" /> 选择启动工具
              </h3>
              <button
                class="p-1 rounded-full hover:bg-black/5 text-gray-500 transition-colors"
                @click="showLaunchToolConfirm = false"
              >
                <X class="w-5 h-5" />
              </button>
            </div>
            <p class="text-gray-600 text-sm mb-6 font-bold">
              系统检测到您同时安装了 <span class="text-[#451a03]">VCC</span> 和 <span class="text-[#451a03]">ALCOM</span>，请选择您要启动的工具：
            </p>
            <div class="flex gap-3">
              <button
                class="flex-1 py-2.5 rounded-2xl bg-amber-500 hover:bg-amber-600 text-white font-bold shadow-lg shadow-amber-500/30 border-2 border-amber-400 transition-colors"
                @click="executeLaunch('vcc')"
              >
                启动 VCC
              </button>
              <button
                class="flex-1 py-2.5 rounded-2xl bg-indigo-500 hover:bg-indigo-600 text-white font-bold shadow-lg shadow-indigo-500/30 border-2 border-indigo-400 transition-colors"
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
            class="absolute inset-0 bg-black/40 backdrop-blur-sm"
            @click="showUninstallToolSelection = false"
          />
          <div class="bg-white/95 backdrop-blur-xl w-full max-w-sm rounded-3xl shadow-2xl relative z-10 p-6 border-2 border-white">
            <div class="flex justify-between items-center mb-4">
              <h3 class="text-xl font-extrabold text-red-600 flex items-center gap-2">
                <AlertTriangle class="w-6 h-6" /> 选择卸载工具
              </h3>
              <button
                class="p-1 rounded-full hover:bg-black/5 text-gray-500 transition-colors"
                @click="showUninstallToolSelection = false"
              >
                <X class="w-5 h-5" />
              </button>
            </div>
            <p class="text-gray-600 text-sm mb-6 font-bold">
              系统检测到您同时安装了 <span class="text-[#451a03]">VCC</span> 和 <span class="text-[#451a03]">ALCOM</span>，请选择您要卸载的工具：
            </p>
            <div class="flex gap-3">
              <button
                class="flex-1 py-2.5 rounded-2xl bg-amber-500 hover:bg-amber-600 text-white font-bold shadow-lg shadow-amber-500/30 border-2 border-amber-400 transition-colors"
                @click="handleUninstallSelection('vcc')"
              >
                卸载 VCC
              </button>
              <button
                class="flex-1 py-2.5 rounded-2xl bg-indigo-500 hover:bg-indigo-600 text-white font-bold shadow-lg shadow-indigo-500/30 border-2 border-indigo-400 transition-colors"
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
          class="fixed inset-0 z-50 flex items-center justify-center p-4"
        >
          <div
            class="absolute inset-0 bg-black/20 backdrop-blur-sm"
            @click="showSettings = false"
          />
          <div class="bg-white/90 backdrop-blur-xl w-full max-w-sm rounded-3xl shadow-2xl relative z-10 p-8 border-2 border-white">
            <div class="flex justify-between items-center mb-6">
              <h2 class="text-2xl font-bold text-[#451a03] flex items-center gap-2">
                {{ t('env.settings') }} <Settings class="w-5 h-5 text-amber-500" />
              </h2>
              <button
                class="p-2 rounded-full hover:bg-black/5 text-amber-700 transition-colors"
                @click="showSettings = false"
              >
                <X class="w-6 h-6" />
              </button>
            </div>
            <div class="space-y-4 text-center">
              <div class="w-24 h-24 mx-auto rounded-full overflow-hidden border-4 border-amber-200 shadow-inner mb-4">
                <img
                  :src="dogImg"
                  class="w-full h-full object-cover"
                >
              </div>
              <h3 class="text-xl font-bold text-[#451a03]">
                VrcDog {{ t('env.dog_manager') }}
              </h3>
              <p class="text-amber-700/80 text-sm">
                v1.1.0 {{ t('env.dog_social') }}
              </p>
              <div class="pt-6 border-t border-amber-100">
                <p class="text-sm text-amber-800 flex items-center justify-center gap-1">
                  Made with <Heart class="w-4 h-4 text-pink-400 fill-pink-400 animate-pulse" /> for VRChat
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
