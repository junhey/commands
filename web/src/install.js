/**
 * 安装命令生成。
 *
 * 站点可能被部署在域名根目录，也可能在 GitHub Pages 的子路径（/commands/），
 * 所以安装地址一律由运行时的 origin + base 推导，绝不硬编码一个不存在的域名。
 */

import { BRAND } from "./commands.js";

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
        label: "一键安装",
        command: `curl -fsSL ${shUrl} | sh`,
        inspect: shUrl,
        note: "用户级安装，不需要 sudo，也不会改动你的 shell 启动文件。优先下载预编译二进制，没有对应平台的包时回退到 cargo 构建。",
      },
      {
        id: "cargo",
        label: "从源码",
        command: `cargo install --locked ${BRAND.bin}`,
        note: "从 crates.io 安装，需要 Rust 1.85+。不依赖本站，也不依赖 Release 资产。",
      },
    ],
    Linux: [
      {
        id: "script",
        label: "一键安装",
        command: `curl -fsSL ${shUrl} | sh`,
        inspect: shUrl,
        note: "支持 x86_64 / aarch64 / armv7 / riscv64。安装目录默认 /usr/local/bin，不可写时自动改用 ~/.local/bin。",
      },
      {
        id: "cargo",
        label: "从源码",
        command: `cargo install --locked ${BRAND.bin}`,
        note: "从 crates.io 安装，需要 Rust 1.85+。",
      },
    ],
    Windows: [
      {
        id: "script",
        label: "一键安装",
        command: `irm ${ps1Url} | iex`,
        inspect: ps1Url,
        note: "安装到 %LOCALAPPDATA%\\Programs\\cmds，并把该目录加入当前用户 PATH。不需要管理员权限。",
      },
      {
        id: "cargo",
        label: "从源码",
        command: `cargo install --locked ${BRAND.bin}`,
        note: "从 crates.io 安装，需要 Rust 1.85+ 与 MSVC 工具链。",
      },
    ],
    releases,
  };
}

/** 安装后的三步上手提示。 */
export const nextSteps = [
  {
    title: "直接启动",
    body: `运行 ${BRAND.bin}，输入 help 查看全部快捷键与内建命令。`,
    command: BRAND.bin,
  },
  {
    title: "生成配置",
    body: `${BRAND.bin} config init 会写出 ~/.config/${BRAND.bin}/config.toml，也可以在 Config 页可视化生成。`,
    command: `${BRAND.bin} config init`,
  },
  {
    title: "只想用它的提示符",
    body: "保留现有的 bash / zsh / fish / PowerShell，只接管提示符渲染。",
    command: `eval "$(${BRAND.bin} init bash)"`,
  },
];
