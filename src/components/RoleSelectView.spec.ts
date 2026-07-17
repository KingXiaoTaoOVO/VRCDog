import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.hoisted(() => {
  const values = new Map<string, string>();
  const storage = {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, String(value)),
    removeItem: (key: string) => values.delete(key),
    clear: () => values.clear(),
    key: (index: number) => Array.from(values.keys())[index] ?? null,
    get length() {
      return values.size;
    },
  };

  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: storage,
  });
});

const mocks = vi.hoisted(() => ({
  getClientServerConfig: vi.fn(),
  getSetting: vi.fn(),
  pingServer: vi.fn(),
  request: vi.fn(),
  saveClientServerConfig: vi.fn(),
  saveSetting: vi.fn(),
  verifyServerPassword: vi.fn(),
}));

vi.mock('../api', () => ({
  DbApi: {
    getSetting: mocks.getSetting,
    saveSetting: mocks.saveSetting,
  },
  SysApi: {
    getClientServerConfig: mocks.getClientServerConfig,
    pingServer: mocks.pingServer,
    saveClientServerConfig: mocks.saveClientServerConfig,
    verifyServerPassword: mocks.verifyServerPassword,
  },
  VrcApi: {
    request: mocks.request,
  },
}));

import i18n, { setAppLocale } from '../i18n';
import RoleSelectView from './RoleSelectView.vue';

const mountRoleSelect = () => mount(RoleSelectView, {
  global: {
    plugins: [i18n],
  },
});

describe('RoleSelectView', () => {
  beforeEach(() => {
    localStorage.clear();
    setAppLocale('zh-CN', { persist: false });
    mocks.getClientServerConfig.mockReset().mockResolvedValue({
      server_url: 'http://127.0.0.1:11451',
    });
    mocks.getSetting.mockReset().mockResolvedValue(null);
    mocks.pingServer.mockReset().mockResolvedValue('ok');
    mocks.request.mockReset().mockResolvedValue({ success: true });
    mocks.saveClientServerConfig.mockReset().mockResolvedValue(undefined);
    mocks.saveSetting.mockReset().mockResolvedValue(undefined);
    mocks.verifyServerPassword.mockReset().mockResolvedValue(undefined);
  });

  it('keeps client mode to one persisted server URL field', async () => {
    const wrapper = mountRoleSelect();
    await flushPromises();

    await wrapper.findAll('.role-choice')[0].trigger('click');

    expect(wrapper.findAll('input[type="url"]')).toHaveLength(1);
    expect(wrapper.findAll('[role="tablist"]')).toHaveLength(0);
  });

  it('shows server target selection only after password verification', async () => {
    const wrapper = mountRoleSelect();
    await flushPromises();

    await wrapper.findAll('.role-choice')[1].trigger('click');

    expect(wrapper.findAll('[role="tablist"]')).toHaveLength(0);
    expect(wrapper.findAll('input[type="password"]')).toHaveLength(1);

    await wrapper.get('input[type="password"]').setValue('root');
    await wrapper.find('.primary-button').trigger('click');
    await flushPromises();

    expect(mocks.verifyServerPassword).toHaveBeenCalledWith({ password: 'root' });
    expect(wrapper.findAll('[role="tablist"]')).toHaveLength(1);
    expect(wrapper.findAll('[role="tab"]')).toHaveLength(2);
    expect(wrapper.findAll('input[type="password"]')).toHaveLength(0);
  });

  it('enters the local dashboard after selecting local service', async () => {
    const wrapper = mountRoleSelect();
    await flushPromises();

    await wrapper.findAll('.role-choice')[1].trigger('click');
    await wrapper.get('input[type="password"]').setValue('root');
    await wrapper.find('.primary-button').trigger('click');
    await flushPromises();
    await wrapper.find('.primary-button').trigger('click');
    await flushPromises();

    expect(wrapper.emitted('role-selected')).toEqual([[
      {
        role: 'server',
        serverMode: 'local',
        url: undefined,
        password: 'root',
      },
    ]]);
  });

  it('authenticates the selected remote dashboard before entering it', async () => {
    const wrapper = mountRoleSelect();
    await flushPromises();

    await wrapper.findAll('.role-choice')[1].trigger('click');
    await wrapper.get('input[type="password"]').setValue('root');
    await wrapper.find('.primary-button').trigger('click');
    await flushPromises();

    await wrapper.findAll('[role="tab"]')[1].trigger('click');
    await wrapper.get('input[type="url"]').setValue('https://admin.example.com/');
    await wrapper.find('.primary-button').trigger('click');
    await flushPromises();

    expect(mocks.pingServer).toHaveBeenCalledWith({ url: 'https://admin.example.com' });
    expect(mocks.request).toHaveBeenCalledWith(
      'https://admin.example.com/api/admin/auth',
      { method: 'POST', params: { password: 'root' } },
    );
    const emitted = wrapper.emitted('role-selected') || [];
    expect(emitted[emitted.length - 1]).toEqual([{
      role: 'server',
      serverMode: 'remote',
      url: 'https://admin.example.com',
      password: 'root',
    }]);
  });
});
