// c-Clean i18n（task-086 5.6）
//
// # 为什么是「手写 key-value + useSyncExternalStore」而不是 i18next
//
// 需求只有两点：Settings 能切语言、核心 UI 跟着变。i18next 会带来
// provider / 插件 / 复数规则等一整套概念，而本项目没有复数、没有插值以外的
// 格式需求。这里用 30 行实现同一件事，且**不引入任何依赖**（任务书明确
// 要求「简单 key-value 字典，不引入库」）。
//
// # 语言存在哪
//
// `localStorage`。语言是**纯客户端展示偏好**：后端不读它、数据库里也不该为
// 它加列（主题已有 prefs，但语言切换必须同步生效且不依赖 DB 往返）。
// 读写失败（隐私模式）时退回默认中文，不影响任何功能。

import { useSyncExternalStore } from "react";
import enUS from "./en-US.json";
import zhCN from "./zh-CN.json";

export type Lang = "zh-CN" | "en-US";

const DICTS: Record<Lang, Record<string, string>> = {
  "zh-CN": zhCN,
  "en-US": enUS,
};

const STORAGE_KEY = "cclean:lang";
const DEFAULT_LANG: Lang = "zh-CN";

function isLang(v: string | null): v is Lang {
  return v === "zh-CN" || v === "en-US";
}

function readStoredLang(): Lang {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (isLang(v)) return v;
  } catch {
    // 隐私模式 / 禁用存储：退回默认
  }
  return DEFAULT_LANG;
}

let current: Lang = readStoredLang();
const listeners = new Set<() => void>();

/** 当前语言（非 React 环境可直接用）。 */
export function getLang(): Lang {
  return current;
}

/** 切换语言；持久化失败只影响下次启动的默认值。 */
export function setLang(lang: Lang): void {
  if (lang === current) return;
  current = lang;
  try {
    localStorage.setItem(STORAGE_KEY, lang);
  } catch {
    // 忽略：本次会话内仍然生效
  }
  for (const fn of listeners) fn();
}

function subscribe(fn: () => void): () => void {
  listeners.add(fn);
  return () => {
    listeners.delete(fn);
  };
}

/**
 * 取一条文案。
 *
 * 缺失 key 时按「英文 → 中文 key 本身」回退：英文词典漏翻时界面显示中文
 * 比显示 `settings.xxx` 更可读，同时又不至于把中文硬编码进组件。
 */
export function t(key: string, vars?: Record<string, string | number>): string {
  const dict = DICTS[current];
  let text = dict[key] ?? DICTS[DEFAULT_LANG][key] ?? key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      text = text.replaceAll(`{${k}}`, String(v));
    }
  }
  return text;
}

/**
 * React Hook：订阅语言变化并触发重渲染。
 *
 * 只在需要跟随语言变化的组件里调用（App 外壳即可——它重渲染时子树整体重算）。
 */
export function useLang(): Lang {
  return useSyncExternalStore(subscribe, getLang, getLang);
}
