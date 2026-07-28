import type { OscSystemSnapshot } from '../api';

export const OSC_CHATBOX_LIMIT = 144;

const percent = (value: number | null) => value == null ? '--' : `${Math.round(value)}%`;

export function formatGigabytes(value: number | null, digits = 1): string {
  if (value == null || !Number.isFinite(value) || value <= 0) return '--';
  const rounded = value.toFixed(digits).replace(/\.0$/, '');
  return `${rounded} GB`;
}

export function formatDuration(seconds: number): string {
  const total = Math.max(0, Math.floor(seconds));
  const days = Math.floor(total / 86_400);
  const hours = Math.floor((total % 86_400) / 3_600);
  const minutes = Math.floor((total % 3_600) / 60);
  if (days > 0) return `${days}天 ${hours}小时`;
  if (hours > 0) return `${hours}小时 ${minutes}分钟`;
  if (minutes > 0) return `${minutes}分钟`;
  return `${total}秒`;
}

export function formatHardwareUsage(snapshot: OscSystemSnapshot): string {
  const memory = `${formatGigabytes(snapshot.memoryUsedGb).replace(' GB', '')}/${formatGigabytes(snapshot.memoryTotalGb)}`;
  const lines = [
    `CPU ${percent(snapshot.cpuUsage)} · GPU ${percent(snapshot.gpuUsage)}`,
    `RAM ${memory}`,
  ];
  if (snapshot.gpuMemoryUsedGb != null && snapshot.gpuMemoryTotalGb != null) {
    lines[1] += ` · VRAM ${formatGigabytes(snapshot.gpuMemoryUsedGb).replace(' GB', '')}/${formatGigabytes(snapshot.gpuMemoryTotalGb)}`;
  }
  return lines.join('\n');
}

export function formatHardwareSpecs(snapshot: OscSystemSnapshot): string {
  const cores = snapshot.cpuPhysicalCores > 0
    ? `${snapshot.cpuPhysicalCores}核${snapshot.cpuLogicalCores}线程`
    : `${snapshot.cpuLogicalCores}线程`;
  return [
    snapshot.cpuName ? `${snapshot.cpuName} · ${cores}` : cores,
    snapshot.gpuName,
    `内存 ${formatGigabytes(snapshot.memoryTotalGb)} · 磁盘 ${formatGigabytes(snapshot.diskTotalGb, 0)}`,
  ].filter(Boolean).join('\n');
}

export function oscTextLength(text: string): number {
  return Array.from(text).length;
}

export function truncateOscText(text: string, limit = OSC_CHATBOX_LIMIT): string {
  const characters = Array.from(text);
  if (characters.length <= limit) return text;
  if (limit <= 1) return characters.slice(0, limit).join('');
  return `${characters.slice(0, limit - 1).join('')}…`;
}

export function renderOscTemplate(
  template: string,
  replacements: Record<string, string>,
  limit = OSC_CHATBOX_LIMIT,
): string {
  let result = template;
  Object.entries(replacements).forEach(([key, value]) => {
    result = result.split(key).join(value);
  });
  const normalized = result
    .split(/\r?\n/)
    .map((line) => line.replace(/[ \t]{2,}/g, ' ').trim())
    .filter(Boolean)
    .join('\n');
  return truncateOscText(normalized, limit);
}
