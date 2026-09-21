import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowDownLeft,
  ArrowRight,
  ArrowUp,
  BookOpen,
  Check,
  ChevronDown,
  ChevronRight,
  ChevronsRight,
  CircleHelp,
  Clock3,
  Code2,
  Copy,
  Download,
  ExternalLink,
  Fish,
  GitBranch,
  History,
  Keyboard,
  Laptop,
  Leaf,
  Maximize2,
  Menu,
  Minus,
  MoreHorizontal,
  Palette,
  PanelLeftClose,
  Search,
  Settings2,
  ShieldCheck,
  SlidersHorizontal,
  Sparkles,
  Star,
  Terminal,
  Trash2,
  X,
  Zap,
} from "lucide-react";
import {
  BRAND,
  buildConfigToml,
  commandCatalog,
  defaultAbbreviations,
  defaultAliases,
  getSuggestions,
  ghostSuggestion,
  initialHistory,
  isKnownCommand,
  loadStored,
  promptModules,
  saveStored,
  simulateCommand,
} from "./commands.js";
import { installMethods, nextSteps } from "./install.js";

const STORAGE = {
  history: "cmds-history",
  settings: "cmds-settings",
  snippets: "cmds-snippets",
  config: "cmds-config",
};

const navigation = [
  { name: "Playground", icon: Terminal, section: "workspace" },
  { name: "History", icon: History, section: "workspace" },
  { name: "Snippets", icon: Code2, section: "workspace" },
  { name: "Config", icon: SlidersHorizontal, section: "configuration" },
  { name: "Appearance", icon: Palette, section: "configuration" },
  { name: "Integrations", icon: Settings2, section: "configuration" },
];

const defaultSettings = {
  auto: true,
  history: true,
  recommendations: true,
  theme: "commands",
  fontSize: 13,
  prompt: true,
};

const defaultConfig = {
  modules: promptModules.map(([id]) => id),
  addNewline: true,
  lineBreak: true,
  autosuggest: true,
  gitStatus: true,
  menuRows: 8,
};

const startingSnippets = [
  {
    id: "status",
    title: "看一眼工作区",
    command: "git status",
    description: "改了什么，一目了然。",
    group: "Git",
  },
  {
    id: "log",
    title: "最近五条提交",
    command: "git log --oneline -5",
    description: "快速回忆刚才在干什么。",
    group: "Git",
  },
  {
    id: "release",
    title: "构建发布版",
    command: "cargo build --release --locked",
    description: "锁定依赖，产出优化过的二进制。",
    group: "Rust",
  },
];

const titles = {
  Playground: [
    "少敲一点，多做一点。",
    "边打字边给历史建议，Tab 弹候选菜单用 ↑ ↓ 选。先在浏览器里试试手感。",
  ],
  History: [
    "用过的命令，值得被记住。",
    "按使用频率和新鲜度排序，找回那条命令，回到你刚才在做的事。",
  ],
  Snippets: ["把常用命令收在手边。", "顺手的命令存成片段，下次一键取用。"],
  Config: ["一份配置，随手生成。", "勾选提示符模块，直接拿到可用的 config.toml。"],
  Appearance: ["调成你喜欢的样子。", "更安静的配色，更合适的字号，你自己的终端。"],
  Integrations: [
    "和现有工具好好相处。",
    "接到 bash / zsh / fish / PowerShell，或者只用它的提示符。",
  ],
  Guide: ["从安装到顺手，十分钟。", "安装、快捷键、配置、集成方式，都在这里。"],
};

const SITE_ORIGIN =
  typeof window === "undefined"
    ? "http://localhost:5173"
    : window.location.origin;
const SITE_BASE = import.meta.env.BASE_URL ?? "/";

const integrationSnippets = [
  ["bash", 'eval "$(cmds init bash)"', "~/.bashrc"],
  ["zsh", 'eval "$(cmds init zsh)"', "~/.zshrc"],
  ["fish", "cmds init fish | source", "~/.config/fish/config.fish"],
  [
    "PowerShell",
    "Invoke-Expression (& cmds init powershell | Out-String)",
    "$PROFILE",
  ],
];

/* ───────────────────────────── 基础组件 ───────────────────────────── */

function Logo({ small = false }) {
  return (
    <span className={`logo-symbol ${small ? "small" : ""}`}>
      <ChevronsRight strokeWidth={3} />
    </span>
  );
}

/**
 * GitHub 图标。
 *
 * lucide-react v1 移除了全部品牌 logo（商标原因），没有内置替代，
 * 这里内联官方 Octicon 的 mark-github 路径（MIT）。
 */
function GithubMark({ size = 18 }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="currentColor"
      aria-hidden="true"
      focusable="false"
    >
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z" />
    </svg>
  );
}

function Key({ children }) {
  return <kbd>{children}</kbd>;
}

function Toggle({ checked, onChange, label }) {
  return (
    <button
      type="button"
      className={`toggle ${checked ? "on" : ""}`}
      role="switch"
      aria-checked={checked}
      aria-label={label}
      onClick={() => onChange(!checked)}
    >
      <span />
    </button>
  );
}

function CopyButton({ text, label, onCopy }) {
  const [copied, setCopied] = useState(false);
  const timer = useRef();
  useEffect(() => () => clearTimeout(timer.current), []);
  async function copy() {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      onCopy?.();
      clearTimeout(timer.current);
      timer.current = setTimeout(() => setCopied(false), 1800);
    } catch {
      onCopy?.("剪贴板不可用，请手动选中命令复制。");
    }
  }
  return (
    <button
      className={`copy-button ${label ? "with-label" : ""}`}
      onClick={copy}
      aria-label={copied ? "已复制" : "复制命令"}
    >
      {copied ? <Check size={15} /> : <Copy size={15} />}
      {label && (copied ? "已复制" : label)}
    </button>
  );
}

function Prompt({ cwd, branch, showPrompt = true }) {
  return (
    <div className="shell-prompt">
      <span className="prompt-folder">{cwd}</span>
      {showPrompt && (
        <>
          <span className="muted">on</span>
          <span className="prompt-branch">
            <GitBranch size={13} />
            {branch}
          </span>
          <span className="muted">via</span>
          <span className="prompt-version">🦀 v1.95.0</span>
        </>
      )}
    </div>
  );
}

function Modal({ title, subtitle, onClose, children, wide = false }) {
  const dialogRef = useRef();
  useEffect(() => {
    const previous = document.activeElement;
    const dialog = dialogRef.current;
    dialog.showModal();
    return () => {
      dialog.close();
      previous?.focus();
    };
  }, []);
  return (
    <dialog
      ref={dialogRef}
      aria-label={title}
      className={`modal ${wide ? "wide" : ""}`}
      onCancel={onClose}
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <header className="modal-heading">
        <div>
          <h2>{title}</h2>
          {subtitle && <p>{subtitle}</p>}
        </div>
        <button
          className="icon-button"
          aria-label="关闭对话框"
          onClick={onClose}
        >
          <X size={20} />
        </button>
      </header>
      {children}
    </dialog>
  );
}

/** 按 cmds 的高亮规则给命令行上色：命令存在绿色、拼错红色。 */
function HighlightedCommand({ text }) {
  const [head, ...rest] = text.split(" ");
  if (!head) return <span>{text}</span>;
  return (
    <span>
      <span className={isKnownCommand(head) ? "token-command" : "token-unknown"}>
        {head}
      </span>
      {rest.length ? ` ${rest.join(" ")}` : ""}
    </span>
  );
}

/* ──────────────────────────── Playground ──────────────────────────── */

