//! 常用命令的子命令表：输入 `git ` 之后直接给出 status / log / commit 这些候选。
//!
//! 为什么内置一张表，而不是去解析 `--help`：
//! 解析各家 help 输出格式各异、还要 fork 进程等结果，补全菜单等不起。这张表
//! 覆盖日常真正会用到的那些子命令，剩下的交给历史记录（用过一次就会被召回）。
//!
//! 表按「命令链」索引，所以 `git remote ` 能给出 add / set-url，而
//! `git status ` 什么都不给——匹配要求整条链精确命中，不会在二级命令后面
//! 又把一级子命令列一遍。
//!
//! 用户可以用 `[completions.<命令>]` 配置段补充或覆盖，见 config。

/// 一条子命令。描述存中英两份，用的时候再按当前语言选——
/// 静态表里不能放 `t!()`，那是运行期才知道结果的。
pub struct SubCommand {
    pub name: &'static str,
    pub en: &'static str,
    pub zh: &'static str,
}

impl SubCommand {
    pub fn description(&self) -> &'static str {
        crate::i18n::t(self.en, self.zh)
    }
}

/// 简写宏，让下面的表读起来是三列而不是一堆字段名。
macro_rules! sub {
    ($name:expr, $en:expr, $zh:expr) => {
        SubCommand {
            name: $name,
            en: $en,
            zh: $zh,
        }
    };
}

/// 按命令链查子命令。`chain` 形如 `"git"` 或 `"git remote"`。
pub fn lookup(chain: &str) -> Option<&'static [SubCommand]> {
    TABLE
        .iter()
        .find(|(key, _)| *key == chain)
        .map(|(_, list)| *list)
}

/// 已覆盖的命令链数量，供测试与文档核对。
pub fn chains() -> impl Iterator<Item = &'static str> {
    TABLE.iter().map(|(key, _)| *key)
}

type Entry = (&'static str, &'static [SubCommand]);

static TABLE: &[Entry] = &[
    ("git", GIT),
    ("git remote", GIT_REMOTE),
    ("git stash", GIT_STASH),
    ("git submodule", GIT_SUBMODULE),
    ("git branch", GIT_BRANCH),
    ("git worktree", GIT_WORKTREE),
    ("git bisect", GIT_BISECT),
    ("cargo", CARGO),
    ("npm", NPM),
    ("pnpm", PNPM),
    ("yarn", YARN),
    ("docker", DOCKER),
    ("docker compose", DOCKER_COMPOSE),
    ("kubectl", KUBECTL),
    ("go", GO),
    ("systemctl", SYSTEMCTL),
    ("brew", BREW),
];

static GIT: &[SubCommand] = &[
    sub!("status", "show the working tree status", "查看工作区状态"),
    sub!("add", "stage file contents", "把改动加入暂存区"),
    sub!("commit", "record staged changes", "提交暂存区的改动"),
    sub!("push", "update remote refs", "推送到远端"),
    sub!("pull", "fetch and integrate", "拉取并合并远端改动"),
    sub!(
        "fetch",
        "download objects and refs",
        "只下载远端改动，不合并"
    ),
    sub!("log", "show commit history", "查看提交历史"),
    sub!("diff", "show changes", "查看改动内容"),
    sub!("switch", "switch branches", "切换分支"),
    sub!(
        "checkout",
        "switch branches or restore files",
        "切换分支或还原文件"
    ),
    sub!("branch", "list or manage branches", "管理分支"),
    sub!("merge", "join development histories", "合并分支"),
    sub!("rebase", "reapply commits on another base", "变基"),
    sub!("reset", "reset HEAD to a state", "重置 HEAD"),
    sub!("restore", "restore working tree files", "还原工作区文件"),
    sub!("stash", "stash changes away", "暂存现场"),
    sub!("tag", "create or list tags", "管理标签"),
    sub!("remote", "manage remotes", "管理远端"),
    sub!("clone", "clone a repository", "克隆仓库"),
    sub!("show", "show an object", "查看某个对象"),
    sub!("cherry-pick", "apply existing commits", "挑选提交"),
    sub!("revert", "revert existing commits", "回滚提交"),
    sub!("blame", "show who changed each line", "逐行追溯作者"),
    sub!("worktree", "manage multiple working trees", "管理多工作区"),
    sub!("bisect", "binary-search a bad commit", "二分查找坏提交"),
    sub!("submodule", "manage submodules", "管理子模块"),
    sub!("init", "create a repository", "初始化仓库"),
];

