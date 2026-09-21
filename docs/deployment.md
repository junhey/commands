# 部署上线

站点是纯静态的：没有后端、没有数据库、没有账号体系，构建产物直接丢到任何静态托管上即可。
唯一的要求是 **`install.sh` / `install.ps1` 必须和页面资源一起放在站点根目录**，
否则 `curl -fsSL <站点>/install.sh | sh` 取不到脚本。

## 一、GitHub Pages（默认方案）

仓库里的 `.github/workflows/pages.yml` 已经配好了。首次启用：

1. 仓库 **Settings → Pages → Build and deployment → Source** 选 **GitHub Actions**
2. 推一次 `master`，或手动触发 `Deploy site` 工作流

站点地址：`https://junhey.github.io/commands/`
安装地址：`https://junhey.github.io/commands/install.sh`

工作流会自动把 `VITE_BASE` 设成 `/<仓库名>/`，所以子路径下资源引用是正确的。
页面用 hash 路由（`#config`、`#guide`），不需要服务端 rewrite。

> **私有仓库注意**：GitHub 对私有仓库启用 Pages 需要 Pro 或组织套餐。
> 仓库是私有且没有相应套餐时，`Deploy site` 会在部署一步失败，
> 但不影响 `CI` 与 `Release` 两个工作流。改为公开仓库后即可正常发布。

## 二、自己的域名

```sh
cd web
npm ci
VITE_BASE=/ npm run build      # 部署在域名根目录时 base 用 /
# 把 web/dist/ 整个目录上传到托管
```

部署在根目录时安装命令就变成：

```sh
curl -fsSL https://你的域名/install.sh | sh
```

站点上的安装对话框会用**运行时的 origin + base** 推导地址，不需要改代码——
换域名后显示的命令会自动跟着变。

### Cloudflare Pages / Vercel / Netlify

| 项 | 值 |
| --- | --- |
| 构建命令 | `npm ci && npm run build` |
| 构建目录 | `web` |
| 产物目录 | `web/dist` |
| 环境变量 | `VITE_BASE=/` |
| Node 版本 | 20.19+ 或 22+ |

### Nginx

```nginx
server {
    listen 443 ssl;
    server_name 你的域名;
    root /var/www/commands;

    # 安装脚本要以纯文本返回，且不允许浏览器猜类型
    location ~ ^/install\.(sh|ps1)$ {
        default_type text/plain;
        add_header X-Content-Type-Options nosniff;
        add_header Cache-Control "public, max-age=300";
    }

    # 带 hash 的静态资源可以长缓存
    location /assets/ {
        add_header Cache-Control "public, max-age=31536000, immutable";
    }

    location / {
        try_files $uri $uri/ /index.html;
    }
}
```

## 三、二进制分发

站点只负责分发**安装脚本**，真正的二进制在 GitHub Releases 上。
打个 `v*` 标签就会触发 `Release` 工作流：

```sh
git tag v0.1.0
git push origin v0.1.0
```

产出六个平台的包，命名必须与安装脚本里拼出的一致：

```
cmds-x86_64-unknown-linux-gnu.tar.gz    (+ .sha256)
cmds-aarch64-unknown-linux-gnu.tar.gz   (+ .sha256)
cmds-x86_64-apple-darwin.tar.gz         (+ .sha256)
cmds-aarch64-apple-darwin.tar.gz        (+ .sha256)
cmds-x86_64-pc-windows-msvc.zip         (+ .sha256)
cmds-aarch64-pc-windows-msvc.zip        (+ .sha256)
SHA256SUMS
```

> 改动 `install.sh` 里的 `BIN` / `REPO`，或改动 Release 工作流里的包名时，
> **两边必须同时改**，否则一键安装会 404 后静默退化成 `cargo` 源码构建。

没有 Release 时安装脚本仍可用：取不到预编译包会自动回退到
`cargo install --locked --git ...`，只是首次安装慢一些。

## 四、上线前检查

```sh
# 产物里有安装脚本
cd web && VITE_BASE=/ npm run build
test -f dist/install.sh && test -f dist/install.ps1

# 本地起一份产物，按站点上显示的命令实际跑一遍
npm run preview
curl -fsSL http://localhost:4173/install.sh | head -20
```

- [ ] `install.sh` 与 `install.ps1` 在站点根目录，返回 `text/plain`
- [ ] 安装对话框显示的域名是你的真实域名
- [ ] Releases 里有对应平台的包与 `.sha256`
- [ ] 至少在一台干净机器上跑通一键安装
