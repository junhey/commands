import test from "node:test";
import assert from "node:assert/strict";
import {
  buildConfigToml,
  builtinNames,
  getSuggestions,
  ghostSuggestion,
  isKnownCommand,
  levenshtein,
  loadStored,
  similarCommands,
  simulateCommand,
} from "../src/commands.js";
import { setLangForTest } from "../src/i18n.js";

// 文案随界面语言变。测试里显式固定语言，别让本机 locale 决定结果。
setLangForTest("en");

const history = [
  { command: "git status", count: 12 },
  { command: "git push origin master", count: 3 },
  { command: "cargo build --release --locked", count: 8 },
];

/* ─────────────────────────── 候选与建议 ─────────────────────────── */

test("匹配不区分大小写，历史排在推荐之前", () => {
  const matches = getSuggestions("GIT ", history);
  assert.equal(matches[0].source, "history");
  assert.ok(matches.every((item) => item.command.toLowerCase().includes("git")));
});

test("同一条命令不会既出现在历史又出现在推荐里", () => {
  const matches = getSuggestions("git", history);
  assert.equal(
    new Set(matches.map((item) => item.command)).size,
    matches.length,
  );
});

test("历史里高频的命令排在低频之前", () => {
  const matches = getSuggestions("git ", history).filter(
    (item) => item.source === "history",
  );
  const status = matches.findIndex((item) => item.command === "git status");
  const push = matches.findIndex(
    (item) => item.command === "git push origin master",
  );
  assert.ok(status >= 0 && push >= 0);
  assert.ok(status < push, "count 更高的 git status 应更靠前");
});

test("历史与推荐可以分别关闭", () => {
  assert.ok(
    getSuggestions("git", history, { history: false }).every(
      (item) => item.source === "suggested",
    ),
  );
  assert.ok(
    getSuggestions("git", history, { recommendations: false }).every(
      (item) => item.source === "history",
    ),
  );
  assert.deepEqual(
    getSuggestions("git", history, { history: false, recommendations: false }),
    [],
  );
});

test("无匹配返回空，空查询数量有上界", () => {
  assert.deepEqual(getSuggestions("missing-example-command", history), []);
  assert.ok(getSuggestions("", history).length <= 5);
});

test("ghost 建议只补前缀匹配的尾巴，以空格开头时不给建议", () => {
  assert.equal(ghostSuggestion("git st", history), "atus");
  assert.equal(ghostSuggestion(" git st", history), "");
  assert.equal(ghostSuggestion("", history), "");
  assert.equal(ghostSuggestion("git status", history), "");
});

/* ───────────────────── 拼写建议：与 Rust 侧同规则 ───────────────────── */

test("levenshtein 与 Rust 实现一致", () => {
  assert.equal(levenshtein("hepl", "help"), 2);
  assert.equal(levenshtein("", "abc"), 3);
  assert.equal(levenshtein("abc", ""), 3);
  assert.equal(levenshtein("同", "同"), 0);
});

test("内建命令优先于 PATH 里的同距离噪声", () => {
  // head 也和 hepl 距离 2，但内建 help 必须排第一
  assert.equal(similarCommands("hepl", 5)[0], "help");
});

test("不会把用户输入的原词当成建议", () => {
  assert.ok(!similarCommands("help", 5).includes("help"));
});

test("短输入收紧阈值，避免噪声", () => {
  assert.ok(similarCommands("ls", 5).every((name) => name !== "config"));
});

test("命令是否存在的判断覆盖内建、别名、缩写与 PATH", () => {
  assert.ok(isKnownCommand("help"));
  assert.ok(isKnownCommand("git"));
  assert.ok(isKnownCommand("ll"));
  assert.ok(isKnownCommand("gcm"));
  assert.ok(!isKnownCommand("definitely-not-a-command-xyz"));
  assert.ok(!isKnownCommand(""));
});

/* ─────────────────────────── 模拟执行 ─────────────────────────── */

test("cd / pwd / clear 只做字符串处理，不执行任何代码", () => {
  assert.equal(
    simulateCommand("cd cli", "~/projects/commands").cwd,
    "~/projects/commands/cli",
  );
  assert.equal(
    simulateCommand("cd ..", "~/projects/commands").cwd,
    "~/projects",
  );
  assert.equal(simulateCommand("pwd", "~/projects").output[0], "/home/you/projects");
  assert.equal(simulateCommand("clear").clear, true);
  // 带 shell 元字符的路径被拒绝，避免演示出「像是真的执行了」的错觉
  assert.equal(simulateCommand("cd cli; touch sample").exit, 1);
});

test("cd - 回到上一个目录", () => {
  const result = simulateCommand("cd -", "~/projects/commands", {
    previousCwd: "~/tmp",
  });
  assert.equal(result.cwd, "~/tmp");
});

test("未知命令返回 127 并给出拼写建议", () => {
  const result = simulateCommand("hepl");
  assert.equal(result.exit, 127);
  assert.ok(result.output[0].includes("help"));
});

test("输出明确声明是模拟，不会启动任何服务", () => {
  assert.ok(
    simulateCommand("npm run dev").output.some((line) =>
      line.includes("no server process"),
    ),
  );
  assert.ok(
    simulateCommand("cargo build").output.some((line) =>
      line.includes("simulated output"),
    ),
  );
});