static GIT_REMOTE: &[SubCommand] = &[
    sub!("-v", "show remotes with URLs", "显示远端及地址"),
    sub!("add", "add a remote", "添加远端"),
    sub!("remove", "remove a remote", "删除远端"),
    sub!("rename", "rename a remote", "重命名远端"),
    sub!("set-url", "change a remote URL", "修改远端地址"),
    sub!("show", "show info about a remote", "查看远端详情"),
    sub!("prune", "drop stale remote refs", "清理失效的远端引用"),
];

static GIT_STASH: &[SubCommand] = &[
    sub!("push", "stash current changes", "把当前改动存起来"),
    sub!(
        "pop",
        "apply and drop the latest stash",
        "取出并删除最近一条"
    ),
    sub!("apply", "apply without dropping", "取出但保留记录"),
    sub!("list", "list stashes", "列出所有暂存"),
    sub!("show", "show a stash diff", "查看某条暂存的改动"),
    sub!("drop", "remove one stash", "删除一条暂存"),
    sub!("clear", "remove all stashes", "清空暂存"),
];

static GIT_SUBMODULE: &[SubCommand] = &[
    sub!("update", "update submodules", "更新子模块"),
    sub!("add", "add a submodule", "添加子模块"),
    sub!("init", "register submodules", "注册子模块"),
    sub!("status", "show submodule status", "查看子模块状态"),
    sub!("sync", "sync submodule URLs", "同步子模块地址"),
    sub!("foreach", "run a command in each", "在每个子模块里执行"),
];

static GIT_BRANCH: &[SubCommand] = &[
    sub!("-a", "list all branches", "列出全部分支"),
    sub!("-r", "list remote branches", "列出远端分支"),
    sub!("-d", "delete a merged branch", "删除已合并的分支"),
    sub!("-D", "force-delete a branch", "强制删除分支"),
    sub!("-m", "rename a branch", "重命名分支"),
    sub!("--merged", "list merged branches", "列出已合并的分支"),
];

static GIT_WORKTREE: &[SubCommand] = &[
    sub!("add", "add a working tree", "新建工作区"),
    sub!("list", "list working trees", "列出工作区"),
    sub!("remove", "remove a working tree", "删除工作区"),
    sub!("prune", "clean stale entries", "清理失效记录"),
];

static GIT_BISECT: &[SubCommand] = &[
    sub!("start", "start bisecting", "开始二分"),
    sub!("good", "mark as good", "标记为正常"),
    sub!("bad", "mark as bad", "标记为有问题"),
    sub!("skip", "skip this commit", "跳过这个提交"),
    sub!("reset", "finish bisecting", "结束二分"),
];

static CARGO: &[SubCommand] = &[
    sub!("build", "compile the package", "编译"),
    sub!("run", "build and run", "编译并运行"),
    sub!("test", "run the tests", "跑测试"),
    sub!("check", "type-check without building", "只做类型检查"),
    sub!("clippy", "run the linter", "跑 lint"),
    sub!("fmt", "format the code", "格式化代码"),
    sub!("add", "add a dependency", "添加依赖"),
    sub!("remove", "remove a dependency", "移除依赖"),
    sub!("update", "update the lockfile", "更新 lock 文件"),
    sub!("tree", "show the dependency tree", "查看依赖树"),
    sub!("bench", "run the benchmarks", "跑基准测试"),
    sub!("doc", "build the documentation", "生成文档"),
    sub!("clean", "remove build artifacts", "清理构建产物"),
    sub!("publish", "publish to the registry", "发布到 registry"),
    sub!("install", "install a binary", "安装二进制"),
];

