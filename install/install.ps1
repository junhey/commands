<#
.SYNOPSIS
    Commands (cmds) installer for Windows.

.DESCRIPTION
    Detects the platform, prefers a prebuilt package from GitHub Releases and
    verifies it against the matching .sha256; falls back to building from source
    with cargo when no prebuilt package matches. Installs for the current user,
    no administrator rights needed.

    Messages are English by default and switch to Chinese on a Chinese system.
    Set CMDS_LANG=zh or CMDS_LANG=en to force one.

.PARAMETER BinDir
    Install directory. Defaults to %LOCALAPPDATA%\Programs\cmds

.PARAMETER Version
    Version to install, e.g. v0.2.0. Defaults to latest.

.PARAMETER Force
    Overwrite an existing installation without warning.

.PARAMETER Build
    Skip the download and build from source with cargo.

.PARAMETER SkipVerify
    Skip the SHA-256 check (not recommended).

.EXAMPLE
    irm https://junhey.github.io/commands/install.ps1 | iex

.EXAMPLE
    .\install.ps1 -BinDir "$env:LOCALAPPDATA\Programs\cmds" -Version v0.2.0
#>
[CmdletBinding()]
param(
    [string]$BinDir = "$env:LOCALAPPDATA\Programs\cmds",
    [string]$Version = 'latest',
    [switch]$Force,
    [switch]$Build,
    [switch]$SkipVerify
)

$ErrorActionPreference = 'Stop'
$Repo = 'junhey/commands'

# ── Language ────────────────────────────────────────────────────────────
# English unless the system (or CMDS_LANG) asks for Chinese.
if ($env:CMDS_LANG) {
    $script:UseZh = $env:CMDS_LANG -like 'zh*'
} else {
    $script:UseZh = ((Get-UICulture).Name -like 'zh*') -or ((Get-Culture).Name -like 'zh*')
}

function Msg {
    param([string]$En, [string]$Zh)
    if ($script:UseZh) { $Zh } else { $En }
}

function Write-Info { param($Message) Write-Host "▸ $Message" -ForegroundColor DarkGray }
function Write-Ok { param($Message) Write-Host "✓ $Message" -ForegroundColor Green }
function Write-Warn2 { param($Message) Write-Host "! $Message" -ForegroundColor Yellow }
function Write-Err { param($Message) Write-Host "✗ $Message" -ForegroundColor Red }

function Get-Target {
    $arch = switch ($env:PROCESSOR_ARCHITECTURE) {
        'AMD64' { 'x86_64' }
        'ARM64' { 'aarch64' }
        'x86' { 'i686' }
        default { 'x86_64' }
    }
    "$arch-pc-windows-msvc"
}

# Verify the downloaded archive. Every release asset ships a matching .sha256.
# A missing checksum file only warns; a mismatch aborts the install.
function Test-Checksum {
    param([string]$ArchivePath, [string]$ChecksumUrl, [string]$TempDir)

    if ($SkipVerify) {
        Write-Warn2 (Msg 'skipping verification as requested' '已按要求跳过校验')
        return $true
    }

    $checksumFile = Join-Path $TempDir 'checksum.txt'
    try {
        Invoke-WebRequest -Uri $ChecksumUrl -OutFile $checksumFile -UseBasicParsing
    } catch {
        Write-Warn2 (Msg 'no checksum file available, skipping verification' '没有取到校验文件，跳过校验')
        return $true
    }

    # Format is "digest  filename"; sha256sum on Windows writes "digest *filename".
    # Either way the first field is what we want.
    $expected = ((Get-Content $checksumFile -Raw).Trim() -split '\s+')[0]
    if (-not $expected) {
        Write-Warn2 (Msg 'checksum file looks malformed, skipping verification' '校验文件内容异常，跳过校验')
        return $true
    }

    $actual = (Get-FileHash -Path $ArchivePath -Algorithm SHA256).Hash
    if ($expected -ine $actual) {
        Write-Err (Msg 'SHA-256 mismatch, install aborted' 'SHA-256 校验失败，已放弃安装')
        Write-Err ("  " + (Msg 'expected: ' '期望：') + $expected)
        Write-Err ("  " + (Msg 'actual:   ' '实际：') + $actual)
        return $false
    }
    Write-Ok (Msg 'SHA-256 verified' 'SHA-256 校验通过')
    return $true
}

function Install-FromRelease {
    param([string]$Target, [string]$Destination)

    $archive = "cmds-$Target.zip"
    $baseUrl = if ($Version -eq 'latest') {
        "https://github.com/$Repo/releases/latest/download"
    } else {
        "https://github.com/$Repo/releases/download/$Version"
    }
    $url = "$baseUrl/$archive"

    $temp = Join-Path $env:TEMP "cmds-install-$([System.Guid]::NewGuid().ToString('N'))"
    New-Item -ItemType Directory -Path $temp -Force | Out-Null
    $zip = Join-Path $temp $archive

    Write-Info ((Msg 'downloading ' '下载 ') + $url)
    try {
        Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
    } catch {
        Remove-Item $temp -Recurse -Force -ErrorAction SilentlyContinue
        return $false
    }

    if (-not (Test-Checksum -ArchivePath $zip -ChecksumUrl "$url.sha256" -TempDir $temp)) {
        Remove-Item $temp -Recurse -Force -ErrorAction SilentlyContinue
        return $false
    }

    Expand-Archive -Path $zip -DestinationPath $temp -Force
    $binary = Get-ChildItem -Path $temp -Recurse -Filter 'cmds.exe' | Select-Object -First 1
    if (-not $binary) {
        Write-Err (Msg 'cmds.exe was not found inside the archive' '压缩包里没有找到 cmds.exe')
        Remove-Item $temp -Recurse -Force -ErrorAction SilentlyContinue
        return $false
    }

    New-Item -ItemType Directory -Path $Destination -Force | Out-Null
    Copy-Item $binary.FullName (Join-Path $Destination 'cmds.exe') -Force
    Remove-Item $temp -Recurse -Force -ErrorAction SilentlyContinue
    return $true
}

