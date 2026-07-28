import { describe, expect, it } from 'vitest';
import type { OscSystemSnapshot } from '../api';
import {
  formatHardwareSpecs,
  formatHardwareUsage,
  oscTextLength,
  renderOscTemplate,
  truncateOscText,
} from './oscFormatting';

const snapshot: OscSystemSnapshot = {
  cpuUsage: 28.4,
  cpuName: 'Intel Core i7-12700K',
  cpuPhysicalCores: 12,
  cpuLogicalCores: 20,
  cpuFrequencyMhz: 4900,
  ramUsage: 54.2,
  memoryUsedGb: 17.3,
  memoryTotalGb: 32,
  gpuName: 'NVIDIA GeForce RTX 4070',
  gpuUsage: 41,
  gpuMemoryUsedGb: 4.8,
  gpuMemoryTotalGb: 12,
  diskUsage: 66,
  diskUsedGb: 1320,
  diskTotalGb: 2000,
  osName: 'Windows 11 Pro',
  hostName: 'VRC-PC',
  systemUptimeSeconds: 3600,
  idleSeconds: 12,
  activeWindow: 'VRChat',
  localTime: '20:15:30',
  localDate: '2026-07-26',
  vrcRunning: true,
};

describe('OSC system text formatting', () => {
  it('lays out live usage in compact aligned groups', () => {
    expect(formatHardwareUsage(snapshot)).toBe(
      'CPU 28% · GPU 41%\nRAM 17.3/32 GB · VRAM 4.8/12 GB',
    );
  });

  it('includes automatically detected hardware specs', () => {
    expect(formatHardwareSpecs(snapshot)).toContain('Intel Core i7-12700K · 12核20线程');
    expect(formatHardwareSpecs(snapshot)).toContain('NVIDIA GeForce RTX 4070');
  });

  it('removes empty template lines and preserves readable line breaks', () => {
    expect(renderOscTemplate('{message}\n{time}  {hardware}\n{music}', {
      '{message}': '',
      '{time}': '20:15',
      '{hardware}': 'CPU 28%\nRAM 54%',
      '{music}': '',
    })).toBe('20:15 CPU 28%\nRAM 54%');
  });

  it('truncates by Unicode code points without breaking emoji', () => {
    const result = truncateOscText(`🎹${'配'.repeat(150)}`, 144);
    expect(oscTextLength(result)).toBe(144);
    expect(result.startsWith('🎹')).toBe(true);
    expect(result.endsWith('…')).toBe(true);
  });
});