static NPM: &[SubCommand] = &[
    sub!("install", "install dependencies", "安装依赖"),
    sub!(
        "ci",
        "clean install from the lockfile",
        "按 lock 文件干净安装"
    ),
    sub!("run", "run a package script", "运行项目脚本"),
    sub!("start", "run the start script", "启动"),
    sub!("test", "run the test script", "跑测试"),
    sub!("publish", "publish the package", "发布包"),
    sub!("update", "update dependencies", "更新依赖"),
    sub!("outdated", "check for outdated deps", "查看过期依赖"),
    sub!("audit", "audit for vulnerabilities", "安全审计"),
    sub!("exec", "run a package binary", "执行包里的命令"),
    sub!("link", "symlink a package", "软链本地包"),
    sub!("init", "create a package.json", "创建 package.json"),
];

static PNPM: &[SubCommand] = &[
    sub!("install", "install dependencies", "安装依赖"),
    sub!("add", "add a dependency", "添加依赖"),
    sub!("remove", "remove a dependency", "移除依赖"),
    sub!("run", "run a package script", "运行项目脚本"),
    sub!("dlx", "fetch and run a package", "临时下载并执行"),
    sub!("exec", "run a local binary", "执行本地命令"),
    sub!("update", "update dependencies", "更新依赖"),
    sub!("why", "explain why a package exists", "解释依赖来源"),
    sub!("outdated", "check for outdated deps", "查看过期依赖"),
];

static YARN: &[SubCommand] = &[
    sub!("install", "install dependencies", "安装依赖"),
    sub!("add", "add a dependency", "添加依赖"),
    sub!("remove", "remove a dependency", "移除依赖"),
    sub!("run", "run a package script", "运行项目脚本"),
    sub!("upgrade", "upgrade dependencies", "升级依赖"),
    sub!("why", "explain why a package exists", "解释依赖来源"),
];

static DOCKER: &[SubCommand] = &[
    sub!("ps", "list containers", "列出容器"),
    sub!("images", "list images", "列出镜像"),
    sub!("build", "build an image", "构建镜像"),
    sub!("run", "run a new container", "运行容器"),
    sub!("exec", "run a command in a container", "在容器里执行命令"),
    sub!("logs", "fetch container logs", "查看容器日志"),
    sub!("compose", "manage a compose project", "管理 compose 项目"),
    sub!("stop", "stop containers", "停止容器"),
    sub!("start", "start containers", "启动容器"),
    sub!("rm", "remove containers", "删除容器"),
    sub!("rmi", "remove images", "删除镜像"),
    sub!("pull", "pull an image", "拉取镜像"),
    sub!("push", "push an image", "推送镜像"),
    sub!("inspect", "show low-level info", "查看详细信息"),
    sub!("stats", "live resource usage", "实时资源占用"),
    sub!("system", "manage Docker itself", "管理 Docker 本身"),
    sub!("volume", "manage volumes", "管理卷"),
    sub!("network", "manage networks", "管理网络"),
];

static DOCKER_COMPOSE: &[SubCommand] = &[
    sub!("up", "create and start services", "创建并启动服务"),
    sub!("down", "stop and remove everything", "停止并清理"),
    sub!("ps", "list containers", "列出容器"),
    sub!("logs", "view service logs", "查看服务日志"),
    sub!("build", "build service images", "构建服务镜像"),
    sub!("restart", "restart services", "重启服务"),
    sub!("exec", "run a command in a service", "在服务里执行命令"),
    sub!("pull", "pull service images", "拉取服务镜像"),
];

static KUBECTL: &[SubCommand] = &[
    sub!("get", "list resources", "列出资源"),
    sub!("describe", "show resource details", "查看资源详情"),
    sub!("logs", "print pod logs", "查看 Pod 日志"),
    sub!("exec", "run a command in a pod", "在 Pod 里执行命令"),
    sub!("apply", "apply a manifest", "应用清单"),
    sub!("delete", "delete resources", "删除资源"),
    sub!("rollout", "manage a rollout", "管理发布"),
    sub!("scale", "change the replica count", "调整副本数"),
    sub!("port-forward", "forward a local port", "端口转发"),
    sub!("top", "show resource usage", "查看资源占用"),
    sub!("config", "manage kubeconfig", "管理 kubeconfig"),
    sub!("edit", "edit a resource", "编辑资源"),
];

