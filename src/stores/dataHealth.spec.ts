import { beforeEach, describe, expect, it } from 'vitest';
import {
  DATA_REFRESH_MAX_SEC,
  DATA_REFRESH_MIN_SEC,
  dataHealth,
  dataRefresh,
  dataServiceStatus,
  healthyWindowMs,
  markDataHealthy,
  nowTs,
  setDataRefreshIntervalSec,
} from './dataHealth';

// 侧边栏那个「数据服务」指示器过去的行为是：健康窗口写死 10 分钟，
// 而后台刷新压根不存在，于是正常使用十几分钟后就永久显示「数据已过期」，
// 用户把设置里的「后台数据刷新频率」调到多少都不会有任何变化。
// 这组测试锁定修复后的不变式：窗口跟随刷新间隔，且间隔被夹在滑块的取值范围内。
describe('dataHealth 后台刷新间隔与过期窗口', () => {
  beforeEach(() => {
    dataHealth.lastSuccessAt = null;
    nowTs.value = Date.now();
    setDataRefreshIntervalSec(30);
  });

  it('把间隔夹到滑块允许的范围内', () => {
    expect(setDataRefreshIntervalSec(1)).toBe(DATA_REFRESH_MIN_SEC);
    expect(setDataRefreshIntervalSec(9999)).toBe(DATA_REFRESH_MAX_SEC);
    expect(dataRefresh.intervalSec).toBe(DATA_REFRESH_MAX_SEC);
  });

  it('非法值退回默认 30 秒而不是变成 NaN 或最激进的下限', () => {
    // `Number('')`/`Number(null)`/`Number(false)` 都是 0，只判 isFinite 会被夹成 10 秒下限
    for (const bad of [undefined, null, NaN, '', 'abc', {}, 0, -5, false, true]) {
      expect(setDataRefreshIntervalSec(bad)).toBe(30);
    }
  });

  it('窗口至少 5 分钟，并随间隔线性放大', () => {
    setDataRefreshIntervalSec(10);
    // 10s × 20 次 = 200s < 5 分钟下限
    expect(healthyWindowMs()).toBe(5 * 60 * 1000);

    setDataRefreshIntervalSec(30);
    expect(healthyWindowMs()).toBe(10 * 60 * 1000);

    setDataRefreshIntervalSec(120);
    expect(healthyWindowMs()).toBe(120 * 20 * 1000);
  });

  it('从未成功过是「暂无数据」，成功过但太久没刷才是「已过期」', () => {
    expect(dataServiceStatus.value).toBe('offline');

    markDataHealthy();
    expect(dataServiceStatus.value).toBe('online');

    setDataRefreshIntervalSec(30);
    nowTs.value = Date.now() + 10 * 60 * 1000 + 1000;
    expect(dataServiceStatus.value).toBe('stale');
  });

  it('把间隔调大之后，同一个时间点上不再被误判为过期', () => {
    // 这是用户实际踩到的场景：数据是几分钟前刷的，指示器却已经变黄。
    dataHealth.lastSuccessAt = Date.now() - 12 * 60 * 1000;
    nowTs.value = Date.now();
    setDataRefreshIntervalSec(30);
    expect(dataServiceStatus.value).toBe('stale');

    // 间隔 120s 时允许 40 分钟无刷新
    setDataRefreshIntervalSec(120);
    expect(dataServiceStatus.value).toBe('online');
  });
});
