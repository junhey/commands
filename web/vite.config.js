import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { readFileSync } from "node:fs";

/**
 * 站点同时承担「一键安装」的托管职责：
 * `curl -fsSL <站点>/install.sh | sh` 取到的就是 install/ 目录里的真实脚本，
 * 所以开发服务器和构建产物都直接读取源文件，而不是维护一份容易过期的副本。
 */
function installScripts() {
  const files = {
    "/install.sh": new URL("../install/install.sh", import.meta.url),
    "/install.ps1": new URL("../install/install.ps1", import.meta.url),
  };
  const read = (url) => readFileSync(url, "utf8");

  return {
    name: "cmds-install-scripts",
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        const path = req.url?.split("?")[0] ?? "";
        const match = Object.keys(files).find((name) => path.endsWith(name));
        if (!match) return next();
        res.setHeader("Content-Type", "text/plain; charset=utf-8");
        res.setHeader("X-Content-Type-Options", "nosniff");
        res.end(read(files[match]));
      });
    },
    generateBundle() {
      for (const [name, url] of Object.entries(files)) {
        this.emitFile({
          type: "asset",
          fileName: name.slice(1),
          source: read(url),
        });
      }
    },
  };
}

export default defineConfig({
  // GitHub Pages 部署在 /commands/ 子路径下，CI 通过 VITE_BASE 传入
  base: process.env.VITE_BASE || "/",
  plugins: [react(), installScripts()],
  build: {
    outDir: "dist",
    sourcemap: false,
    // 纯静态站点，第三方依赖单独分包便于长期缓存
    rollupOptions: {
      output: {
        manualChunks: {
          vendor: ["react", "react-dom"],
          icons: ["lucide-react"],
        },
      },
    },
  },
});
