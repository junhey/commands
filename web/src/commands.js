/**
 * Playground 内核：在浏览器里模拟 `cmds` 的交互行为。
 *
 * 这里刻意与 Rust 实现保持同一套规则，这样网站演示的手感和真实安装后的手感一致：
 *  - 内建命令清单与 cli/src/builtins.rs 的 BUILTINS 一一对应
 *  - 候选排序：历史整条命令最前，其后内建 / 别名 / 缩写 / PATH 命令
 *  - 历史排序：使用频率 + 新鲜度，前缀匹配优先
 *  - 拼错命令的建议算法与 cli/src/shell.rs 的 similar_commands 一致
 *
 * 所有命令都是模拟执行：不读本地文件、不起进程、不发网络请求。
 */

export const BRAND = {
  name: "Commands",
  bin: "cmds",
  version: "0.1.1",
  repo: "junhey/commands",
  repoUrl: "https://github.com/junhey/commands",
  tagline: "少敲一点，多做一点。",
};

/** 与 cli/src/builtins.rs 的 BUILTINS 对齐。 */
export const builtins = [
  ["abbr", "查看/设置缩写，输入后按空格展开"],
  ["alias", "查看/设置别名"],
  ["cd", "切换目录，cd - 回到上一个目录"],
  ["clear", "清屏"],
  ["config", "配置管理：path / init / reload / show"],
  ["echo", "输出文本，支持 -n / -e"],
  ["exit", "退出 shell，可带退出码"],
  ["export", "设置环境变量，也可写作 set"],
  ["false", "什么都不做，返回 1"],
  ["help", "查看使用说明与快捷键"],
  ["history", "查看/搜索/清空历史"],
  ["jobs", "查看后台任务"],
  ["pwd", "打印当前目录"],
  ["set", "设置环境变量（export 的别名）"],
  ["source", "在当前 shell 执行脚本，别名 ."],
  ["true", "什么都不做，返回 0"],
  ["type", "查看命令类型，别名 which"],
  ["unabbr", "删除缩写"],
  ["unalias", "删除别名"],
  ["unset", "删除环境变量"],
  ["which", "查看命令类型"],
];

export const builtinNames = builtins.map(([name]) => name);

/** 演示用的 PATH 命令表，模拟真实机器上能补全到的外部命令。 */
export const pathCommands = [
  "cargo",
  "curl",
  "docker",
  "git",
  "grep",
  "head",
  "ls",
  "node",
  "npm",
  "pnpm",
  "python3",
  "rg",
  "rustc",
  "ssh",
  "tar",
];

export const defaultAbbreviations = {
  gcm: "git commit -m",
  gco: "git checkout",
  gst: "git status",
};

export const defaultAliases = {
  ll: "ls -lah",
};

export const commandCatalog = [
  {
    command: "git status",
    description: "看看工作区改了什么",
    group: "Git",
    source: "history",
  },
  {
    command: 'git commit -m "fix: 补全越界"',
    description: "提交当前暂存的改动",
    group: "Git",
    source: "history",
  },
  {
    command: "git checkout -b feature/menu",
    description: "新建并切换分支",
    group: "Git",
    source: "history",
  },
  {
    command: "git log --oneline -5",
    description: "最近五条提交",
    group: "Git",
    source: "suggested",
  },
  {
    command: "git diff",
    description: "查看未暂存的改动",
    group: "Git",
    source: "suggested",
  },
  {
    command: "cargo build --release --locked",
    description: "构建发布版二进制",
    group: "Rust",
    source: "history",
  },
  {
    command: "cargo test",
    description: "跑单元测试",
    group: "Rust",
    source: "suggested",
  },
  {
    command: "npm run dev",
    description: "启动本地开发服务",
    group: "Node.js",
    source: "history",
  },
  {
    command: "npm run build",
    description: "产出生产构建",
    group: "Node.js",
    source: "suggested",
  },
  {
    command: "ls -lah",
    description: "列出全部文件与详情",
    group: "Files",
    source: "history",
  },
  {
    command: "pwd",
    description: "打印当前目录",
    group: "Shell",
    source: "suggested",
  },
  {
    command: 'abbr gcm "git commit -m"',
    description: "定义一个 fish 式缩写",
    group: "Shell",
    source: "suggested",
  },
  {
    command: "config init",
    description: "生成配置模板",
    group: "Shell",
    source: "suggested",
  },
  {
    command: "help",
    description: "查看全部快捷键与内建命令",
    group: "Shell",
    source: "suggested",
  },
];

