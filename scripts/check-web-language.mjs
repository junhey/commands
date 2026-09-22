/**
 * 检查站点源码里没有「裸中文」——也就是没走 i18n 的中文文案。
 *
 * 产品对外默认英文，中文只在浏览器语言是中文时出现。这条规则最容易在后续改动里
 * 破掉：加个按钮直接写中文，英文用户就看到中文了。
 *
 * 实现上刻意用 oxc 的真 AST（vite 自带），不手写 tokenizer。原因是 JSX 正文里
 * 的撇号、引号、斜杠都是普通文字：
 *
 *     <p>Don't put secrets on the command line</p>
 *     value.replace(/"/g, '\\"')
 *
 * 手写词法分析会把 `Don't` 的撇号当成字符串起始、把 `/"/g` 当成字符串，之后整个
 * 扫描状态全错位（第一版就是这么错的，还「检查通过」了一次假绿）。AST 没有这个问题。
 *
 * 判定规则：任何含中文的字符串字面量 / JSX 文本 / 模板串片段，其祖先链上必须有一个
 * `t(...)` 或 `tf(...)` 调用，且自己落在该调用的实参里。否则算漏网。
 *
 * 用法：node scripts/check-web-language.mjs <文件>...
 */

import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import process from "node:process";

const HAN = /[\u4e00-\u9fff]/;
const TRANSLATORS = new Set(["t", "tf"]);

// 双语数据表：一行里同时写英文和中文（内建命令表 [名称, 英文说明, 中文说明] 之类）。
// 这是刻意的「两种语言写在一起」，不是漏翻译，用注释标出范围让检查跳过。
const PRAGMA_BEGIN = "i18n-pairs-begin";
const PRAGMA_END = "i18n-pairs-end";

// vite 装在 web/ 下，从仓库根跑脚本时要从那里解析。
const repoRoot = path.resolve(import.meta.dirname, "..");
const require = createRequire(path.join(repoRoot, "web", "package.json"));
const { parseAst } = require("vite");

/** offset → 行号（1 起）。 */
function lineIndex(code) {
  const starts = [0];
  for (let i = 0; i < code.length; i += 1) {
    if (code[i] === "\n") starts.push(i + 1);
  }
  return (offset) => {
    let lo = 0;
    let hi = starts.length - 1;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (starts[mid] <= offset) lo = mid;
      else hi = mid - 1;
    }
    return lo + 1;
  };
}

function pragmaLines(code) {
  const skipped = new Set();
  let inside = false;
  code.split("\n").forEach((line, index) => {
    const number = index + 1;
    if (line.includes(PRAGMA_BEGIN)) {
      inside = true;
      skipped.add(number);
    } else if (line.includes(PRAGMA_END)) {
      inside = false;
      skipped.add(number);
    } else if (inside) {
      skipped.add(number);
    }
  });
  return skipped;
}

/** 这个节点是不是「携带中文的文案」。 */
function chineseText(node) {
  if (node.type === "Literal" && typeof node.value === "string") {
    return HAN.test(node.value) ? node.value : null;
  }
  if (node.type === "JSXText") {
    return HAN.test(node.value) ? node.value : null;
  }
  if (node.type === "TemplateElement") {
    const raw = node.value?.raw ?? "";
    return HAN.test(raw) ? raw : null;
  }
  return null;
}

/** 祖先链上有 t()/tf() 调用，且本节点在它的实参里。 */
function coveredByTranslator(stack) {
  for (let i = stack.length - 1; i >= 1; i -= 1) {
    const node = stack[i];
    if (node.type !== "CallExpression") continue;
    const callee = node.callee;
    const name =
      callee?.type === "Identifier"
        ? callee.name
        : callee?.type === "MemberExpression" &&
            callee.property?.type === "Identifier"
          ? callee.property.name
          : null;
    if (!name || !TRANSLATORS.has(name)) continue;
    // 必须落在实参里，而不是 callee 上。
    const child = stack[i + 1];
    if (node.arguments.includes(child) || containsNode(node.arguments, child)) {
      return true;
    }
  }
  return false;
}

function containsNode(args, child) {
  // stack[i + 1] 是 CallExpression 的直接子节点；实参可能被包在数组/表达式里，
  // 用位置区间判断更稳。
  if (!child) return false;
  return args.some((arg) => arg.start <= child.start && child.end <= arg.end);
}

function check(file) {
  const code = readFileSync(file, "utf8");
  const ast = parseAst(code, { lang: path.extname(file) === ".jsx" ? "jsx" : "js" });
  const toLine = lineIndex(code);
  const skipped = pragmaLines(code);
  const problems = [];
  const stack = [];

  (function walk(node) {
    if (!node || typeof node !== "object") return;
    if (Array.isArray(node)) {
      node.forEach(walk);
      return;
    }
    if (typeof node.type !== "string") return;

    stack.push(node);
    const text = chineseText(node);
    if (text && !coveredByTranslator(stack)) {
      const line = toLine(node.start);
      if (!skipped.has(line)) {
        problems.push({ line, text: text.trim().replace(/\s+/g, " ") });
      }
    }
    for (const key of Object.keys(node)) {
      if (key === "type" || key === "start" || key === "end") continue;
      walk(node[key]);
    }
    stack.pop();
  })(ast);

  return problems;
}

const files = process.argv.slice(2);
if (!files.length) {
  console.error("用法：node scripts/check-web-language.mjs <文件>...");
  process.exit(2);
}

let total = 0;
for (const file of files) {
  const problems = check(file);
  if (problems.length) {
    console.error(`=== ${file}：${problems.length} 处未经 i18n 的中文 ===`);
    for (const { line, text } of problems) {
      console.error(`  ${line}: ${text.slice(0, 90)}`);
    }
  }
  total += problems.length;
}

if (total) {
  console.error(
    `\n共 ${total} 处中文没有走 i18n。用 t("English", "中文") 包起来；` +
      `如果是刻意的双语数据表，用 ${PRAGMA_BEGIN} / ${PRAGMA_END} 注释标出范围。`,
  );
  process.exit(1);
}

console.log(`默认语言检查通过：${files.length} 个文件均无裸中文文案。`);
