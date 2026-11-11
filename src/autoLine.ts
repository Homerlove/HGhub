/* 自动选线在门牌化版本停用：
 * 数据源来自用户输入的门牌，不再从发布页抓固定线路，也不做自动切换。
 * 这里保留同名兼容导出，设置页与旧调用方可以继续编译；阶段 3 会改接 gate_state。
 */

export const autoLineEnabled = (): boolean => false;

export const setAutoLine = (_on: boolean): void => {
  /* 本版没有自动选线，开关仅保留形状 */
};

export async function pickLine(_reason: string, _force = false): Promise<void> {
  /* no-op：门牌由用户手动输入 */
}

export function initAutoLine(): void {
  /* no-op */
}

export const lastPick: { base: string; tier: string; ms: number } | null = null;
