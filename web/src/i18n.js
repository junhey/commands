/**
 * 站点界面语言。**默认英文**，浏览器语言是中文时自动切中文，用户手动选过之后
 * 以手动选择为准（存在 localStorage）。
 *
 * 和 CLI 侧 `cli/src/i18n.rs` 的 `t!(en, zh)` 刻意同构：同一句话的两种语言写在
 * 一起，改文案时两种语言都在眼前，不容易只改一半。
 *
 * 这个模块保持纯 JS、不依赖 React，这样 commands.js（Playground 内核，有自己的
 * node:test 测试）也能直接用。
 */

const KEY = "cmds-lang";

function fromBrowser() {
  const list =
    typeof navigator !== "undefined" && navigator.languages?.length
      ? navigator.languages
      : [(typeof navigator !== "undefined" && navigator.language) || ""];
  return list.some((item) => String(item).toLowerCase().startsWith("zh"))
    ? "zh"
    : "en";
}

function detect() {
  try {
    const saved = localStorage.getItem(KEY);
    if (saved === "en" || saved === "zh") return saved;
  } catch {
    // 隐私模式下读 localStorage 会抛，忽略后按浏览器语言判断
  }
  return fromBrowser();
}

let current = typeof window === "undefined" ? "en" : detect();
const listeners = new Set();

export function lang() {
  return current;
}

export function isChinese() {
  return current === "zh";
}

export function setLang(next) {
  if (next !== "en" && next !== "zh" || next === current) return;
  current = next;
  try {
    localStorage.setItem(KEY, next);
  } catch {
    // 存不下就只在本次会话生效
  }
  applyDocumentLang();
  listeners.forEach((fn) => fn(next));
}

export function subscribe(fn) {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

/**
 * 选一条文案：`t("Install", "安装")`。
 *
 * 返回的是原样的入参，所以也能传 JSX：`t(<>see <code>help</code></>, <>看 <code>help</code></>)`。
 * 段落里混着 `<code>` 时，逐段拆成字符串会被语序拆散（英文的定语在前、中文在后），
 * 整块传 JSX 反而更好读也更难写错。
 */
export function t(en, zh) {
  return current === "zh" ? zh : en;
}

/**
 * 同步 `<html lang>`、`<title>` 与 meta description。
 *
 * index.html 里这三样固定写英文——那是给搜索引擎、社交预览和 React 启动前的首屏用的，
 * 那时没有 JS 可以判断语言。中文用户进来后由这里改写。
 */
export function applyDocumentLang() {
  if (typeof document === "undefined") return;
  document.documentElement.lang = current === "zh" ? "zh-CN" : "en";
  document.title = t(
    "Commands — a small, fast interactive shell",
    "Commands — 轻量高效的交互式终端",
  );
  const description = document.querySelector('meta[name="description"]');
  if (description) {
    description.setAttribute(
      "content",
      t(
        "Commands (cmds) — a small, fast interactive shell. History suggestions as you type, a Tab candidate menu, and a starship-style modular prompt. One executable, no runtime dependencies.",
        "Commands (cmds) — 轻量高效的交互式终端。边打字边给历史建议，Tab 弹候选菜单，starship 风格的模块化提示符。一个可执行文件，零运行时依赖。",
      ),
    );
  }
}

/** 只给测试用：把语言强制成某个值，避免依赖运行环境。 */
export function setLangForTest(next) {
  current = next === "zh" ? "zh" : "en";
}