function Install-FromSource {
    param([string]$Destination)

    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Err (Msg `
            'no prebuilt package and no cargo; install Rust first: https://rustup.rs' `
            '没有预编译包，也没有 cargo；请先安装 Rust：https://rustup.rs')
        return $false
    }
    $root = Split-Path $Destination -Parent

    # Prefer crates.io: proper version semantics, no need to clone the repo.
    if ($Version -eq 'latest') {
        Write-Info (Msg 'installing from crates.io (first build takes about a minute)' `
            '从 crates.io 安装（首次编译约 1 分钟）')
        cargo install --locked --root $root cmds
    } else {
        Write-Info ((Msg 'installing from crates.io: ' '从 crates.io 安装 ') + $Version + `
            (Msg ' (first build takes about a minute)' '（首次编译约 1 分钟）'))
        # --version takes vX.Y.Z but cargo wants X.Y.Z
        cargo install --locked --root $root --version $Version.TrimStart('v') cmds
    }
    if ($LASTEXITCODE -eq 0) { return $true }

    # That version may not be on crates.io yet; fall back to the repository.
    Write-Warn2 (Msg 'crates.io install did not succeed, building from the repository' `
        'crates.io 安装未成功，改从仓库源码构建')
    cargo install --locked --git "https://github.com/$Repo" --root $root cmds
    return $LASTEXITCODE -eq 0
}

Write-Host ''
Write-Host 'Commands' -ForegroundColor Cyan -NoNewline
Write-Host (' — ' + (Msg 'a small, fast interactive shell' '轻量高效的交互式终端'))
Write-Host (Msg 'history autosuggestions · Tab candidate menu · starship-style prompt' `
    '历史自动建议 · Tab 候选菜单 · starship 风格提示符') -ForegroundColor DarkGray
Write-Host ''

$target = Get-Target
Write-Info ((Msg 'platform: ' '平台：') + $target)
Write-Info ((Msg 'install directory: ' '安装目录：') + $BinDir)

$exe = Join-Path $BinDir 'cmds.exe'
if ((Test-Path $exe) -and -not $Force) {
    Write-Warn2 ($exe + (Msg ' already exists and will be replaced' ' 已存在，将覆盖升级'))
}

$installed = $false
if ($Build) {
    $installed = Install-FromSource -Destination $BinDir
} else {
    $installed = Install-FromRelease -Target $target -Destination $BinDir
    if (-not $installed) {
        Write-Warn2 (Msg 'no prebuilt package for this platform, building from source' `
            '没有对应平台的预编译包，改为源码构建')
        $installed = Install-FromSource -Destination $BinDir
    }
}

if (-not $installed -or -not (Test-Path $exe)) {
    Write-Err (Msg 'install failed' '安装失败')
    exit 1
}

Write-Ok ((Msg 'installed to ' '已安装到 ') + $exe)
& $exe --version

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($userPath -notlike "*$BinDir*") {
    [Environment]::SetEnvironmentVariable('Path', "$BinDir;$userPath", 'User')
    Write-Ok ((Msg 'added ' '已把 ') + $BinDir + `
        (Msg ' to your user PATH (takes effect in a new terminal)' ' 加入用户 PATH（新开终端生效）'))
}

Write-Host ''
# 收尾这段整块分语言写：句子里穿插了命令和配色，逐词拼接容易漏掉空格。
if ($script:UseZh) {
    Write-Host '接下来' -ForegroundColor White
    Write-Host '  • 直接启动：' -NoNewline; Write-Host 'cmds' -ForegroundColor Cyan -NoNewline; Write-Host '（输入 help 查看全部快捷键）'
    Write-Host '  • 生成配置模板：' -NoNewline; Write-Host 'cmds config init' -ForegroundColor Cyan
    Write-Host '  • 只想用提示符：' -NoNewline; Write-Host 'Invoke-Expression (& cmds init powershell | Out-String)' -ForegroundColor Cyan
    Write-Host ''
    Write-Host '文档：https://junhey.github.io/commands/#guide' -ForegroundColor Blue
} else {
    Write-Host 'Next' -ForegroundColor White
    Write-Host '  • Start it: ' -NoNewline; Write-Host 'cmds' -ForegroundColor Cyan -NoNewline; Write-Host ' (type help for every keybinding)'
    Write-Host '  • Write a config template: ' -NoNewline; Write-Host 'cmds config init' -ForegroundColor Cyan
    Write-Host '  • Only want the prompt: ' -NoNewline; Write-Host 'Invoke-Expression (& cmds init powershell | Out-String)' -ForegroundColor Cyan
    Write-Host ''
    Write-Host 'Docs: https://junhey.github.io/commands/#guide' -ForegroundColor Blue
}
Write-Host ''