export const initialHistory = [
  { command: "git status", time: "2 分钟前", shell: "cmds", exit: 0, count: 12 },
  {
    command: "cargo build --release --locked",
    time: "6 分钟前",
    shell: "cmds",
    exit: 0,
    count: 8,
  },
  {
    command: 'git commit -m "fix: 补全越界"',
    time: "18 分钟前",
    shell: "cmds",
    exit: 0,
    count: 12,
  },
  {
    command: "git checkout -b feature/menu",
    time: "25 分钟前",
    shell: "cmds",
    exit: 0,
    count: 5,
  },
  {
    command: "npm run dev",
    time: "40 分钟前",
    shell: "cmds",
    exit: 0,
    count: 6,
  },
  { command: "ls -lah", time: "52 分钟前", shell: "cmds", exit: 0, count: 21 },
];

/* ─────────────────────────── 通用算法 ─────────────────────────── */

/** 与 cli/src/util.rs 的 levenshtein 同算法。 */
export function levenshtein(a, b) {
  const left = [...a];
  const right = [...b];
  if (!left.length) return right.length;
  if (!right.length) return left.length;
  let previous = [0, ...right.map((_, index) => index + 1)];
  for (let i = 0; i < left.length; i += 1) {
    const current = [i + 1];
    for (let j = 0; j < right.length; j += 1) {
      current[j + 1] = Math.min(
        previous[j] + (left[i] === right[j] ? 0 : 1),
        previous[j + 1] + 1,
        current[j] + 1,
      );
    }
    previous = current;
  }
  return previous[right.length];
}

const KIND_BUILTIN = 0;
const KIND_USER = 1;
const KIND_PATH = 2;

/**
 * 拼错命令时的建议，规则与 cli/src/shell.rs 的 similar_commands 一致：
 * 编辑距离 → 类型（内建 > 别名缩写 > PATH）→ 首字母是否相同 → 长度差 → 字典序。
 */
export function similarCommands(name, limit = 3, extras = {}) {
  const typed = (name ?? "").toLowerCase();
  if (!typed) return [];
  const typedLength = [...typed].length;
  const threshold = typedLength <= 3 ? 1 : 2;
  const first = [...typed][0];
  const pool = [
    ...builtinNames.map((value) => [value, KIND_BUILTIN]),
    ...Object.keys(extras.aliases ?? defaultAliases).map((value) => [
      value,
      KIND_USER,
    ]),
    ...Object.keys(extras.abbreviations ?? defaultAbbreviations).map(
      (value) => [value, KIND_USER],
    ),
    ...pathCommands.map((value) => [value, KIND_PATH]),
  ];
  return pool
    .map(([candidate, kind]) => {
      const folded = candidate.toLowerCase();
      return {
        candidate,
        kind,
        folded,
        distance: levenshtein(typed, folded),
        differingFirst: [...folded][0] !== first,
        lengthGap: Math.abs(typedLength - [...folded].length),
      };
    })
    .filter((item) => item.folded !== typed && item.distance <= threshold)
    .sort(
      (a, b) =>
        a.distance - b.distance ||
        a.kind - b.kind ||
        Number(a.differingFirst) - Number(b.differingFirst) ||
        a.lengthGap - b.lengthGap ||
        a.candidate.localeCompare(b.candidate),
    )
    .slice(0, limit)
    .map((item) => item.candidate);
}

/**
 * Tab 候选菜单。历史整条命令排最前，其后是内建 / 别名 / 缩写 / PATH 命令。
 * `settings.history` / `settings.recommendations` 可分别关闭两路来源。
 */
export function getSuggestions(query, history = [], settings = {}) {
  const needle = query.trim().toLowerCase();
  const catalog = new Map(commandCatalog.map((item) => [item.command, item]));

  const fromHistory =
    settings.history === false
      ? []
      : history.map((item, index) => ({
          command: item.command,
          description:
            catalog.get(item.command)?.description ?? "来自你的历史记录",
          source: "history",
          // 频率 + 新鲜度：与 CLI 的历史排序同思路
          weight: (item.count ?? 1) * 100 - index,
        }));

  const fromCatalog =
    settings.recommendations === false
      ? []
      : commandCatalog.map((item) => ({
          ...item,
          source: "suggested",
          weight: 0,
        }));

  const merged = new Map();
  for (const item of [...fromHistory, ...fromCatalog]) {
    // 同一条命令同时出现在历史和推荐里时，保留历史身份与权重
    if (!merged.has(item.command) || item.source === "history") {
      merged.set(item.command, { ...item });
    }
  }

  return [...merged.values()]
    .filter((item) => !needle || item.command.toLowerCase().includes(needle))
    .sort((a, b) => {
      const aPrefix = a.command.toLowerCase().startsWith(needle);
      const bPrefix = b.command.toLowerCase().startsWith(needle);
      return (
        Number(bPrefix) - Number(aPrefix) ||
        Number(b.source === "history") - Number(a.source === "history") ||
        b.weight - a.weight ||
        a.command.localeCompare(b.command)
      );
    })
    .slice(0, 5);
}

