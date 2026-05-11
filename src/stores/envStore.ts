import { defineStore } from 'pinia';
import { ref } from 'vue';
import { SysApi } from '../api';
import { isTauri } from '@tauri-apps/api/core';
import type { ComponentStatus } from '../components/StatusCard.vue';
import i18n from '../i18n';

export const useEnvStore = defineStore('env', () => {
  const t = i18n.global.t;
  
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
  const dialogConfig = ref({ title: '', target: '', isVccSelection: false });

  const checkEnvironment = async () => {
    hubStatus.value = 'checking'; unityStatus.value = 'checking'; toolStatus.value = 'checking';
    try {
      if (!isTauri()) throw new Error(t('auto_a4e099d3'));
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
    if (target === 'hub') dialogConfig.value = { title: t('app.install_hub') || t('auto_decf5dc6'), target: 'hub', isVccSelection: false };
    else if (target === 'unity') dialogConfig.value = { title: t('app.install_unity') || t('auto_676eab24'), target: 'unity', isVccSelection: false };
    else if (target === 'tool') dialogConfig.value = { title: t('app.install_tool') || t('auto_7f86d277'), target: 'tool', isVccSelection: true };
    showInstallDialog.value = true;
  };

  const handleUninstallSpecific = async (target: string, onError: (err: string) => void) => {
    if (target === 'hub') hubStatus.value = 'checking';
    if (target === 'unity') unityStatus.value = 'checking';
    if (target === 'tool' || target === 'vcc' || target === 'alcom') toolStatus.value = 'checking';
    try {
      if (!isTauri()) throw new Error(t('auto_48d0aedc'));
      await SysApi.uninstallSoftware({ target });
      await checkEnvironment();
    } catch (err: any) { 
      onError(err.message || err); 
      await checkEnvironment(); 
    }
  };

  const handleDialogConfirm = async (config: any) => {
    showInstallDialog.value = false;
    const target = dialogConfig.value.target;
    if (target === 'hub') hubStatus.value = 'installing';
    if (target === 'unity') hubStatus.value = 'installing';
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
      if (target === 'hub') { hubStatus.value = 'error'; hubProgressMsg.value = error.message || error; }
      if (target === 'unity') { unityStatus.value = 'error'; unityProgressMsg.value = error.message || error; }
      if (target === 'tool') { toolStatus.value = 'error'; toolProgressMsg.value = error.message || error; }
    }
  };

  return {
    hubStatus,
    hubProgress,
    hubProgressMsg,
    unityStatus,
    unityProgress,
    unityProgressMsg,
    toolStatus,
    toolProgress,
    toolProgressMsg,
    vccStatus,
    alcomStatus,
    showInstallDialog,
    dialogConfig,
    checkEnvironment,
    handleInstallClick,
    handleUninstallSpecific,
    handleDialogConfirm
  };
});
