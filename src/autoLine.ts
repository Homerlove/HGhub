/* 自动选线：启动 / 换网时比一次，之后「按需」体检。
 *
 * 判活只有一种：真的拉一次列表 `/api/videos?page=1&size=1`，200 就算这条线路能用。
 * 首页 200 不算——`.ai` 首页好好的，`/api` 全 404，拿首页判活会把坏线路判成好的。
 *
 * 什么时候探：
 *   1. 启动 / 网络恢复 / 用户打开开关：直接比一次，选完就定
 *   2. 业务请求链路失败（前端 `call` 的观察点）：立刻体检，不用等定时器
 *   3. 低频巡检兜底：60s → 120s → 300s 逐级拉长，一切正常时几乎不耗电
 *   4. 页面不可见时完全不探
 *
 * 探到不通也不马上切：4 秒后再确认一次，连续两次不通才换，避免一次抖动就跳线路。
 */
import { api, hostOf, logLine, setNetObserver } from "./api";

const KEY = "hg.autoLine";
/** 两次「选线路」的最小间隔：换网事件与失败重试可能同时吵，别重复比 */
const PICK_COOLDOWN_MS = 15_000;
/** 健康时的巡检间隔，逐级拉长（省电的关键） */
const HEALTH_STEPS = [60_000, 120_000, 300_000];
/** 判不通后的复查间隔：4 秒，够快又不至于一秒一个请求 */
const RECHECK_MS = 4_000;

export const autoLineEnabled = (): boolean => {
  try { return localStorage.getItem(KEY) !== "off"; } catch { return true; }
};
export const setAutoLine = (on: boolean) => {
  try { localStorage.setItem(KEY, on ? "on" : "off"); } catch { /* 隐私模式写不进去，忽略 */ }
  if (!on) {
    stopTimer();
    return;
  }
  // 运行中重新打开：把事件与巡检补挂回去，并立刻比一次
  if (started && !attached) attach();
  if (started) void pickLine("manual", true);
};

let lastPickAt = 0;
let downStreak = 0;
let failStreak = 0;
let picking = false;
let checking = false;
let started = false;
let step = 0;
let timer = 0;

/** 最近一次选择结果，仅供展示 / 排查 */
export let lastPick: { base: string; tier: string; ms: number } | null = null;

function stopTimer() {
  window.clearTimeout(timer);
  timer = 0;
}

function schedule(delay: number) {
  stopTimer();
  if (!autoLineEnabled()) return;
  timer = window.setTimeout(() => {
    // 后台不探：省电，也避免锁屏时白跑
    if (document.visibilityState === "visible") void check("timer");
    else schedule(HEALTH_STEPS[0]);
  }, delay);
}

/** 比一次线路（.com → .ai 落地页上的镜像 → .ai），选完定下来。 */
export async function pickLine(reason: string, force = false): Promise<void> {
  if (!autoLineEnabled() || picking) return;
  if (!force && Date.now() - lastPickAt < PICK_COOLDOWN_MS) return;
  picking = true;
  try {
    const r = await api.lineAuto();
    lastPickAt = Date.now();
    downStreak = 0;
    failStreak = 0;
    step = 0;
    lastPick = { base: r.base, tier: r.pick.tier, ms: r.pick.ms };
    const detail = r.pick.candidates
      .map((c) => `${c.tier}:${c.ok ? "ok" : c.reachable ? "reachable" : "down"}/${c.ms}ms`)
      .join(",");
    logLine(
      `[line] auto(${reason}) ${hostOf(r.base)} tier=${r.pick.tier} ms=${r.pick.ms} ` +
        `switched=${r.switched} prev=${hostOf(r.previous)} | ${detail}`,
    );
    // 只有真的换了线路才提示，启动时本来就在这条线上就别打扰用户
    if (r.switched) {
      const region: "cn" | "intl" = r.pick.tier === "main" ? "intl" : "cn";
      window.dispatchEvent(
        new CustomEvent("hg:line", {
          detail: { region, host: hostOf(r.base), ms: r.pick.ms },
        }),
      );
    }
  } catch (e) {
    logLine(`[line] auto(${reason}) failed: ${(e as Error).message}`);
  } finally {
    picking = false;
    if (started) schedule(HEALTH_STEPS[0]);
  }
}

/** 体检当前线路：探真接口，不算延迟阈值。连续两次不通才换。 */
async function check(reason: string): Promise<void> {
  if (!autoLineEnabled() || checking) return;
  if (picking) {
    // 正在选线路，等它完了再体检（选完会自己重排定时器）
    schedule(HEALTH_STEPS[step]);
    return;
  }
  checking = true;
  try {
    const h = await api.lineHealth();
    if (h.ok) {
      downStreak = 0;
      step = Math.min(step + 1, HEALTH_STEPS.length - 1);
      schedule(HEALTH_STEPS[step]);
      return;
    }
    downStreak += 1;
    logLine(
      `[line] health(${reason}) down=${downStreak} base=${hostOf(h.base)} ` +
        `ms=${h.ms} status=${h.status ?? "-"}`,
    );
    if (downStreak < 2) {
      // 4 秒后再确认一次，别被一次抖动骗走
      schedule(RECHECK_MS);
      return;
    }
    downStreak = 0;
    step = 0;
    await pickLine(reason, true);
  } catch (e) {
    // 体检本身失败（本地协议不可用等）不代表线路坏了，按原节奏继续
    logLine(`[line] health(${reason}) failed: ${(e as Error).message}`);
    schedule(HEALTH_STEPS[step]);
  } finally {
    checking = false;
  }
}

let attached = false;

/** 真正挂事件与巡检。开关关掉再打开时也要能重新挂上，所以单独一层。 */
function attach(): void {
  if (attached) return;
  attached = true;
  // 业务请求链路失败 → 立刻体检，这是「无感切换」的主要来源
  setNetObserver((r) => {
    if (!autoLineEnabled()) return;
    if (r.ok) {
      failStreak = 0;
      return;
    }
    if (!r.networkish) return;
    if (++failStreak < 2) return;
    failStreak = 0;
    void check("api-fail");
  });
  window.addEventListener("online", () => {
    step = 0;
    void pickLine("online", true);
  });
  window.addEventListener("offline", () => {
    logLine("[line] offline，暂停巡检");
    stopTimer();
  });
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "hidden") {
      stopTimer();
      return;
    }
    downStreak = 0;
    step = 0;
    void check("visible");
  });
  schedule(HEALTH_STEPS[0]);
}

/** 启动时挂上：boot 比一次，之后靠 事件 + 巡检 兜底。 */
export function initAutoLine(): void {
  if (started) return;
  started = true;
  if (!autoLineEnabled()) return;
  attach();
  void pickLine("boot");
}