function TerminalPanel({
  session,
  setSession,
  history,
  addHistory,
  settings,
  queuedCommand,
  consumeCommand,
  notify,
}) {
  const [sessions, setSessions] = useState(["cmds", "cmds 2"]);
  const [input, setInput] = useState("git ");
  const [isOpen, setIsOpen] = useState(true);
  const [selected, setSelected] = useState(0);
  const [cwd, setCwd] = useState("~/projects/commands");
  const [previousCwd, setPreviousCwd] = useState("~");
  const [branch, setBranch] = useState("master");
  const [blocks, setBlocks] = useState([]);
  const [intro, setIntro] = useState(true);
  const [expanded, setExpanded] = useState(false);
  const [menu, setMenu] = useState(false);
  const [hint, setHint] = useState("输入命令试试，Tab 弹出候选菜单");
  const inputRef = useRef();
  const scrollRef = useRef();
  const sessionCache = useRef(new Map());
  const latest = useRef({});

  latest.current = { input, cwd, previousCwd, branch, blocks, intro };

  // 切换标签页时把当前标签的状态存起来，再恢复目标标签的状态
  useEffect(() => {
    const cache = sessionCache.current;
    const saved = cache.get(session);
    if (saved) {
      setInput(saved.input);
      setCwd(saved.cwd);
      setPreviousCwd(saved.previousCwd);
      setBranch(saved.branch);
      setBlocks(saved.blocks);
      setIntro(saved.intro);
      setIsOpen(false);
      setSelected(0);
    }
    return () => {
      cache.set(session, latest.current);
    };
  }, [session]);

  const suggestions = useMemo(
    () => getSuggestions(input, history, settings).slice(0, 5),
    [input, history, settings],
  );
  const showing = isOpen && suggestions.length > 0;
  const ghost = useMemo(
    () => (showing || !settings.history ? "" : ghostSuggestion(input, history)),
    [showing, settings.history, input, history],
  );

  useEffect(() => {
    setSelected((current) =>
      Math.min(current, Math.max(0, suggestions.length - 1)),
    );
  }, [suggestions]);

  useEffect(() => {
    if (queuedCommand !== null) {
      setInput(queuedCommand);
      setIsOpen(false);
      inputRef.current?.focus();
      consumeCommand();
    }
  }, [queuedCommand, consumeCommand]);

  useEffect(() => {
    if (!settings.auto) setIsOpen(false);
  }, [settings.auto]);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = blocks.length
        ? scrollRef.current.scrollHeight
        : 0;
    }
  }, [blocks, intro]);

  useEffect(() => {
    const handle = (event) => {
      if (
        (event.metaKey || event.ctrlKey) &&
        event.key.toLowerCase() === "l" &&
        document.activeElement === inputRef.current
      ) {
        event.preventDefault();
        setBlocks([]);
        setIntro(false);
      }
      if (event.key === "Escape") {
        setExpanded(false);
        setMenu(false);
      }
    };
    window.addEventListener("keydown", handle);
    return () => window.removeEventListener("keydown", handle);
  }, []);

  function accept() {
    if (!suggestions[selected]) return;
    setInput(suggestions[selected].command);
    setIsOpen(false);
    setHint("采纳候选不会执行，再按一次 Enter 才运行");
    inputRef.current?.focus();
  }

  function run() {
    if (!input.trim()) return;
    const result = simulateCommand(input, cwd, {
      history,
      previousCwd,
      aliases: defaultAliases,
      abbreviations: defaultAbbreviations,
    });
    if (result.clear) {
      setBlocks([]);
      setIntro(false);
    } else {
      setBlocks((items) => [
        ...items.slice(-49),
        {
          command: input,
          cwd,
          branch,
          output: result.output,
          exit: result.exit,
        },
      ]);
    }
    if (result.cwd && result.cwd !== cwd) {
      setPreviousCwd(cwd);
      setCwd(result.cwd);
    }
    if (result.branch) setBranch(result.branch);
    // 以空格开头的命令不进历史，与 cmds 的行为一致
    if (!input.startsWith(" ")) {
      addHistory({
        command: input.trim(),
        shell: session,
        exit: result.exit,
        time: new Date().toISOString(),
      });
    }
    setInput("");
    setIsOpen(false);
    setSelected(0);
    setHint(
      result.exit
        ? "输入 help 看看有哪些内建命令"
        : `上一条命令退出码 ${result.exit}`,
    );
  }

  function handleKey(event) {
    if (event.nativeEvent.isComposing) return;
    const atLineEnd = event.currentTarget.selectionStart === input.length;

    if (event.key === "Tab") {
      event.preventDefault();
      if (!showing) {
        setIsOpen(true);
        setSelected(0);
      } else if (event.shiftKey) {
        setSelected(
          (current) => (current - 1 + suggestions.length) % suggestions.length,
        );
      } else {
        accept();
      }
      return;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      if (!showing) {
        setIsOpen(true);
        setSelected(0);
      } else {
        setSelected(
          (current) =>
            (current +
              (event.key === "ArrowDown" ? 1 : -1) +
              suggestions.length) %
            suggestions.length,
        );
      }
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      if (showing) accept();
      else run();
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      setIsOpen(false);
      return;
    }
    // → / End 采纳整条灰色历史建议
    if (
      (event.key === "ArrowRight" || event.key === "End") &&
      ghost &&
      atLineEnd
    ) {
      event.preventDefault();
      setInput(input + ghost);
      setHint("已采纳历史建议，按 Enter 运行");
      return;
    }
    if (event.key === "ArrowRight" && showing && atLineEnd) {
      event.preventDefault();
      accept();
      return;
    }
    if (event.ctrlKey && event.key === "c") {
      event.preventDefault();
      setInput("");
      setIsOpen(false);
      setHint("已放弃当前输入");
    }
  }

  function newSession() {
    if (sessions.length >= 5) {
      notify("最多同时开五个 Playground 标签。");
      return;
    }
    const name = `cmds ${sessions.length + 1}`;
    setSessions([...sessions, name]);
    setSession(name);
    inputRef.current?.focus();
  }

  return (
    <section
      className={`terminal-window theme-${settings.theme} ${expanded ? "expanded" : ""}`}
      style={{ "--terminal-font": `${settings.fontSize}px` }}
      aria-label="交互式终端 Playground"
    >
      <div className="terminal-tabs">
        <div className="window-controls">
          <i />
          <i />
          <i />
        </div>
        <div className="shell-tabs">
          {sessions.map((name) => (
            <button
              key={name}
              className={`shell-tab ${session === name ? "active" : ""}`}
              onClick={() => setSession(name)}
            >
              <Terminal size={14} />
              {name}
              <span className="session-dot" />
            </button>
          ))}
          <button
            className="terminal-icon add-tab"
            aria-label="新建标签"
            onClick={newSession}
          >
            <span aria-hidden="true">+</span>
          </button>
        </div>
        <div className="terminal-tools">
          <button
            className="terminal-icon"
            aria-label={expanded ? "还原" : "展开"}
            onClick={() => setExpanded(!expanded)}
          >
            {expanded ? <Minus size={16} /> : <Maximize2 size={14} />}
          </button>
          <button
            className="terminal-icon"
            aria-label="更多操作"
            onClick={() => setMenu(!menu)}
          >
            <MoreHorizontal size={19} />
          </button>
          {menu && (
            <div className="terminal-menu">
              <button
                onClick={() => {
                  setBlocks([]);
                  setIntro(false);
                  setMenu(false);
                }}
              >
                清屏 <Key>⌃ L</Key>
              </button>
              <button
                onClick={() => {
                  setInput("help");
                  setIsOpen(false);
                  setMenu(false);
                  inputRef.current?.focus();
                }}
              >
                查看 help <CircleHelp size={14} />
              </button>
            </div>
          )}
        </div>
      </div>
      <div className="terminal-content" ref={scrollRef}>
        {intro && (
          <div className="terminal-intro">
            <div className="welcome-line">
              <Logo small />
              <span>
                <strong>{BRAND.name}</strong>
                <span className="welcome-dot">.</span>
              </span>
              <span className="version-label">v{BRAND.version}</span>
            </div>
            <p>输入 help 查看快捷键 · Tab 弹出候选 · config init 生成配置模板</p>
            <div className="previous-command">
              <Prompt
                cwd="~/projects/commands"
                branch="master"
                showPrompt={settings.prompt}
              />
              <div className="command-line">
                <span className="prompt-chevron">❯</span>
                <span>
                  <span className="token-command">cargo</span> bu
                  <span className="ghost-text">ild --release --locked</span>
                </span>
              </div>
              <div className="intro-hint">
                灰色部分是历史建议，按 <Key>→</Key> 采纳整条
              </div>
            </div>
          </div>
        )}
        {blocks.map((block, index) => (
          <div className="command-block" key={`${block.command}-${index}`}>
            <Prompt
              cwd={block.cwd}
              branch={block.branch}
              showPrompt={settings.prompt}
            />
            <div className="command-line">
              <span className={block.exit ? "prompt-error" : "prompt-chevron"}>
                ❯
              </span>
              <HighlightedCommand text={block.command} />
            </div>
            {block.output.length > 0 && (
              <pre className={block.exit ? "error-output" : ""}>
                {block.output.join("\n")}
              </pre>
            )}
          </div>
        ))}
        <div className="active-command">
          <Prompt cwd={cwd} branch={branch} showPrompt={settings.prompt} />
          <div className="command-line input-line">
            <span className="prompt-chevron">❯</span>
            <div className="input-wrap">
              <input
                ref={inputRef}
                value={input}
                onChange={(event) => {
                  setInput(event.target.value);
                  setSelected(0);
                  setIsOpen(settings.auto && event.target.value.length > 0);
                }}
                onKeyDown={handleKey}
                spellCheck="false"
                autoComplete="off"
                autoCapitalize="off"
                aria-label="终端命令"
                role="combobox"
                aria-expanded={showing}
                aria-controls="command-suggestions"
                aria-activedescendant={
                  showing ? `suggestion-${selected}` : undefined
                }
                placeholder="输入命令…"
              />
              <span className="input-ghost" aria-hidden="true">
                <span>{input}</span>
                <span className="ghost-text">
                  {showing && suggestions[selected]?.command.startsWith(input)
                    ? suggestions[selected].command.slice(input.length)
                    : ghost}
                </span>
              </span>
            </div>
            <span className="input-key">
              <Key>tab</Key> 补全
            </span>
          </div>
          {showing && (
            <div className="suggestions">
              <div className="suggestion-heading">
                <span>
                  <Sparkles size={12} /> 候选菜单
                </span>
                <span>
                  {selected + 1}/{suggestions.length}
                </span>
              </div>
              <ul id="command-suggestions" role="listbox" aria-label="命令候选">
                {suggestions.map((item, index) => (
                  <li
                    id={`suggestion-${index}`}
                    key={item.command}
                    role="option"
                    aria-selected={index === selected}
                    className={index === selected ? "selected" : ""}
                    onMouseEnter={() => setSelected(index)}
                    onMouseDown={(event) => event.preventDefault()}
                    onClick={() => {
                      setInput(item.command);
                      setIsOpen(false);
                      inputRef.current?.focus();
                    }}
                  >
                    {item.source === "history" ? (
                      <Clock3 size={14} />
                    ) : (
                      <Sparkles size={14} />
                    )}
                    <span className="suggestion-command">{item.command}</span>
                    <span className={`source-label ${item.source}`}>
                      {item.source === "history" ? "历史" : "推荐"}
                    </span>
                    {index === selected && <ArrowDownLeft size={13} />}
                  </li>
                ))}
              </ul>
              <div className="suggestions-help">
                <span>
                  <Key>↑</Key>
                  <Key>↓</Key> 选择
                </span>
                <span>
                  <Key>tab</Key> 采纳
                </span>
                <span>
                  <Key>esc</Key> 关闭
                </span>
              </div>
            </div>
          )}
          {!showing && (
            <div className="terminal-hint">
              <Sparkles size={13} /> {hint}
            </div>
          )}
        </div>
      </div>
      <div className="terminal-status">
        <div>
          <span className="status-green" />
          {session}
          <span className="status-divider" />
          <GitBranch size={12} />
          {branch}
        </div>
        <div>
          <span className="playground-tag">模拟环境</span>
          <span className="status-divider" />
          UTF-8
          <span className="status-divider" />
          Ln 1, Col {input.length + 1}
        </div>
      </div>
    </section>
  );
}

