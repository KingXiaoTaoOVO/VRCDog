import { ref, onMounted, onUnmounted } from 'vue';
import { useToast } from './useToast';

export interface WebUpdateInfo {
  version: string;
  name: string;
  published_at: string;
  body: string;
  html_url: string;
}

const CHECK_INTERVAL_MS = 30 * 60 * 1000; // 30 minutes
const DEFAULT_UPDATE_SOURCE = 'https://api.github.com/repos/KingXiaoTaoOVO/vrcdog-releases/releases/latest';

function parseVersion(v: string): { nums: number[]; pre: string } {
  const core = v.trim().replace(/^v/i, '');
  const m = core.match(/^(\d+)(?:\.(\d+))?(?:\.(\d+))?(?:[-.](.+))?$/);
  if (!m) return { nums: [0, 0, 0], pre: '' };
  return {
    nums: [parseInt(m[1] || '0', 10), parseInt(m[2] || '0', 10), parseInt(m[3] || '0', 10)],
    pre: (m[4] || '').toLowerCase(),
  };
}

function cmpVersions(a: string, b: string): number {
  const pa = parseVersion(a);
  const pb = parseVersion(b);
  for (let i = 0; i < 3; i++) {
    if (pa.nums[i] !== pb.nums[i]) return pa.nums[i] < pb.nums[i] ? -1 : 1;
  }
  if (!pa.pre && pb.pre) return 1;
  if (pa.pre && pb.pre && pa.pre !== pb.pre) return pa.pre < pb.pre ? -1 : 1;
  return 0;
}

async function fetchUpdateFromSource(source: string): Promise<WebUpdateInfo | null> {
  try {
    const res = await fetch(source, {
      headers: { Accept: 'application/json' },
    });
    if (!res.ok) return null;
    const data = await res.json();
    const version = data.tag_name || data.version || data.name || '';
    const name = data.name || version;
    const published_at = data.published_at || data.publishedAt || '';
    const body = data.body || '';
    const html_url = data.html_url || data.url || '';
    if (!version) return null;
    return { version, name, published_at, body, html_url };
  } catch {
    return null;
  }
}

export function useWebUpdate() {
  const toast = useToast();
  const updateSource = ref<string>(DEFAULT_UPDATE_SOURCE);
  const currentVersion = ref<string>('');
  const latestVersion = ref<string>('');
  const updateAvailable = ref(false);
  const updateInfo = ref<WebUpdateInfo | null>(null);
  const checkingUpdate = ref(false);
  let timer: ReturnType<typeof setInterval> | null = null;

  const loadSettings = async () => {
    try {
      const raw = localStorage.getItem('vrcdog_web_update_source');
      if (raw) updateSource.value = raw;
    } catch {
      // ignore
    }
  };

  const saveSettings = async () => {
    try {
      localStorage.setItem('vrcdog_web_update_source', updateSource.value);
    } catch {
      // ignore
    }
  };

  const resetSettings = () => {
    updateSource.value = DEFAULT_UPDATE_SOURCE;
    saveSettings();
  };

  const checkForUpdates = async (silent = false): Promise<void> => {
    if (checkingUpdate.value || !currentVersion.value) return;
    checkingUpdate.value = true;
    try {
      const info = await fetchUpdateFromSource(updateSource.value);
      if (!info) {
        if (!silent) toast.warning('Web update check failed: unable to fetch update source');
        return;
      }
      latestVersion.value = info.version;
      if (cmpVersions(info.version, currentVersion.value) > 0) {
        updateAvailable.value = true;
        updateInfo.value = info;
        if (!silent) {
          toast.info(`New version ${info.version} available. Click to reload.`);
        }
      } else if (!silent) {
        toast.success('You are on the latest version');
      }
    } catch (err) {
      if (!silent) toast.error(`Update check failed: ${err}`);
    } finally {
      checkingUpdate.value = false;
    }
  };

  const applyUpdate = () => {
    if (updateInfo.value?.html_url) {
      window.open(updateInfo.value.html_url, '_blank');
    }
    window.location.reload();
  };

  onMounted(async () => {
    await loadSettings();
    checkForUpdates(true);
    timer = setInterval(() => checkForUpdates(true), CHECK_INTERVAL_MS);
  });

  onUnmounted(() => {
    if (timer) clearInterval(timer);
  });

  return {
    updateSource,
    currentVersion,
    latestVersion,
    updateAvailable,
    updateInfo,
    checkingUpdate,
    checkForUpdates,
    applyUpdate,
    resetSettings,
    saveSettings,
    loadSettings,
    DEFAULT_UPDATE_SOURCE,
  };
}
