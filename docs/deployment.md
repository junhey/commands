# Deployment

> 中文版：[docs/deployment.zh-CN.md](https://github.com/junhey/commands/blob/master/docs/deployment.zh-CN.md)

The site is fully static: no backend, no database, no accounts. Drop the build output onto any
static host. The one requirement is that **`install.sh` and `install.ps1` must sit in the site
root alongside the page assets**, otherwise `curl -fsSL <site>/install.sh | sh` cannot fetch them.

## 1. GitHub Pages (the default)

`.github/workflows/pages.yml` is already set up. To enable it the first time:

1. **Settings → Pages → Build and deployment → Source** → choose **GitHub Actions**
2. Push to `master`, or trigger the `Deploy site` workflow manually

Site: `https://junhey.github.io/commands/`
Installer: `https://junhey.github.io/commands/install.sh`

The workflow sets `VITE_BASE` to `/<repo>/` automatically, so asset references are correct under
the sub-path. The page uses hash routing (`#config`, `#guide`), so no server-side rewrite is
needed.

> **Private repositories:** GitHub requires a Pro or organisation plan to enable Pages on a
> private repository. Without one, `Deploy site` fails at the deploy step while `CI` and `Release`
> keep working. Making the repository public fixes it.

## 2. Your own domain

```sh
cd web
npm ci
VITE_BASE=/ npm run build      # base is / when deploying at the domain root
# upload the whole web/dist/ directory to your host
```

Deployed at the root, the install command becomes:

```sh
curl -fsSL https://your-domain/install.sh | sh
```

The install dialog on the site derives the URL from the **runtime origin + base**, so no code
change is needed — the command shown updates itself when the domain changes.

### Cloudflare Pages / Vercel / Netlify

| Setting | Value |
| --- | --- |
| Build command | `npm ci && npm run build` |
| Build directory | `web` |
| Output directory | `web/dist` |
| Environment variable | `VITE_BASE=/` |
| Node version | 20.19+ or 22+ |

### Nginx

```nginx
server {
    listen 443 ssl;
    server_name your-domain;
    root /var/www/commands;

    # install scripts must come back as plain text, and no sniffing
    location ~ ^/install\.(sh|ps1)$ {
        default_type text/plain;
        add_header X-Content-Type-Options nosniff;
        add_header Cache-Control "public, max-age=300";
    }

    # hashed assets can be cached forever
    location /assets/ {
        add_header Cache-Control "public, max-age=31536000, immutable";
    }

    location / {
        try_files $uri $uri/ /index.html;
    }
}
```

## 3. Binary distribution

The site only distributes the **install scripts**; the binaries themselves live on GitHub
Releases. Pushing a `v*` tag triggers the `Release` workflow:

```sh
git tag v0.2.0
git push origin v0.2.0
```

It produces packages for six platforms, and the names must match what the install script builds:

```
cmds-x86_64-unknown-linux-gnu.tar.gz    (+ .sha256)
cmds-aarch64-unknown-linux-gnu.tar.gz   (+ .sha256)
cmds-x86_64-apple-darwin.tar.gz         (+ .sha256)
cmds-aarch64-apple-darwin.tar.gz        (+ .sha256)
cmds-x86_64-pc-windows-msvc.zip         (+ .sha256)
cmds-aarch64-pc-windows-msvc.zip        (+ .sha256)
SHA256SUMS
```

> When you change `BIN` / `REPO` in `install.sh`, or the package names in the Release workflow,
> **change both sides together**. Otherwise one-line install 404s and silently degrades to a
> `cargo` source build.

The install script still works without any Release: if it cannot fetch a prebuilt package it
falls back to `cargo install --locked --git ...`, just slower on first install.

## 4. Pre-launch checklist

```sh
# the install scripts are in the build output
cd web && VITE_BASE=/ npm run build
test -f dist/install.sh && test -f dist/install.ps1

# serve the output locally and actually run the command the site shows
npm run preview
curl -fsSL http://localhost:4173/install.sh | head -20

# the installer must be English by default
env -u LANG -u LC_MESSAGES LC_ALL=C sh dist/install.sh --help | grep -q '[一-龥]' \
  && echo 'BROKEN: Chinese in an English environment' || echo 'default language OK'
```

- [ ] `install.sh` and `install.ps1` are in the site root and return `text/plain`
- [ ] The domain shown in the install dialog is your real domain
- [ ] Releases contain the packages and `.sha256` files for each platform
- [ ] One-line install verified on at least one clean machine
- [ ] The site renders in English by default, and the sidebar language switch works