/* ───────────────────────────── 安装对话框 ───────────────────────────── */

function InstallContent({ notify, platform, setPlatform }) {
  const methods = useMemo(() => installMethods(SITE_ORIGIN, SITE_BASE), []);
  const [methodId, setMethodId] = useState("script");
  const list = methods[platform];
  const method = list.find((item) => item.id === methodId) ?? list[0];

  return (
    <div className="install-content">
      <div className="platform-tabs">
        {["macOS", "Linux", "Windows"].map((name) => (
          <button
            className={platform === name ? "active" : ""}
            onClick={() => setPlatform(name)}
            key={name}
          >
            {name === "Windows" ? <Laptop size={15} /> : <Terminal size={15} />}
            {name}
          </button>
        ))}
      </div>
      <div className="method-tabs">
        {list.map((item) => (
          <button
            key={item.id}
            className={method.id === item.id ? "active" : ""}
            onClick={() => setMethodId(item.id)}
          >
            {item.label}
          </button>
        ))}
      </div>
      <div className="install-code">
        <span>$</span>
        <code>{method.command}</code>
        <CopyButton text={method.command} onCopy={notify} />
      </div>
      <div className="install-note">
        <ShieldCheck size={17} />
        <p>{method.note}</p>
      </div>
      {method.inspect && (
        <a
          className="text-link"
          href={method.inspect}
          target="_blank"
          rel="noreferrer"
        >
          先看一眼脚本内容 <ExternalLink size={13} />
        </a>
      )}
      {nextSteps.map((step, index) => (
        <div className="install-next" key={step.title}>
          <span className="step-number">{index + 1}</span>
          <div>
            <h4>{step.title}</h4>
            <p>{step.body}</p>
            <div className="inline-code">
              <code>{step.command}</code>
              <CopyButton text={step.command} onCopy={notify} />
            </div>
          </div>
        </div>
      ))}
      <div className="notice">
        <BookOpen size={16} />
        <span>
          安装地址取自本站当前域名，不是虚构链接。也可以直接从{" "}
          <a href={methods.releases} target="_blank" rel="noreferrer">
            GitHub Releases
          </a>{" "}
          下载对应平台的压缩包，每个包都附带 SHA-256 校验值。
        </span>
      </div>
    </div>
  );
}

/* ───────────────────────────── History ───────────────────────────── */

function HistoryPage({ history, setHistory, queue, notify }) {
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState("全部会话");
  const [confirm, setConfirm] = useState(false);
  const filtered = history.filter(
    (item) =>
      item.command.toLowerCase().includes(query.toLowerCase()) &&
      (filter === "全部会话" || item.shell === filter),
  );

  function exportHistory() {
    const blob = new Blob([JSON.stringify(history, null, 2)], {
      type: "application/json",
    });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = "cmds-history.json";
    anchor.click();
    URL.revokeObjectURL(url);
    notify("历史已导出为 JSON。");
  }

  return (
    <section className="content-card history-page">
      <div className="section-toolbar">
        <label className="field-search">
          <Search size={17} />
          <input
            aria-label="搜索历史命令"
            placeholder="找一条命令…"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
        </label>
        <select
          aria-label="按会话筛选"
          value={filter}
          onChange={(event) => setFilter(event.target.value)}
        >
          <option>全部会话</option>
          {[...new Set(history.map((item) => item.shell))].map((name) => (
            <option key={name}>{name}</option>
          ))}
        </select>
        <button className="secondary-button" onClick={exportHistory}>
          <Download size={15} />
          导出
        </button>
        <button
          className="icon-button"
          aria-label="清空历史"
          onClick={() => setConfirm(true)}
        >
          <Trash2 size={17} />
        </button>
      </div>
      <div className="table-heading">
        <span>命令</span>
        <span>会话</span>
        <span>最近使用</span>
        <span />
      </div>
      {filtered.map((item, index) => (
        <div className="history-row" key={`${item.command}-${index}`}>
          <div>
            <span className={`exit-dot ${item.exit ? "failed" : ""}`} />
            <code>{item.command}</code>
          </div>
          <span className="pill">{item.shell}</span>
          <span className="history-time">
            {item.time.includes("T")
              ? new Date(item.time).toLocaleTimeString([], {
                  hour: "2-digit",
                  minute: "2-digit",
                })
              : item.time}
          </span>
          <div className="row-actions">
            <CopyButton text={item.command} onCopy={notify} />
            <button
              className="icon-button"
              aria-label={`在 Playground 使用 ${item.command}`}
              onClick={() => queue(item.command)}
            >
              <ArrowUp size={15} />
            </button>
          </div>
        </div>
      ))}
      {!filtered.length && (
        <div className="empty-state">
          <History size={30} />
          <h3>这里还没有命令。</h3>
          <p>{query ? "换个关键词试试。" : "去 Playground 跑几条看看。"}</p>
        </div>
      )}
      <div className="table-footer">
        <ShieldCheck size={14} />
        只存在这个浏览器里，不会上传任何地方。
        <span>{filtered.length} 条</span>
      </div>
      {confirm && (
        <Modal
          title="清空历史？"
          subtitle="会删除这个浏览器里保存的全部历史记录。"
          onClose={() => setConfirm(false)}
        >
          <div className="modal-actions">
            <button
              className="secondary-button"
              onClick={() => setConfirm(false)}
            >
              先留着
            </button>
            <button
              className="danger-button"
              onClick={() => {
                setHistory([]);
                setConfirm(false);
                notify("历史已清空。");
              }}
            >
              确认清空
            </button>
          </div>
        </Modal>
      )}
    </section>
  );
}

