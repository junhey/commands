/**
 * 安装命令生成。
 *
 * 站点可能被部署在域名根目录，也可能在 GitHub Pages 的子路径（/commands/），
 * 所以安装地址一律由运行时的 origin + base 推导，绝不硬编码一个不存在的域名。
 */

import { BRAND } from "./commands.js";
import { t } from "./i18n.js";

/** 把 origin 与 base 拼成可直接访问的资源地址。 */
export function assetUrl(file, origin, base = "/") {
  const cleanBase = base.endsWith("/") ? base : `${base}/`;
  const cleanOrigin = origin.replace(/\/+$/, "");
  return `${cleanOrigin}${cleanBase}${file}`;
}

/**
 * 返回各平台的安装方式。
 *
 * - `shell`：一键脚本，脚本本身托管在本站，可以先点开审阅再执行
 * - `cargo`：源码安装，不依赖本站与 Release 资产
 * - `release`：直接下载对应平台的预编译包
 */
export function installMethods(origin, base = "/") {
  const shUrl = assetUrl("install.sh", origin, base);
  const ps1Url = assetUrl("install.ps1", origin, base);
  const releases = `${BRAND.repoUrl}/releases/latest`;

  return {
    macOS: [
      {
        id: "script",
        label: t("One-liner", "一键安装"),
        command: `curl -fsSL ${shUrl} | sh`,
        inspect: shUrl,
        note: t(
          "Installs for your user only: no sudo, and it never edits your shell startup files. Prefers a prebuilt binary and falls back to a cargo build when no package matches.",
          "用户级安装，不需要 sudo，也不会改动你的 shell 启动文件。优先下载预编译二进制，没有对应平台的包时回退到 cargo 构建。",
        ),
      },
      {
        id: "cargo",
        label: t("From source", "从源码"),
        command: `cargo install --locked ${BRAND.bin}`,
        note: t(
          "Installs from crates.io; needs Rust 1.85+. Depends on neither this site nor the release assets.",
          "从 crates.io 安装，需要 Rust 1.85+。不依赖本站，也不依赖 Release 资产。",
        ),
      },
    ],
    Linux: [
      {
        id: "script",
        label: t("One-liner", "一键安装"),
        command: `curl -fsSL ${shUrl} | sh`,
        inspect: shUrl,
        note: t(
          "Supports x86_64 / aarch64 / armv7 / riscv64. Installs to /usr/local/bin when writable, otherwise ~/.local/bin.",
          "支持 x86_64 / aarch64 / armv7 / riscv64。安装目录默认 /usr/local/bin，不可写时自动改用 ~/.local/bin。",
        ),
      },
      {
        id: "cargo",
        label: t("From source", "从源码"),
        command: `cargo install --locked ${BRAND.bin}`,
        note: t(
          "Installs from crates.io; needs Rust 1.85+.",
          "从 crates.io 安装，需要 Rust 1.85+。",
        ),
      },
    ],
    Windows: [
      {
        id: "script",
        label: t("One-liner", "一键安装"),
        command: `irm ${ps1Url} | iex`,
        inspect: ps1Url,
        note: t(
          "Installs to %LOCALAPPDATA%\\Programs\\cmds and adds that directory to your user PATH. No administrator rights needed.",
          "安装到 %LOCALAPPDATA%\\Programs\\cmds，并把该目录加入当前用户 PATH。不需要管理员权限。",
        ),
      },
      {
        id: "cargo",
        label: t("From source", "从源码"),
        command: `cargo install --locked ${BRAND.bin}`,
        note: t(
          "Installs from crates.io; needs Rust 1.85+ and the MSVC toolchain.",
          "从 crates.io 安装，需要 Rust 1.85+ 与 MSVC 工具链。",
        ),
      },
    ],
    releases,
  };
}

/** 安装后的三步上手提示。文案用 getter，语言切换后要跟着变。 */
export const nextSteps = [
  {
    get title() {
      return t("Start it", "直接启动");
    },
    get body() {
      return t(
        `Run ${BRAND.bin}, then type help for every keybinding and builtin.`,
        `运行 ${BRAND.bin}，输入 help 查看全部快捷键与内建命令。`,
      );
    },
    command: BRAND.bin,
  },
  {
    get title() {
      return t("Write a config", "生成配置");
    },
    get body() {
      return t(
        `${BRAND.bin} config init writes ~/.config/${BRAND.bin}/config.toml; the Config page can generate it visually too.`,
        `${BRAND.bin} config init 会写出 ~/.config/${BRAND.bin}/config.toml，也可以在 Config 页可视化生成。`,
      );
    },
    command: `${BRAND.bin} config init`,
  },
  {
    get title() {
      return t("Only want the prompt", "只想用它的提示符");
    },
    get body() {
      return t(
        "Keep your current bash / zsh / fish / PowerShell and let cmds render just the prompt.",
        "保留现有的 bash / zsh / fish / PowerShell，只接管提示符渲染。",
      );
    },
    command: `eval "$(${BRAND.bin} init bash)"`,
  },
];