/** 灰色的历史 ghost 建议：按前缀取权重最高的一条，返回待补上的尾巴。 */
export function ghostSuggestion(input, history = []) {
  if (!input || input.startsWith(" ")) return "";
  const lowered = input.toLowerCase();
  const matched = getSuggestions(input, history, { recommendations: false })
    .map((item) => item.command)
    .filter(
      (command) =>
        command.toLowerCase().startsWith(lowered) &&
        command.length > input.length,
    );
  return matched.length ? matched[0].slice(input.length) : "";
}

/** 语法高亮用：命令名是否存在。 */
export function isKnownCommand(name, extras = {}) {
  if (!name) return false;
  return (
    builtinNames.includes(name) ||
    pathCommands.includes(name) ||
    Object.prototype.hasOwnProperty.call(
      extras.aliases ?? defaultAliases,
      name,
    ) ||
    Object.prototype.hasOwnProperty.call(
      extras.abbreviations ?? defaultAbbreviations,
      name,
    )
  );
}

/* ─────────────────────── 配置模板生成 ─────────────────────── */

export const promptModules = [
  ["$dir", "当前目录"],
  ["$git_branch", "Git 分支"],
  ["$git_status", "Git 状态"],
  ["$languages", "语言与版本"],
  ["$cmd_duration", "上条命令耗时"],
  ["$status", "上条命令退出码"],
  ["$character", "提示符号 ❯"],
];

const MODULE_ORDER = promptModules.map(([id]) => id);

/**
 * 依据网站上的选择生成 `~/.config/cmds/config.toml`，可直接复制到本地使用。
 */
export function buildConfigToml(options = {}) {
  const {
    modules = MODULE_ORDER,
    addNewline = true,
    lineBreak = true,
    autosuggest = true,
    gitStatus = true,
    menuRows = 8,
    aliases = defaultAliases,
    abbreviations = defaultAbbreviations,
  } = options;

  const ordered = MODULE_ORDER.filter((module) => modules.includes(module));
  const hasCharacter = ordered.includes("$character");
  const body = ordered.filter((module) => module !== "$character");
  const format =
    body.join("") +
    (hasCharacter ? `${lineBreak ? "$line_break" : ""}$character` : "");

  const lines = [
    `# ${BRAND.name} (${BRAND.bin}) 配置 · 由官网生成`,
    `# 保存到 ~/.config/${BRAND.bin}/config.toml，或用 \`${BRAND.bin} config init\` 生成默认模板`,
    "",
    `format = "${format || "$character"}"`,
    `add_newline = ${addNewline}`,
    "",
    "[autosuggest]",
    `enabled = ${autosuggest}`,
    'sources = ["history", "completion"]',
    "",
    "[menu]",
    `max_rows = ${menuRows}`,
    'selected_style = "bold fg:black bg:cyan"',
    "",
    "[git]",
    gitStatus
      ? "status_enabled = true"
      : "status_enabled = false   # 超大仓库关掉可以明显提速",
  ];

  const aliasEntries = Object.entries(aliases);
  if (aliasEntries.length) {
    lines.push("", "[aliases]");
    for (const [key, value] of aliasEntries) {
      lines.push(`${key} = "${value.replace(/"/g, '\\"')}"`);
    }
  }

  const abbrEntries = Object.entries(abbreviations);
  if (abbrEntries.length) {
    lines.push("", "[abbreviations]");
    for (const [key, value] of abbrEntries) {
      lines.push(`${key} = "${value.replace(/"/g, '\\"')}"`);
    }
  }

  return `${lines.join("\n")}\n`;
}

/* ─────────────────────────── 模拟执行 ─────────────────────────── */

const SIMULATED = "（Playground 模拟输出，本机没有任何命令被真正执行）";
const GUIDE_URL = "https://junhey.github.io/commands/#guide";

