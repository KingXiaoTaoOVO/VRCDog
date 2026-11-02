import { describe, expect, it } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import VrcAvatar from './VrcAvatar.vue';

describe('VrcAvatar', () => {
  it('uses the new VRChat iconUrl field', async () => {
    const iconUrl = 'https://api.vrchat.cloud/api/1/image/file_abc/1/256';
    const wrapper = mount(VrcAvatar, { props: { user: { id: 'usr_1', iconUrl } } });
    await flushPromises();
    expect(wrapper.find('img').attributes('src')).toBe(iconUrl);
  });

  it('keeps legacy avatar fields working', async () => {
    const legacy = 'https://api.vrchat.cloud/api/1/file/file_abc/1/file';
    const wrapper = mount(VrcAvatar, { props: { user: { id: 'usr_1', userIcon: legacy } } });
    await flushPromises();
    // 旧 file 地址会被改写成 256px 的 image 地址
    expect(wrapper.find('img').attributes('src')).toBe('https://api.vrchat.cloud/api/1/image/file_abc/1/256');
  });
});
