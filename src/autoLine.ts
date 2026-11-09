/* 自动选线：启动时比一次 `.com` / `.ai`，运行中感知网络变化再比。

   站点只有两条入口，国内基本只能连 `.ai`，海外 `.ai` 一定比 `.com` 慢（它是国内专线）。
   所以这里不判断地区、不扫镜像、不看延迟阈值——两条各探一次，通的那条里快的那条胜出，
   判断全在 Rust 的 `lines::auto_pick` 里做，前端只负责「什么时候探」和日志。 */
import { api, hostOf, logLine } from "./api";

const KEY = "hg.autoLine";
/** 两次自动探测的最小间隔：换网事件和定时体检可能同时触发，别重复探 */
const PICK_COOLDOWN_MS = 20_000;
/** 定时体检间隔：WiFi↔流量切换不会触发 online 事件，只能靠轮询兜底 */
const HEALTH_INTERVAL_MS = 60_000;

export const autoLineEnabled = (): boolean => {
  try { return localStorage.getItem(KEY) !== "off"; } catch { return true; }
};

export const setAutoLine = (on: boolean) => {
  try { localStorage.setItem(KEY, on ? "on" : "off"); } catch { /* 隐私模式写不进去，忽略 */ }
};

let lastPickAt = 0;
let downStreak = 0;
let running = false;
let started = false;

/** 最近一次选择结果，仅供展示/排查 */
export let lastPick: { base: string; tier: string; ms: number } | null = null;

/** 比一次线路。`reason` 只进日志；`force` 跳过节流（用户手动点、换网事件）。 */
export async function pickLine(reason: string, force = false): Promise<void> {
  if (!autoLineEnabled() || running) return;
  if (!force && Date.now() - lastPickAt < PICK_COOLDOWN_MS) return;
  running = true;
  try {
    const r = await api.lineAuto();
    lastPickAt = Date.now();
    downStreak = 0;
    lastPick = { base: r.base, tier: r.pick.tier, ms: r.pick.ms };
    const detail = r.pick.candidates
      .map((c) => `${c.tier}:${c.ok ? "ok" : c.reachable ? "reachable" : "down"}/${c.ms}ms`)
      .join(",");
    logLine(
      `[line] auto(${reason}) ${hostOf(r.base)} tier=${r.pick.tier} ms=${r.pick.ms} ` +
        `switched=${r.switched} prev=${hostOf(r.previous)} | ${detail}`,
    );
  } catch (e) {
    logLine(`[line] auto(${reason}) failed: ${(e as Error).message}`);
  } finally {
    running = false;
  }
}

/** 体检当前线路：连续两次不通才换，避免一次抖动就切走。 */
async function healthCheck(reason: string): Promise<void> {
  if (!autoLineEnabled() || running) return;
  try {
    const s = await api.lineState();
    const [probe] = await api.lineProbe([s.base]);
    if (probe?.ok) {
      downStreak = 0;
      return;
    }
    downStreak += 1;
    logLine(`[line] health(${reason}) down=${downStreak} base=${hostOf(s.base)}`);
    if (downStreak >= 2) {
      downStreak = 0;
      await pickLine(reason, true);
    }
  } catch {
    /* 体检本身失败（本地协议不可用等）不代表线路坏了，不计数 */
  }
}

/** 启动时挂上：boot 比一次，之后 online / 回前台 / 定时复查三种时机。 */
export function initAutoLine(): void {
  if (started) return;
  started = true;
  if (!autoLineEnabled()) return;
  void pickLine("boot");
  window.addEventListener("online", () => { void pickLine("online", true); });
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible") void healthCheck("visible");
  });
  window.setInterval(() => {
    if (document.visibilityState === "visible") void healthCheck("tick");
  }, HEALTH_INTERVAL_MS);
}