/* ───────────────────────────── Snippets ───────────────────────────── */

function SnippetsPage({ snippets, setSnippets, queue, notify }) {
  const [editor, setEditor] = useState(false);
  const [query, setQuery] = useState("");
  const visible = snippets.filter((item) =>
    `${item.title} ${item.command}`.toLowerCase().includes(query.toLowerCase()),
  );

  function save(event) {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    const title = data.get("title").trim();
    const command = data.get("command").trim();
    if (!title || !command) return;
    setSnippets([
      ...snippets,
      {
        id: crypto.randomUUID(),
        title,
        command,
        description: data.get("description").trim(),
        group: "个人",
      },
    ]);
    setEditor(false);
    notify("片段已保存。");
  }

  return (
    <>
      <div className="section-toolbar snippets-toolbar">
        <label className="field-search">
          <Search size={17} />
          <input
            aria-label="搜索片段"
            placeholder="找一个片段…"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
        </label>
        <button className="primary-button" onClick={() => setEditor(true)}>
          新建片段
        </button>
      </div>
      <div className="snippet-grid">
        {visible.map((item) => (
          <article className="snippet-card" key={item.id}>
            <div className="snippet-top">
              <span className="feature-icon">
                <Code2 size={19} />
              </span>
              <span className="pill">{item.group}</span>
            </div>
            <h3>{item.title}</h3>
            <p>{item.description || "你自己的命令片段。"}</p>
            <div className="snippet-command">
              <code>{item.command}</code>
              <CopyButton text={item.command} onCopy={notify} />
            </div>
            <div className="snippet-footer">
              <button className="text-link" onClick={() => queue(item.command)}>
                在 Playground 试 <ArrowRight size={14} />
              </button>
              <button
                className="icon-button"
                aria-label={`删除 ${item.title}`}
                onClick={() => {
                  setSnippets(snippets.filter((entry) => entry.id !== item.id));
                  notify("片段已删除。");
                }}
              >
                <Trash2 size={15} />
              </button>
            </div>
          </article>
        ))}
      </div>
      {!visible.length && (
        <div className="empty-state">
          <Code2 size={30} />
          <h3>还没有片段。</h3>
          <p>新建一个，或者换个关键词搜。</p>
        </div>
      )}
      {editor && (
        <Modal
          title="存下一个常用命令"
          subtitle="把顺手的命令变成随时可取的片段。"
          onClose={() => setEditor(false)}
        >
          <form className="snippet-form" onSubmit={save}>
            <label>
              名称
              <input
                name="title"
                placeholder="构建发布版"
                maxLength={70}
                required
                autoFocus
              />
            </label>
            <label>
              命令
              <textarea
                name="command"
                placeholder="cargo build --release --locked"
                maxLength={500}
                required
                rows={3}
              />
            </label>
            <label>
              说明 <span>（可选）</span>
              <input
                name="description"
                placeholder="提醒自己这条命令做什么"
                maxLength={140}
              />
            </label>
            <div className="modal-actions">
              <button
                type="button"
                className="secondary-button"
                onClick={() => setEditor(false)}
              >
                取消
              </button>
              <button className="primary-button" type="submit">
                保存 <Check size={15} />
              </button>
            </div>
          </form>
        </Modal>
      )}
    </>
  );
}

/* ───────────────────────────── Config ───────────────────────────── */