function helpOutput() {
  return [
    `${BRAND.name} (${BRAND.bin}) v${BRAND.version} — 轻量高效的交互式终端`,
    "",
    "快捷键",
    "  Tab / Shift-Tab      打开候选菜单 / 菜单内上一项",
    "  ↑ ↓ (Ctrl-P/Ctrl-N)  菜单内移动；菜单关闭时按前缀翻历史",
    "  Enter                菜单打开时采纳候选，否则执行",
    "  → / End / Ctrl-F     采纳整条灰色历史建议",
    "  Alt-→                只采纳建议里的一个词",
    "  Ctrl-R               模糊搜索历史",
    "  Ctrl-A/E/W/U/K/L     行首 / 行尾 / 删词 / 删到行首 / 删到行尾 / 清屏",
    "  Ctrl-C / Ctrl-D      放弃当前输入 / 空行退出",
    "",
    "内建命令",
    ...builtins.map(
      ([name, description]) => `  ${name.padEnd(9)} ${description}`,
    ),
    "",
    `文档：${GUIDE_URL}`,
  ];
}

function splitWords(text) {
  return text.split(/\s+/).filter(Boolean);
}

/**
 * 模拟执行一条命令。返回 { output, exit, cwd?, branch?, clear? }。
 * 一切都是查表和字符串处理，不接触真实环境。
 */