test("echo 去掉引号并支持 -n/-e", () => {
  assert.equal(simulateCommand('echo "hello"').output[0], "hello");
  assert.equal(simulateCommand("echo -n hello").output[0], "hello");
});

test("help 列出全部内建命令", () => {
  const text = simulateCommand("help").output.join("\n");
  for (const name of builtinNames) {
    assert.ok(text.includes(name), `help 输出应包含内建命令 ${name}`);
  }
});

test("true / false 的退出码正确", () => {
  assert.equal(simulateCommand("true").exit, 0);
  assert.equal(simulateCommand("false").exit, 1);
});

test("type 能区分内建、别名、缩写与外部命令", () => {
  assert.ok(simulateCommand("type cd").output[0].includes("builtin"));
  assert.ok(simulateCommand("type ll").output[0].includes("alias"));
  assert.ok(simulateCommand("type gcm").output[0].includes("abbreviation"));
  assert.ok(simulateCommand("type git").output[0].includes("/usr/bin/git"));
  assert.equal(simulateCommand("type nope-xyz").exit, 1);
});

test("中文界面下 type 给出中文说明", () => {
  setLangForTest("zh");
  try {
    assert.ok(simulateCommand("type cd").output[0].includes("内建"));
    assert.ok(simulateCommand("type ll").output[0].includes("别名"));
    assert.ok(simulateCommand("type gcm").output[0].includes("缩写"));
  } finally {
    setLangForTest("en");
  }
});

test("默认英文界面下模拟输出不含中文", () => {
  setLangForTest("en");
  const probes = [
    "help",
    "jobs",
    "history",
    "config path",
    "config show",
    "config init",
    "config reload",
    "type cd",
    "type ll",
    "type gcm",
    "type nope-xyz",
    "alias gg=\"git pull\"",
    "abbr gg \"git pull\"",
    "git status",
    "git log",
    "git push",
    "npm run dev",
    "npm run build",
    "npm install",
    "cargo build",
    "cargo test",
    "nonexistent-command-xyz",
    "cd /tmp",
    "cd ;rm -rf /",
  ];
  for (const command of probes) {
    const text = simulateCommand(command).output.join("\n");
    assert.ok(
      !/[\u4e00-\u9fff]/.test(text),
      `英文界面下 \`${command}\` 的输出混进了中文：${text}`,
    );
  }
});

test("config path/init/show 指向真实的配置位置", () => {
  assert.ok(
    simulateCommand("config path").output[0].includes(".config/cmds/config.toml"),
  );
  assert.ok(simulateCommand("config init").output[0].includes("config.toml"));
  assert.ok(simulateCommand("config show").output.join("\n").includes("format ="));
});

test("git checkout -b 会切换分支上下文", () => {
  const result = simulateCommand("git checkout -b feature/menu");
  assert.equal(result.branch, "feature/menu");
  assert.equal(result.exit, 0);
});

test("空命令是安全的 no-op", () => {
  assert.deepEqual(simulateCommand("   "), { output: [], exit: 0 });
});

/* ─────────────────────── 配置生成 ─────────────────────── */

test("默认配置是合法且完整的 TOML 片段", () => {
  const toml = buildConfigToml();
  assert.ok(toml.includes('format = "'));
  assert.ok(toml.includes("$line_break$character"));
  assert.ok(toml.includes("add_newline = true"));
  assert.ok(toml.includes("[autosuggest]"));
  assert.ok(toml.includes("[menu]"));
  assert.ok(toml.includes("[git]"));
  assert.ok(toml.includes("[aliases]"));
  assert.ok(toml.includes("[abbreviations]"));
  assert.ok(toml.endsWith("\n"));
});

test("取消勾选的模块不会出现在 format 里", () => {
  const toml = buildConfigToml({
    modules: ["$dir", "$character"],
  });
  const format = toml.match(/format = "(.*)"/)[1];
  assert.equal(format, "$dir$line_break$character");
  assert.ok(!format.includes("$git_branch"));
});

test("模块顺序固定，不受勾选顺序影响", () => {
  const toml = buildConfigToml({
    modules: ["$character", "$git_branch", "$dir"],
  });
  assert.equal(
    toml.match(/format = "(.*)"/)[1],
    "$dir$git_branch$line_break$character",
  );
});

test("关掉 line_break 时 character 紧跟在同一行", () => {
  const toml = buildConfigToml({
    modules: ["$dir", "$character"],
    lineBreak: false,
  });
  assert.equal(toml.match(/format = "(.*)"/)[1], "$dir$character");
});

test("全部取消勾选时仍产出可用的 format", () => {
  assert.equal(
    buildConfigToml({ modules: [] }).match(/format = "(.*)"/)[1],
    "$character",
  );
});

test("关闭 Git 状态时写出 false 并附带说明", () => {
  const toml = buildConfigToml({ gitStatus: false });
  assert.ok(toml.includes("status_enabled = false"));
});

test("别名与缩写里的引号会被转义", () => {
  const toml = buildConfigToml({
    abbreviations: { gcm: 'git commit -m ""' },
    aliases: {},
  });
  assert.ok(toml.includes('gcm = "git commit -m \\"\\""'));
  assert.ok(!toml.includes("[aliases]"));
});

/* ─────────────────────── 存储降级 ─────────────────────── */

test("没有 localStorage 时读取会回退到默认值", () => {
  assert.equal(loadStored("unknown", "fallback"), "fallback");
});