static GO: &[SubCommand] = &[
    sub!("build", "compile packages", "编译"),
    sub!("run", "compile and run", "编译并运行"),
    sub!("test", "run the tests", "跑测试"),
    sub!("mod", "manage modules", "管理模块"),
    sub!("get", "add a dependency", "添加依赖"),
    sub!("install", "build and install", "编译并安装"),
    sub!("fmt", "format the code", "格式化代码"),
    sub!("vet", "report suspicious code", "静态检查"),
    sub!("generate", "run code generators", "执行代码生成"),
    sub!("work", "manage workspaces", "管理工作区"),
];

static SYSTEMCTL: &[SubCommand] = &[
    sub!("status", "show unit status", "查看服务状态"),
    sub!("start", "start a unit", "启动服务"),
    sub!("stop", "stop a unit", "停止服务"),
    sub!("restart", "restart a unit", "重启服务"),
    sub!("reload", "reload configuration", "重载配置"),
    sub!("enable", "enable at boot", "开机自启"),
    sub!("disable", "disable at boot", "取消自启"),
    sub!("list-units", "list loaded units", "列出已加载单元"),
    sub!(
        "daemon-reload",
        "reload systemd itself",
        "重载 systemd 本身"
    ),
];

static BREW: &[SubCommand] = &[
    sub!("install", "install a formula", "安装"),
    sub!("uninstall", "remove a formula", "卸载"),
    sub!("update", "update Homebrew", "更新 Homebrew"),
    sub!("upgrade", "upgrade formulae", "升级软件"),
    sub!("search", "search for formulae", "搜索"),
    sub!("info", "show formula info", "查看信息"),
    sub!("list", "list installed formulae", "列出已安装"),
    sub!("services", "manage background services", "管理后台服务"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_up_by_full_chain() {
        assert!(lookup("git").is_some());
        assert!(lookup("git remote").is_some());
        // 二级命令后面不该再给一级子命令，所以这里必须查不到
        assert!(lookup("git status").is_none());
        assert!(lookup("nonexistent-tool").is_none());
    }

    #[test]
    fn git_covers_the_everyday_commands() {
        let git = lookup("git").expect("git 表存在");
        for expected in ["status", "log", "commit", "diff", "push", "stash"] {
            assert!(
                git.iter().any(|s| s.name == expected),
                "git 子命令表缺少 {expected}"
            );
        }
    }

    /// 描述两种语言都得有，不然换个语言就出现空白条目。
    #[test]
    fn every_entry_has_both_languages() {
        for chain in chains() {
            for entry in lookup(chain).expect("表存在") {
                assert!(!entry.name.is_empty(), "{chain} 有空的子命令名");
                assert!(!entry.en.is_empty(), "{chain}/{} 缺英文描述", entry.name);
                assert!(!entry.zh.is_empty(), "{chain}/{} 缺中文描述", entry.name);
            }
        }
    }

    /// 同一条链里不该有重复项——重复会在菜单里显示两遍。
    #[test]
    fn no_duplicate_entries_within_a_chain() {
        for chain in chains() {
            let list = lookup(chain).expect("表存在");
            for (index, entry) in list.iter().enumerate() {
                assert!(
                    !list[..index].iter().any(|e| e.name == entry.name),
                    "{chain} 里 {} 重复了",
                    entry.name
                );
            }
        }
    }

    /// 多词链的父命令必须也在表里，否则 `git remote` 有表、`git` 没表，
    /// 用户根本走不到二级。
    #[test]
    fn parent_chains_exist() {
        for chain in chains() {
            if let Some((parent, _)) = chain.rsplit_once(' ') {
                assert!(lookup(parent).is_some(), "{chain} 的父链 {parent} 不在表里");
            }
        }
    }
}