export function simulateCommand(
  command,
  cwd = "~/projects/commands",
  state = {},
) {
  const text = command.trim();
  if (!text) return { output: [], exit: 0 };

  const words = splitWords(text);
  const head = words[0];
  const rest = text.slice(head.length).trim();
  const aliases = state.aliases ?? defaultAliases;
  const abbreviations = state.abbreviations ?? defaultAbbreviations;

  if (head === "clear") return { clear: true, output: [], exit: 0 };
  if (head === "exit") return { output: ["exit"], exit: 0 };
  if (head === "pwd") return { output: [cwd.replace("~", "/home/you")], exit: 0 };
  if (head === "true") return { output: [], exit: 0 };
  if (head === "false") return { output: [], exit: 1 };
  if (head === "help") return { output: helpOutput(), exit: 0 };

  if (head === "echo") {
    const flagless = rest.replace(/^-[ne]+\s+/, "");
    return {
      output: [flagless.replace(/^(["'])([\s\S]*)\1$/, "$2")],
      exit: 0,
    };
  }

  if (head === "cd") {
    const destination = rest || "~";
    if (/[;&|<>`$()]/.test(destination)) {
      return { output: ["Playground: 只支持简单的目录路径。"], exit: 1 };
    }
    if (destination === "-") {
      return { output: [], cwd: state.previousCwd ?? cwd, exit: 0 };
    }
    if (destination.startsWith("~") || destination.startsWith("/")) {
      return { output: [], cwd: destination, exit: 0 };
    }
    if (destination === "..") {
      return {
        output: [],
        cwd: cwd.split("/").slice(0, -1).join("/") || "/",
        exit: 0,
      };
    }
    return { output: [], cwd: `${cwd}/${destination}`, exit: 0 };
  }

  if (head === "alias") {
    if (!rest) {
      return {
        output: Object.entries(aliases).map(
          ([key, value]) => `alias ${key}='${value}'`,
        ),
        exit: 0,
      };
    }
    return { output: [`已记录别名：${rest}`, SIMULATED], exit: 0 };
  }

  if (head === "abbr") {
    if (!rest) {
      return {
        output: Object.entries(abbreviations).map(
          ([key, value]) => `abbr ${key} '${value}'`,
        ),
        exit: 0,
      };
    }
    return {
      output: [
        `已记录缩写：${rest}`,
        "输入缩写后按空格即展开，历史里保存的是展开后的完整命令。",
        SIMULATED,
      ],
      exit: 0,
    };
  }

  if (head === "type" || head === "which") {
    if (!rest) return { output: [`${head}: 需要一个命令名`], exit: 2 };
    if (builtinNames.includes(rest)) {
      return { output: [`${rest} 是内建命令`], exit: 0 };
    }
    if (Object.prototype.hasOwnProperty.call(aliases, rest)) {
      return { output: [`${rest} 是别名，展开为 ${aliases[rest]}`], exit: 0 };
    }
    if (Object.prototype.hasOwnProperty.call(abbreviations, rest)) {
      return {
        output: [`${rest} 是缩写，展开为 ${abbreviations[rest]}`],
        exit: 0,
      };
    }
    if (pathCommands.includes(rest)) {
      return { output: [`${rest} 位于 /usr/bin/${rest}`], exit: 0 };
    }
    return { output: [`${rest}：未找到`], exit: 1 };
  }

  if (head === "history") {
    const entries = (state.history ?? initialHistory).slice(0, 10);
    return {
      output: entries.length
        ? entries.map(
            (item, index) => `${String(index + 1).padStart(4)}  ${item.command}`,
          )
        : ["（历史为空）"],
      exit: 0,
    };
  }

  if (head === "jobs") return { output: ["（没有后台任务）"], exit: 0 };

  if (head === "config") {
    const sub = splitWords(rest)[0] ?? "show";
    if (sub === "path") {
      return {
        output: [
          "配置文件：/home/you/.config/cmds/config.toml",
          "历史文件：/home/you/.local/share/cmds/history",
        ],
        exit: 0,
      };
    }
    if (sub === "init") {
      return {
        output: [
          "已写出配置模板：/home/you/.config/cmds/config.toml",
          "在网站的 Config 页可以可视化生成同一份文件。",
          SIMULATED,
        ],
        exit: 0,
      };
    }
    if (sub === "reload") {
      return { output: ["配置已重新加载。", SIMULATED], exit: 0 };
    }
    return { output: buildConfigToml().trimEnd().split("\n"), exit: 0 };
  }

  if (head === "export" || head === "set" || head === "unset") {
    return { output: [`${head} ${rest}`.trim(), SIMULATED], exit: 0 };
  }

  if (head === "git") {
    const sub = words[1];
    if (sub === "status") {
      return {
        output: [
          "On branch master",
          "Your branch is up to date with 'origin/master'.",
          "",
          "nothing to commit, working tree clean",
        ],
        exit: 0,
      };
    }
    if (sub === "diff") {
      return { output: ["工作区干净，没有未暂存的改动。"], exit: 0 };
    }
    if (sub === "log") {
      return {
        output: [
          "a31d48f (HEAD → master) feat: 候选菜单支持 Shift-Tab",
          "80b2e19 perf: 历史按频率 + 新鲜度排序",
          "e9255ab feat: 模块化提示符",
          "72ac3b1 docs: 补上配置说明",
          "1aaeb0d init: hello, commands",
        ],
        exit: 0,
      };
    }
    if (sub === "checkout" && words[2] === "-b" && words[3]) {
      return {
        output: [`Switched to a new branch '${words[3]}'`],
        branch: words[3],
        exit: 0,
      };
    }
    return {
      output: [`Playground: git ${sub ?? ""} 不会改动任何真实仓库。`, SIMULATED],
      exit: 0,
    };
  }

  if (head === "ls" || head === "ll") {
    return {
      output: [
        "drwxr-xr-x   cli/",
        "drwxr-xr-x   web/",
        "drwxr-xr-x   install/",
        "drwxr-xr-x   docs/",
        "-rw-r--r--   Cargo.toml",
        "-rw-r--r--   README.md",
      ],
      exit: 0,
    };
  }

  if (head === "cargo") {
    if (words[1] === "test") {
      return {
        output: [
          "   Compiling cmds v0.1.0",
          "    Finished `test` profile in 0.31s",
          "     Running unittests src/main.rs",
          "",
          "test result: ok. 99 passed; 0 failed; 0 ignored",
          SIMULATED,
        ],
        exit: 0,
      };
    }
    return {
      output: [
        "   Compiling cmds v0.1.0",
        "    Finished `release` profile [optimized] target(s) in 24.71s",
        SIMULATED,
      ],
      exit: 0,
    };
  }

  if (head === "npm" || head === "pnpm") {
    if (words.includes("dev")) {
      return {
        output: [
          "> commands-web@0.1.0 dev",
          "> vite",
          "",
          "  VITE v6.1.0  ready in 148 ms",
          "  ➜  Local:   http://localhost:5173/",
          "Playground 预览：no server process was started.",
        ],
        exit: 0,
      };
    }
    if (words.includes("build")) {
      return {
        output: [
          "✓ 32 modules transformed.",
          "构建演示完成，没有写入任何文件。",
        ],
        exit: 0,
      };
    }
    return { output: ["依赖已是最新，本机没有安装任何包。"], exit: 0 };
  }

  const suggestions = similarCommands(head, 3, { aliases, abbreviations });
  return {
    output: [
      suggestions.length
        ? `cmds: ${head}：未找到命令，也许你想输入：${suggestions.join("、")}`
        : `cmds: ${head}：未找到命令`,
      "输入 help 查看全部内建命令，或安装 cmds 在本机执行真实命令。",
    ],
    exit: 127,
  };
}

/* ─────────────────────────── 本地存储 ─────────────────────────── */

export function loadStored(key, fallback) {
  try {
    const value = JSON.parse(localStorage.getItem(key));
    return value ?? fallback;
  } catch {
    return fallback;
  }
}

export function saveStored(key, value) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
    return true;
  } catch {
    // 隐私模式下 localStorage 可能不可写，当次会话仍然可用
    return false;
  }
}