function ConfigPage({ config, setConfig, notify }) {
  const toml = useMemo(() => buildConfigToml(config), [config]);

  function toggleModule(id) {
    setConfig({
      ...config,
      modules: config.modules.includes(id)
        ? config.modules.filter((item) => item !== id)
        : [...config.modules, id],
    });
  }

  function download() {
    const blob = new Blob([toml], { type: "text/plain;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = "config.toml";
    anchor.click();
    URL.revokeObjectURL(url);
    notify("config.toml 已下载。");
  }

  return (
    <div className="config-page">
      <section className="content-card">
        <div className="section-label">
          <h3>提示符模块</h3>
          <p>勾选要显示的部分，顺序固定，和 CLI 的 format 一致。</p>
        </div>
        <div className="module-grid">
          {promptModules.map(([id, label]) => (
            <button
              key={id}
              className={`module-chip ${config.modules.includes(id) ? "on" : ""}`}
              onClick={() => toggleModule(id)}
              aria-pressed={config.modules.includes(id)}
            >
              <span className="radio-check">
                {config.modules.includes(id) && <Check size={12} />}
              </span>
              <code>{id}</code>
              <small>{label}</small>
            </button>
          ))}
        </div>
        <div className="setting-row">
          <div>
            <h4>提示符前空一行</h4>
            <p>对应 add_newline，长命令之间更好分辨。</p>
          </div>
          <Toggle
            label="提示符前空一行"
            checked={config.addNewline}
            onChange={(value) => setConfig({ ...config, addNewline: value })}
          />
        </div>
        <div className="setting-row">
          <div>
            <h4>输入另起一行</h4>
            <p>对应 $line_break，把 ❯ 放到下一行。</p>
          </div>
          <Toggle
            label="输入另起一行"
            checked={config.lineBreak}
            onChange={(value) => setConfig({ ...config, lineBreak: value })}
          />
        </div>
        <div className="setting-row">
          <div>
            <h4>历史自动建议</h4>
            <p>对应 [autosuggest]，边打字边给灰色建议。</p>
          </div>
          <Toggle
            label="历史自动建议"
            checked={config.autosuggest}
            onChange={(value) => setConfig({ ...config, autosuggest: value })}
          />
        </div>
        <div className="setting-row">
          <div>
            <h4>Git 状态</h4>
            <p>对应 [git] status_enabled，超大仓库关掉能明显提速。</p>
          </div>
          <Toggle
            label="Git 状态"
            checked={config.gitStatus}
            onChange={(value) => setConfig({ ...config, gitStatus: value })}
          />
        </div>
        <div className="setting-row">
          <div>
            <h4>候选菜单行数</h4>
            <p>对应 [menu] max_rows。</p>
          </div>
          <select
            aria-label="候选菜单行数"
            value={config.menuRows}
            onChange={(event) =>
              setConfig({ ...config, menuRows: Number(event.target.value) })
            }
          >
            {[5, 8, 10, 12, 16].map((rows) => (
              <option value={rows} key={rows}>
                {rows} 行
              </option>
            ))}
          </select>
        </div>
        <div className="setting-row">
          <div>
            <h4>恢复默认</h4>
            <p>把上面的选项全部还原。</p>
          </div>
          <button
            className="secondary-button"
            onClick={() => setConfig(defaultConfig)}
          >
            重置
          </button>
        </div>
      </section>
      <section className="content-card config-preview">
        <div className="card-title">
          <h3>~/.config/cmds/config.toml</h3>
          <div className="row-actions">
            <CopyButton text={toml} label="复制" onCopy={notify} />
            <button className="secondary-button" onClick={download}>
              <Download size={15} />
              下载
            </button>
          </div>
        </div>
        <pre className="config-code">{toml}</pre>
        <div className="notice">
          <ShieldCheck size={16} />
          <span>
            保存到 <code>~/.config/cmds/config.toml</code>（Windows：
            <code>%APPDATA%\cmds\config.toml</code>），在 cmds 里执行{" "}
            <code>config reload</code> 即可热重载。
          </span>
        </div>
      </section>
    </div>
  );
}

/* ───────────────────────────── Appearance ───────────────────────────── */

function AppearancePage({ settings, updateSettings }) {
  const themes = [
    { id: "commands", name: "Commands", subtitle: "默认配色，安静克制。" },
    { id: "midnight", name: "Midnight", subtitle: "深夜写代码用。" },
    { id: "sand", name: "Warm sand", subtitle: "暖一点的终端。" },
  ];
  return (
    <div className="appearance-page">
      <div className="section-label">
        <h3>配色</h3>
        <p>Playground 的配色方案，选择会记在这个浏览器里。</p>
      </div>
      <div className="theme-grid">
        {themes.map((theme) => (
          <button
            className={`theme-card ${settings.theme === theme.id ? "chosen" : ""}`}
            key={theme.id}
            onClick={() => updateSettings({ theme: theme.id })}
          >
            <div className={`theme-preview theme-${theme.id}`}>
              <div>
                <i />
                <i />
                <i />
              </div>
              <span>
                ~/projects/commands <b>on master</b>
              </span>
              <code>❯ cargo test</code>
              <span className="preview-cursor" />
            </div>
            <div className="theme-description">
              <span>
                <strong>{theme.name}</strong>
                <small>{theme.subtitle}</small>
              </span>
              <span className="radio-check">
                {settings.theme === theme.id && <Check size={13} />}
              </span>
            </div>
          </button>
        ))}
      </div>
      <section className="content-card settings-card">
        <div className="setting-row">
          <div>
            <h4>字号</h4>
            <p>等宽字体，宽字符按两格排版。</p>
          </div>
          <select
            aria-label="终端字号"
            value={settings.fontSize}
            onChange={(event) =>
              updateSettings({ fontSize: Number(event.target.value) })
            }
          >
            {[12, 13, 14, 15, 16].map((size) => (
              <option value={size} key={size}>
                {size}px
              </option>
            ))}
          </select>
        </div>
        <div className="setting-row">
          <div>
            <h4>显示上下文提示符</h4>
            <p>显示目录、Git 分支与语言版本，类似 starship。</p>
          </div>
          <Toggle
            label="显示上下文提示符"
            checked={settings.prompt}
            onChange={(value) => updateSettings({ prompt: value })}
          />
        </div>
        <div className="setting-row">
          <div>
            <h4>恢复默认</h4>
            <p>把外观与建议相关的设置还原。</p>
          </div>
          <button
            className="secondary-button"
            onClick={() => updateSettings(defaultSettings)}
          >
            重置偏好
          </button>
        </div>
      </section>
    </div>
  );
}

/* ───────────────────────────── Integrations ───────────────────────────── */

function IntegrationsPage({ settings, updateSettings, openInstall, notify }) {
  return (
    <>
      <section className="content-card">
        <div className="section-label">
          <h3>只用它的提示符</h3>
          <p>
            不换 shell 也能用。把下面一行加进对应的启动文件，cmds 只负责渲染提示符。
          </p>
        </div>
        <div className="integration-snippets">
          {integrationSnippets.map(([shell, command, file]) => (
            <div className="integration-snippet" key={shell}>
              <div>
                <strong>{shell}</strong>
                <small>{file}</small>
              </div>
              <code>{command}</code>
              <CopyButton text={command} onCopy={notify} />
            </div>
          ))}
        </div>
      </section>
      <div className="integration-grid">
        <article className="content-card integration-card">
          <span className="integration-logo starship-logo">
            <Star size={28} />
          </span>
          <span className="integration-badge">提示符思路来源</span>
          <h3>starship</h3>
          <p>
            模块化提示符沿用 starship 的思路：目录、Git 分支、语言版本、耗时都是独立模块，用 TOML 拼装。
          </p>
          <a
            href="https://starship.rs/guide/"
            target="_blank"
            rel="noreferrer"
            className="secondary-button"
          >
            starship 官方指南 <ExternalLink size={14} />
          </a>
        </article>
        <article className="content-card integration-card">
          <span className="integration-logo fish-logo">
            <Fish size={28} />
          </span>
          <span className="integration-badge">输入体验来源</span>
          <h3>fish</h3>
          <p>
            历史自动建议与缩写（abbr）来自 fish 的体验。cmds 用一个零依赖的二进制实现同一套手感。
          </p>
          <a
            href="https://fishshell.com/"
            target="_blank"
            rel="noreferrer"
            className="secondary-button"
          >
            了解 fish <ExternalLink size={14} />
          </a>
        </article>
      </div>
      <section className="content-card settings-card">
        <div className="setting-row">
          <div>
            <h4>自动弹出候选</h4>
            <p>边打字边显示匹配的命令。关掉之后仍可按 Tab 手动唤出。</p>
          </div>
          <Toggle
            checked={settings.auto}
            onChange={(value) => updateSettings({ auto: value })}
            label="自动弹出候选"
          />
        </div>
        <div className="setting-row">
          <div>
            <h4>历史参与匹配</h4>
            <p>把用过的命令混进候选。以空格开头的命令不会进历史。</p>
          </div>
          <Toggle
            checked={settings.history}
            onChange={(value) => updateSettings({ history: value })}
            label="历史参与匹配"
          />
        </div>
        <div className="setting-row">
          <div>
            <h4>推荐命令</h4>
            <p>混入内置命令库里的常用命令。</p>
          </div>
          <Toggle
            checked={settings.recommendations}
            onChange={(value) => updateSettings({ recommendations: value })}
            label="推荐命令"
          />
        </div>
      </section>
      <div className="notice integrations-notice">
        <ShieldCheck size={18} />
        <span>
          以上开关只影响这个网页的 Playground。要在本机执行真实命令需要安装 cmds；这个页面不会在你的机器上装任何东西。
        </span>
        <button className="text-link" onClick={openInstall}>
          安装 cmds <ArrowRight size={15} />
        </button>
      </div>
    </>
  );
}

/* ───────────────────────────── Guide ───────────────────────────── */

const guideTopics = [
  "安装与上手",
  "输入与补全",
  "快捷键",
  "配置",
  "接到现有 shell",
  "历史与隐私",
];

function GuidePage({ openInstall, queue, notify }) {
  const [topic, setTopic] = useState(guideTopics[0]);
  return (
    <div className="guide-layout">
      <aside className="guide-toc">
        <span>使用指南</span>
        {guideTopics.map((item) => (
          <button
            className={topic === item ? "active" : ""}
            onClick={() => setTopic(item)}
            key={item}
          >
            {item}
            <ChevronRight size={14} />
          </button>
        ))}
        <a href={BRAND.repoUrl} target="_blank" rel="noreferrer">
          源码与 Issue <ExternalLink size={13} />
        </a>
      </aside>
      <article className="content-card guide-article">
        <span className="eyebrow">
          {BRAND.name} v{BRAND.version}
        </span>
        <h2>{topic}</h2>

        {topic === "安装与上手" && (
          <>
            <p>
              cmds 是一个交互式终端：边打字边给历史建议，<Key>Tab</Key>{" "}
              弹候选菜单用 <Key>↑</Key> <Key>↓</Key>{" "}
              选。单个可执行文件，零运行时依赖。
            </p>
            <h3>01 — 先在浏览器里试</h3>
            <p>
              Playground 是完全隔离的模拟环境，不读本地文件、不起进程、不发网络请求。手感和装好之后一致。
            </p>
            <button className="secondary-button" onClick={() => queue("git ")}>
              去 Playground <ArrowRight size={15} />
            </button>
            <h3>02 — 装到本机</h3>
            <p>
              一键脚本会自动识别平台并优先下载预编译二进制，没有对应平台的包时回退到 cargo 构建。用户级安装，不需要 sudo，也不改动你的 shell 启动文件。
            </p>
            <button className="primary-button" onClick={openInstall}>
              <Download size={16} />
              查看安装命令
            </button>
            <h3>03 — 三十秒上手</h3>
            <div className="shortcut-table">
              {[
                ["cmds", "进入交互式 shell"],
                ["help", "查看全部快捷键与内建命令"],
                ["cmds config init", "生成配置模板"],
                ['cmds -c "cargo test"', "执行一条命令后退出"],
              ].map(([command, text]) => (
                <div key={command}>
                  <code>{command}</code>
                  <span>{text}</span>
                </div>
              ))}
            </div>
            <div className="notice">
              <Leaf size={17} />
              <span>
                不需要账号，没有云端历史，没有遥测。依赖只有 crossterm 与 unicode-width。
              </span>
            </div>
          </>
        )}

        {topic === "输入与补全" && (
          <>
            <p>
              两条建议通道：光标后的灰色文字是<strong>历史建议</strong>，
              <Key>Tab</Key> 弹出的是<strong>候选菜单</strong>。
            </p>
            <h3>历史建议</h3>
            <p>
              输入时在光标后用灰色显示最近匹配的历史命令。<Key>→</Key>、
              <Key>End</Key> 或 <Key>Ctrl-F</Key> 采纳整条，<Key>Alt-→</Key>{" "}
              只采纳一个词。历史未命中时会退回补全建议。
            </p>
            <h3>候选菜单</h3>
            <p>
              历史整条命令排最前，其后依次是内建命令、别名、缩写、PATH 命令、目录文件、变量。
              <Key>↑</Key> <Key>↓</Key> 选择，<Key>Enter</Key> 采纳，
              <Key>Esc</Key> 关闭。采纳<strong>永远不会直接执行</strong>
              ，需要再按一次 <Key>Enter</Key>。
            </p>
            <h3>补全细节</h3>
            <p>
              唯一候选直接补全；多候选先补公共前缀；<code>cd</code>{" "}
              后只列目录；含空格的路径自动加引号。命令拼错时按编辑距离给建议，内建命令与你自己的别名优先于 PATH 里的同分候选。
            </p>
            <h3>缩写</h3>
            <p>
              <code>abbr gcm &quot;git commit -m&quot;</code> 之后输入{" "}
              <code>gcm</code> 按空格即展开，历史里保存的是展开后的完整命令。
            </p>
          </>
        )}

        {topic === "快捷键" && (
          <>
            <p>手不离键盘。完整列表也可以在 cmds 里输入 help 查看。</p>
            <div className="shortcut-table">
              {[
                ["Tab / Shift-Tab", "打开候选菜单 / 菜单内上一项"],
                ["↑ ↓ (Ctrl-P/Ctrl-N)", "菜单内移动；菜单关闭时按前缀翻历史"],
                ["Enter", "菜单打开时采纳候选，否则执行"],
                ["→ / End / Ctrl-F", "采纳整条灰色历史建议"],
                ["Alt-→ / Alt-←", "采纳建议中的一个词 / 按词左移"],
                ["Ctrl-R", "模糊搜索历史"],
                ["Ctrl-A / Ctrl-E", "行首 / 行尾"],
                ["Ctrl-W / Ctrl-U / Ctrl-K", "删词 / 删到行首 / 删到行尾"],
                ["Ctrl-L", "清屏"],
                ["Ctrl-C / Ctrl-D", "放弃当前输入 / 空行退出"],
              ].map(([key, text]) => (
                <div key={key}>
                  <Key>{key}</Key>
                  <span>{text}</span>
                </div>
              ))}
            </div>
          </>
        )}

        {topic === "配置" && (
          <>
            <p>
              配置文件在 <code>~/.config/cmds/config.toml</code>（Windows：
              <code>%APPDATA%\cmds\config.toml</code>），可用{" "}
              <code>CMDS_CONFIG</code> 指定其它路径。
              <code>cmds config init</code> 生成模板，<code>config reload</code>{" "}
              热重载。
            </p>
            <h3>提示符模块</h3>
            <p>
              <code>$dir</code> <code>$git_branch</code> <code>$git_status</code>{" "}
              <code>$languages</code> <code>$cmd_duration</code>{" "}
              <code>$status</code> <code>$character</code> <code>$time</code>{" "}
              <code>$identity</code> <code>$jobs</code>，用 <code>format</code>{" "}
              串起来即可。
            </p>
            <div className="install-code">
              <span>$</span>
              <code>cmds config init</code>
              <CopyButton text="cmds config init" onCopy={notify} />
            </div>
            <h3>启动脚本</h3>
            <p>
              <code>~/.cmdsrc</code>（或 <code>~/.config/cmds/init.cmds</code>
              ）里可以写任意 cmds 命令，用 <code>CMDS_RC</code> 可以指定别的路径。
            </p>
            <div className="notice">
              <SlidersHorizontal size={17} />
              <span>Config 页可以勾选模块直接生成这份文件，省得手写 TOML。</span>
            </div>
          </>
        )}

        {topic === "接到现有 shell" && (
          <>
            <p>
              不想换 shell，也可以只用它的提示符。cmds 会输出一段集成脚本，接到现有的 bash / zsh / fish / PowerShell。
            </p>
            <div className="shortcut-table">
              {integrationSnippets.map(([shell, command]) => (
                <div key={shell}>
                  <code>{command}</code>
                  <span>{shell}</span>
                </div>
              ))}
            </div>
            <h3>执行能力</h3>
            <p>
              作为 shell 使用时支持管道、<code>&amp;&amp;</code> <code>||</code>{" "}
              <code>;</code>、重定向（<code>&gt;</code> <code>&gt;&gt;</code>{" "}
              <code>&lt;</code> <code>2&gt;</code> <code>&amp;&gt;</code>）、后台{" "}
              <code>&amp;</code>、glob（<code>*</code> <code>?</code>{" "}
              <code>[a-z]</code> <code>**</code>）与变量展开。
            </p>
            <h3>定位说明</h3>
            <p>
              cmds 面向<strong>交互使用</strong>，不是 POSIX sh 的替代品，没有函数与{" "}
              <code>if/for</code> 等脚本语法。系统脚本请继续用 <code>sh</code>/
              <code>bash</code>；设为登录 shell 前建议先日常用一段时间。
            </p>
          </>
        )}

        {topic === "历史与隐私" && (
          <>
            <p>命令是你自己的。cmds 不会把命令、历史或建议发到任何服务器。</p>
            <h3>网页 Playground</h3>
            <p>
              历史、片段与偏好都保存在这个浏览器的 localStorage 里。History 页可以导出 JSON 或一键清空。隐私模式下写入失败时，当次会话仍然可用。
            </p>
            <h3>本机 cmds</h3>
            <p>
              历史文件在 <code>~/.local/share/cmds/history</code>（Windows：
              <code>%LOCALAPPDATA%\cmds\history</code>），Unix 下是仅所有者可读写。以空格开头的命令不会写入历史。不要把密钥直接写在命令行里，用环境变量或密钥管理工具。
            </p>
            <h3>卸载</h3>
            <p>
              删掉安装目录里的 <code>cmds</code>{" "}
              可执行文件即可。配置与历史是独立文件，可以保留也可以一起删。没有需要还原的 shell 启动配置。
            </p>
          </>
        )}
      </article>
    </div>
  );
}

/* ───────────────────────────── App ───────────────────────────── */

function readPage() {
  const value = window.location.hash.slice(1).toLowerCase();
  return (
    Object.keys(titles).find((name) => name.toLowerCase() === value) ??
    "Playground"
  );
}

export default function App() {
  const [page, setPage] = useState(readPage);
  const [history, setHistory] = useState(() => {
    const value = loadStored(STORAGE.history, initialHistory);
    return Array.isArray(value)
      ? value.filter(
          (item) =>
            typeof item?.command === "string" &&
            typeof item?.time === "string" &&
            typeof item?.shell === "string",
        )
      : initialHistory;
  });
  const [settings, setSettings] = useState(() => {
    const value = loadStored(STORAGE.settings, defaultSettings);
    return {
      ...defaultSettings,
      ...(value && typeof value === "object" ? value : {}),
    };
  });
  const [snippets, setSnippets] = useState(() => {
    const value = loadStored(STORAGE.snippets, startingSnippets);
    return Array.isArray(value)
      ? value.filter(
          (item) =>
            typeof item?.command === "string" && typeof item?.title === "string",
        )
      : startingSnippets;
  });
  const [config, setConfig] = useState(() => {
    const value = loadStored(STORAGE.config, defaultConfig);
    const merged = {
      ...defaultConfig,
      ...(value && typeof value === "object" ? value : {}),
    };
    if (!Array.isArray(merged.modules)) merged.modules = defaultConfig.modules;
    return merged;
  });
  const [session, setSession] = useState("cmds");
  const [modal, setModal] = useState(null);
  const [platform, setPlatform] = useState("macOS");
  const [toast, setToast] = useState("");
  const toastTimer = useRef();
  const [paletteQuery, setPaletteQuery] = useState("");
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [queuedCommand, setQueuedCommand] = useState(null);

  const navigate = useCallback((name) => {
    setPage(name);
    window.location.hash = name.toLowerCase();
    setSidebarOpen(false);
  }, []);

  const notify = useCallback((message = "已复制到剪贴板。") => {
    setToast(message);
    clearTimeout(toastTimer.current);
    toastTimer.current = setTimeout(() => setToast(""), 3500);
  }, []);

  function updateSettings(patch) {
    setSettings((current) => ({ ...current, ...patch }));
  }

  const queue = useCallback(
    (command) => {
      navigate("Playground");
      setQueuedCommand(command);
      setModal(null);
    },
    [navigate],
  );

  function addHistory(item) {
    setHistory((current) => {
      const previous = current.find((entry) => entry.command === item.command);
      const next = { ...item, count: (previous?.count ?? 0) + 1 };
      return [
        next,
        ...current.filter((entry) => entry.command !== item.command),
      ].slice(0, 200);
    });
  }

  useEffect(() => {
    saveStored(STORAGE.history, history);
  }, [history]);
  useEffect(() => {
    saveStored(STORAGE.settings, settings);
  }, [settings]);
  useEffect(() => {
    saveStored(STORAGE.snippets, snippets);
  }, [snippets]);
  useEffect(() => {
    saveStored(STORAGE.config, config);
  }, [config]);

  useEffect(() => {
    const onHashChange = () => setPage(readPage());
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, []);

  useEffect(() => {
    const onKeyDown = (event) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPaletteQuery("");
        setModal((current) => (current === "palette" ? null : "palette"));
      }
      if ((event.metaKey || event.ctrlKey) && event.key === "1") {
        event.preventDefault();
        navigate("Playground");
        setModal(null);
        requestAnimationFrame(() =>
          document.querySelector('[aria-label="终端命令"]')?.focus(),
        );
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      clearTimeout(toastTimer.current);
    };
  }, [navigate]);

  const paletteItems = [
    ...Object.keys(titles).map((name) => ({
      label: `前往 ${name}`,
      icon: name === "Playground" ? Terminal : ArrowRight,
      action: () => {
        navigate(name);
        setModal(null);
      },
    })),
    ...commandCatalog.map((item) => ({
      label: item.command,
      icon: Terminal,
      action: () => queue(item.command),
    })),
  ]
    .filter((item) =>
      item.label.toLowerCase().includes(paletteQuery.toLowerCase()),
    )
    .slice(0, 9);

  return (
    <div className="app-shell">
      {sidebarOpen && (
        <button
          className="sidebar-backdrop"
          onClick={() => setSidebarOpen(false)}
          aria-label="关闭导航"
        />
      )}
      <aside className={`sidebar ${sidebarOpen ? "open" : ""}`}>
        <a
          className="brand"
          href="#playground"
          onClick={() => navigate("Playground")}
        >
          <Logo />
          <span>
            cmds<span className="brand-period">.</span>
          </span>
        </a>
        <button
          className="workspace-switch"
          onClick={() => setModal("workspace")}
        >
          <span className="workspace-avatar">C</span>
          <span>
            本地工作区<small>不需要账号，数据只在本机</small>
          </span>
          <ChevronDown size={14} />
        </button>
        <nav aria-label="主导航">
          <div className="nav-label">工作区</div>
          {navigation
            .filter((item) => item.section === "workspace")
            .map(({ name, icon: Icon }) => (
              <button
                key={name}
                className={`nav-item ${page === name ? "active" : ""}`}
                onClick={() => navigate(name)}
              >
                <Icon size={18} />
                <span>{name}</span>
                {name === "Playground" && (
                  <span className="nav-shortcut">⌘ 1</span>
                )}
                {name === "History" && (
                  <span className="nav-count">{history.length}</span>
                )}
              </button>
            ))}
          <div className="nav-label configuration-label">配置</div>
          {navigation
            .filter((item) => item.section === "configuration")
            .map(({ name, icon: Icon }) => (
              <button
                key={name}
                className={`nav-item ${page === name ? "active" : ""}`}
                onClick={() => navigate(name)}
              >
                <Icon size={18} />
                <span>{name}</span>
              </button>
            ))}
          <div className="nav-label resources-label">资源</div>
          <button
            className={`nav-item ${page === "Guide" ? "active" : ""}`}
            onClick={() => navigate("Guide")}
          >
            <BookOpen size={18} />
            <span>使用指南</span>
          </button>
          <a
            className="nav-item"
            href={BRAND.repoUrl}
            target="_blank"
            rel="noreferrer"
          >
            <GithubMark size={18} />
            <span>GitHub</span>
            <ArrowUp className="diagonal-arrow" size={14} />
          </a>
        </nav>
        <div className="sidebar-bottom">
          <div className="sidebar-note">
            <span className="note-icon">
              <Sparkles size={17} />
            </span>
            <h4>一个可执行文件</h4>
            <p>
              零运行时依赖，
              <br />
              依赖只有两个 crate。
            </p>
            <button onClick={() => setModal("release")}>
              v{BRAND.version} 有什么 <ArrowRight size={13} />
            </button>
          </div>
          <div className="sidebar-footer">
            <span>
              <span className="status-green" />
              99 个单元测试
            </span>
            <span>v{BRAND.version}</span>
          </div>
        </div>
      </aside>
      <div className="workspace-main">
        <header className="topbar">
          <div className="breadcrumb">
            <button
              className="icon-button mobile-menu"
              aria-label="打开导航"
              onClick={() => setSidebarOpen(true)}
            >
              <Menu size={19} />
            </button>
            <PanelLeftClose size={17} className="desktop-sidebar-icon" />
            <span className="breadcrumb-divider" />
            <span>cmds</span>
            <ChevronRight size={13} />
            <strong>{page}</strong>
          </div>
          <div className="topbar-right">
            <button
              className="global-search"
              onClick={() => {
                setPaletteQuery("");
                setModal("palette");
              }}
            >
              <Search size={15} />
              <span>跳转到…</span>
              <Key>⌘ K</Key>
            </button>
            <button
              className="icon-button help-button"
              aria-label="快捷键"
              onClick={() => setModal("shortcuts")}
            >
              <CircleHelp size={18} />
            </button>
          </div>
        </header>
        <main className="main-content">
          <div className="page-heading">
            <div>
              <div className="eyebrow">
                <span />
                {BRAND.name} · 交互式终端
              </div>
              <h1>{titles[page][0]}</h1>
              <p>{titles[page][1]}</p>
            </div>
            <button
              className="primary-button install-top"
              onClick={() => setModal("install")}
            >
              <Download size={16} />
              安装 cmds
            </button>
          </div>

          <div style={{ display: page === "Playground" ? undefined : "none" }}>
            <div className="terminal-section">
              <div className="terminal-column">
                <div className="section-top">
                  <h2>
                    <Terminal size={16} />
                    Playground
                    <span className="live-badge">
                      <span />
                      模拟环境
                    </span>
                  </h2>
                  <button
                    className="text-link shortcuts-link"
                    onClick={() => setModal("shortcuts")}
                  >
                    <Keyboard size={15} />
                    快捷键
                  </button>
                </div>
                <TerminalPanel
                  session={session}
                  setSession={setSession}
                  history={history}
                  addHistory={addHistory}
                  settings={settings}
                  queuedCommand={queuedCommand}
                  consumeCommand={() => setQueuedCommand(null)}
                  notify={notify}
                />
                <div className="terminal-caption">
                  <ShieldCheck size={13} />
                  <span>
                    命令全部在浏览器里模拟：不读本地文件、不起进程、不发网络请求。
                  </span>
                  <button
                    className="text-link"
                    onClick={() => navigate("Guide")}
                  >
                    看看原理 <ArrowRight size={12} />
                  </button>
                </div>
              </div>
              <aside className="right-rail">
                <section className="content-card environment-card">
                  <div className="card-title">
                    <h3>这是什么</h3>
                    <span className="tiny-status" />
                  </div>
                  <div className="environment-row">
                    <span className="environment-icon shell-env">
                      <Terminal size={17} />
                    </span>
                    <div>
                      <strong>交互式 shell</strong>
                      <small>管道 / 重定向 / glob / 后台任务</small>
                    </div>
                  </div>
                  <div className="environment-row">
                    <span className="environment-icon starship-env">
                      <Star size={18} />
                    </span>
                    <div>
                      <strong>模块化提示符</strong>
                      <small>starship 式，TOML 配置</small>
                    </div>
                  </div>
                  <div className="environment-row">
                    <span className="environment-icon fish-env">
                      <Fish size={19} />
                    </span>
                    <div>
                      <strong>fish 式输入</strong>
                      <small>历史建议 + 缩写展开</small>
                    </div>
                  </div>
                  <div className="card-divider" />
                  <div className="suggestion-setting">
                    <span>自动弹出候选</span>
                    <Toggle
                      label="自动弹出候选"
                      checked={settings.auto}
                      onChange={(value) => updateSettings({ auto: value })}
                    />
                  </div>
                  <div className="suggestion-setting">
                    <span>历史参与匹配</span>
                    <Toggle
                      label="历史参与匹配"
                      checked={settings.history}
                      onChange={(value) => updateSettings({ history: value })}
                    />
                  </div>
                  <button
                    className="card-footer-link"
                    onClick={() => navigate("Integrations")}
                  >
                    更多集成方式 <ArrowRight size={14} />
                  </button>
                </section>
                <section className="make-yours-card">
                  <div className="decorative-orbit" aria-hidden="true">
                    <SlidersHorizontal size={40} />
                  </div>
                  <span className="customize-label">配置</span>
                  <h3>生成一份 config.toml</h3>
                  <p>
                    勾选提示符模块，
                    <br />
                    直接拿到可用的配置文件。
                  </p>
                  <button onClick={() => navigate("Config")}>
                    去生成 <ArrowRight size={14} />
                  </button>
                </section>
              </aside>
            </div>
            <div className="features-heading">
              <span>核心特性</span>
              <div />
            </div>
            <div className="feature-grid">
              <button className="feature-card" onClick={() => queue("git c")}>
                <span className="feature-icon">
                  <Sparkles size={20} />
                </span>
                <div>
                  <h3>历史自动建议</h3>
                  <p>
                    光标后用灰色显示最近匹配的
                    <br className="desktop-break" /> 历史命令，按 → 采纳整条。
                  </p>
                  <span>
                    试试输入 git c <ArrowRight size={13} />
                  </span>
                </div>
              </button>
              <button
                className="feature-card"
                onClick={() => navigate("History")}
              >
                <span className="feature-icon warm">
                  <History size={20} />
                </span>
                <div>
                  <h3>按频率排序的历史</h3>
                  <p>
                    使用频率 + 新鲜度综合排序，
                    <br className="desktop-break" /> 常用的永远在最前面。
                  </p>
                  <span>
                    查看历史 <ArrowRight size={13} />
                  </span>
                </div>
              </button>
              <button
                className="feature-card"
                onClick={() => setModal("shortcuts")}
              >
                <span className="feature-icon lilac">
                  <Zap size={20} />
                </span>
                <div>
                  <h3>一个二进制</h3>
                  <p>
                    零运行时依赖，
                    <br className="desktop-break" /> 依赖只有两个 crate。
                  </p>
                  <span>
                    快捷键一览 <ArrowRight size={13} />
                  </span>
                </div>
              </button>
            </div>
            <div className="install-banner">
              <span className="banner-logo">
                <Logo small />
              </span>
              <div>
                <h3>装上试试</h3>
                <p>
                  一条命令，自动识别平台。不需要 sudo，不改动 shell 启动文件。
                </p>
              </div>
              <button
                className="secondary-button"
                onClick={() => setModal("install")}
              >
                <Copy size={14} />
                获取安装命令
                <ArrowRight size={14} />
              </button>
            </div>
          </div>

          {page === "History" && (
            <HistoryPage
              history={history}
              setHistory={setHistory}
              queue={queue}
              notify={notify}
            />
          )}
          {page === "Snippets" && (
            <SnippetsPage
              snippets={snippets}
              setSnippets={setSnippets}
              queue={queue}
              notify={notify}
            />
          )}
          {page === "Config" && (
            <ConfigPage config={config} setConfig={setConfig} notify={notify} />
          )}
          {page === "Appearance" && (
            <AppearancePage
              settings={settings}
              updateSettings={updateSettings}
            />
          )}
          {page === "Integrations" && (
            <IntegrationsPage
              settings={settings}
              updateSettings={updateSettings}
              openInstall={() => setModal("install")}
              notify={notify}
            />
          )}
          {page === "Guide" && (
            <GuidePage
              openInstall={() => setModal("install")}
              queue={queue}
              notify={notify}
            />
          )}

          <footer className="page-footer">
            <span>
              灵感来自 starship 与 fish<span className="footer-dot">•</span>
              ISC 许可
            </span>
            <span>
              <Leaf size={12} />
              依赖只有 crossterm 与 unicode-width
            </span>
          </footer>
        </main>
      </div>

      {modal === "install" && (
        <Modal
          title="安装 cmds"
          subtitle="一条命令，自动识别平台。"
          onClose={() => setModal(null)}
          wide
        >
          <InstallContent
            notify={notify}
            platform={platform}
            setPlatform={setPlatform}
          />
        </Modal>
      )}
      {modal === "shortcuts" && (
        <Modal
          title="快捷键"
          subtitle="和装好之后的 cmds 一致。"
          onClose={() => setModal(null)}
        >
          <div className="shortcut-table">
            {[
              ["Tab / Shift-Tab", "打开候选菜单 / 菜单内上一项"],
              ["↑ ↓", "菜单内移动"],
              ["Enter", "采纳候选，再按一次才执行"],
              ["→ / End", "采纳整条灰色历史建议"],
              ["Esc", "关闭候选菜单"],
              ["Ctrl-C", "放弃当前输入"],
              ["Ctrl-L", "清屏"],
              ["⌘ / Ctrl + K", "跳转到任意页面或命令"],
            ].map(([key, text]) => (
              <div key={key}>
                <span>{text}</span>
                <Key>{key}</Key>
              </div>
            ))}
          </div>
          <p className="modal-footnote">采纳候选永远不会自动执行命令。</p>
        </Modal>
      )}
      {modal === "palette" && (
        <Modal title="跳转到…" onClose={() => setModal(null)}>
          <label className="palette-search">
            <Search size={19} />
            <input
              placeholder="页面或命令…"
              aria-label="搜索页面与命令"
              autoFocus
              value={paletteQuery}
              onChange={(event) => setPaletteQuery(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && paletteItems[0]) {
                  event.preventDefault();
                  event.stopPropagation();
                  paletteItems[0].action();
                }
                if (event.key === "ArrowDown") {
                  event.preventDefault();
                  document.querySelector(".palette-results button")?.focus();
                }
              }}
            />
          </label>
          <div className="palette-results">
            {paletteItems.map(({ label, icon: Icon, action }, index) => (
              <button
                key={label}
                onClick={action}
                onKeyDown={(event) => {
                  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
                    event.preventDefault();
                    const items =
                      event.currentTarget.parentElement.querySelectorAll(
                        "button",
                      );
                    items[
                      (index +
                        (event.key === "ArrowDown" ? 1 : -1) +
                        items.length) %
                        items.length
                    ].focus();
                  }
                }}
              >
                <Icon size={16} />
                <span>{label}</span>
                <ArrowDownLeft size={14} />
              </button>
            ))}
            {!paletteItems.length && (
              <p className="palette-empty">
                没有匹配项。试试 “config” 或 “git”。
              </p>
            )}
          </div>
        </Modal>
      )}
      {modal === "workspace" && (
        <Modal
          title="本地工作区"
          subtitle="不需要账号，历史与偏好都留在这个浏览器里。"
          onClose={() => setModal(null)}
        >
          <div className="workspace-stats">
            <div>
              <strong>{history.length}</strong>
              <span>条历史</span>
            </div>
            <div>
              <strong>{snippets.length}</strong>
              <span>个片段</span>
            </div>
            <div>
              <strong>0</strong>
              <span>云端依赖</span>
            </div>
          </div>
          <button
            className="primary-button full-width"
            onClick={() => {
              navigate("Guide");
              setModal(null);
            }}
          >
            打开使用指南 <ArrowRight size={15} />
          </button>
        </Modal>
      )}
      {modal === "release" && (
        <Modal
          title={`${BRAND.name} v${BRAND.version}`}
          subtitle="第一个可用版本。"
          onClose={() => setModal(null)}
        >
          <div className="release-list">
            <p>
              <Sparkles size={18} />
              <span>历史自动建议 + Tab 候选菜单，采纳与执行严格分开。</span>
            </p>
            <p>
              <Star size={18} />
              <span>
                模块化提示符：目录、Git 分支与状态、语言版本、耗时、退出码。
              </span>
            </p>
            <p>
              <Terminal size={18} />
              <span>
                管道、逻辑连接、重定向、后台任务、glob 与变量展开；也可只作为提示符接到现有 shell。
              </span>
            </p>
            <p>
              <ShieldCheck size={18} />
              <span>
                本地优先：没有账号、没有云端历史、没有遥测。99 个单元测试。
              </span>
            </p>
          </div>
          <button
            className="primary-button full-width"
            onClick={() => setModal("install")}
          >
            查看安装命令 <ArrowRight size={15} />
          </button>
        </Modal>
      )}
      {toast && (
        <div className="toast" role="status">
          <Check size={16} />
          {toast}
          <button aria-label="关闭提示" onClick={() => setToast("")}>
            <X size={14} />
          </button>
        </div>
      )}
    </div>
  );
}
