import test from "node:test";
import assert from "node:assert/strict";
import { assetUrl, installMethods, nextSteps } from "../src/install.js";

test("子路径部署时安装地址仍然正确", () => {
  assert.equal(
    assetUrl("install.sh", "https://junhey.github.io", "/commands/"),
    "https://junhey.github.io/commands/install.sh",
  );
});

test("base 缺少尾斜杠也能拼对", () => {
  assert.equal(
    assetUrl("install.sh", "https://example.com", "/commands"),
    "https://example.com/commands/install.sh",
  );
});

test("origin 带尾斜杠不会拼出双斜杠", () => {
  assert.equal(
    assetUrl("install.ps1", "https://example.com/", "/"),
    "https://example.com/install.ps1",
  );
});

test("本地开发时地址指向 dev server", () => {
  assert.equal(
    assetUrl("install.sh", "http://localhost:5173", "/"),
    "http://localhost:5173/install.sh",
  );
});

test("三个平台都有一键脚本与源码两种方式", () => {
  const methods = installMethods("https://junhey.github.io", "/commands/");
  for (const platform of ["macOS", "Linux", "Windows"]) {
    const ids = methods[platform].map((item) => item.id);
    assert.deepEqual(ids, ["script", "cargo"]);
  }
});

test("Unix 用 curl | sh，Windows 用 irm | iex", () => {
  const methods = installMethods("https://junhey.github.io", "/commands/");
  assert.match(methods.macOS[0].command, /^curl -fsSL \S+\/install\.sh \| sh$/);
  assert.match(methods.Linux[0].command, /^curl -fsSL \S+\/install\.sh \| sh$/);
  assert.match(methods.Windows[0].command, /^irm \S+\/install\.ps1 \| iex$/);
});

test("一键脚本提供可审阅的原文地址", () => {
  const methods = installMethods("https://junhey.github.io", "/commands/");
  assert.equal(
    methods.macOS[0].inspect,
    "https://junhey.github.io/commands/install.sh",
  );
  assert.equal(
    methods.Windows[0].inspect,
    "https://junhey.github.io/commands/install.ps1",
  );
});

test("源码安装指向真实仓库，不依赖站点域名", () => {
  const methods = installMethods("https://example.com", "/");
  for (const platform of ["macOS", "Linux", "Windows"]) {
    const cargo = methods[platform].find((item) => item.id === "cargo");
    assert.equal(
      cargo.command,
      "cargo install --locked --git https://github.com/junhey/commands cmds",
    );
  }
});

test("Releases 链接指向真实仓库", () => {
  const methods = installMethods("https://example.com", "/");
  assert.equal(
    methods.releases,
    "https://github.com/junhey/commands/releases/latest",
  );
});

test("上手步骤都带可复制的命令", () => {
  assert.equal(nextSteps.length, 3);
  assert.ok(nextSteps.every((step) => step.title && step.body && step.command));
});
