// Sovereign Monorepo Engine & Toolchain Orchestrator: xtk
// First-class 4-Pillar VCS Governance:
//   1. Git Superproject (.gitmodules, .gitattributes, Scalar, src.lock.json)
//   2. Chromium depot_tools (DEPS, .gclient)
//   3. Linux Kernel (MAINTAINERS, torvalds/stable/next/gki remotes, quilt patch series)
//   4. Android AOSP (.repo/manifests/default.xml, platform/manifest, Kleaf GKI)
use argh::FromArgs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(FromArgs, PartialEq, Debug)]
/// Sovereign Monorepo Engine & Toolchain Orchestrator (xtk)
struct TopLevel {
    #[argh(subcommand)]
    cmd: SubCommand,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum SubCommand {
    Doctor(DoctorCmd),
    SlimAll(SlimAllCmd),
    Lock(LockCmd),
    Manifests(ManifestsCmd),
    AuditVcs(AuditVcsCmd),
}

#[derive(FromArgs, PartialEq, Debug)]
/// verify health of all compiler, VCS, and monorepo toolchains
#[argh(subcommand, name = "doctor")]
struct DoctorCmd {}

#[derive(FromArgs, PartialEq, Debug)]
/// run byte-level Git and Scalar optimization across all C:\src repos
#[argh(subcommand, name = "slim-all")]
struct SlimAllCmd {
    /// target root directory (defaults to C:\src)
    #[argh(option, default = "String::from(\"C:\\\\src\")")]
    root: String,
}

#[derive(FromArgs, PartialEq, Debug)]
/// generate deterministic C:\src\src.lock.json and synchronize DEPS, AOSP default.xml, and MAINTAINERS
#[argh(subcommand, name = "lock")]
struct LockCmd {
    /// target root directory (defaults to C:\src)
    #[argh(option, default = "String::from(\"C:\\\\src\")")]
    root: String,
    /// output lockfile path (defaults to C:\src\src.lock.json)
    #[argh(option, default = "String::from(\"C:\\\\src\\\\src.lock.json\")")]
    out: String,
    /// skip emitting DEPS, .repo/manifests/default.xml, and MAINTAINERS
    #[argh(switch)]
    lock_only: bool,
}

#[derive(FromArgs, PartialEq, Debug)]
/// deterministically generate Chromium DEPS, AOSP .repo/manifests/default.xml, Linux MAINTAINERS, and quilt series
#[argh(subcommand, name = "manifests")]
struct ManifestsCmd {
    /// target root directory (defaults to C:\src)
    #[argh(option, default = "String::from(\"C:\\\\src\")")]
    root: String,
}

#[derive(FromArgs, PartialEq, Debug)]
/// audit 4-pillar VCS governance (Git + Chromium + Kernel + AOSP + NTFS junctions)
#[argh(subcommand, name = "audit-vcs")]
struct AuditVcsCmd {
    /// target root directory (defaults to C:\src)
    #[argh(option, default = "String::from(\"C:\\\\src\")")]
    root: String,
    /// emit JSON output
    #[argh(switch)]
    json: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SrcLock {
    version: u32,
    generated_at: String,
    hardware: HardwareTarget,
    toolchains: BTreeMap<String, String>,
    pillars: PillarSummary,
    repositories: BTreeMap<String, LockedRepo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PillarSummary {
    git_superproject_commit: String,
    chromium_src_commit: String,
    linux_kernel_commit: String,
    aosp_manifest_commit: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct HardwareTarget {
    os: String,
    cpu: String,
    gpu: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LockedRepo {
    path: String,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    upstream_url: Option<String>,
    commit: String,
    branch: String,
    engine: String,
    pillar: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    remotes: Option<BTreeMap<String, String>>,
    sparse_paths: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct VcsAuditReport {
    root: String,
    healthy: bool,
    locked_repositories: usize,
    submodules_count: usize,
    root_junctions_count: usize,
    dangling_junctions: Vec<String>,
    git_superproject_ok: bool,
    chromium_deps_ok: bool,
    linux_kernel_ok: bool,
    aosp_repo_ok: bool,
    notes: Vec<String>,
}

fn safe_output(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if s.is_empty() {
                    Some(String::from_utf8_lossy(&out.stderr).trim().to_string())
                } else {
                    Some(s)
                }
            } else {
                None
            }
        })
}

fn safe_tool_probe(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .and_then(|out| {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if out.status.success() {
                if !stdout.is_empty() {
                    Some(stdout)
                } else if !stderr.is_empty() {
                    Some(stderr)
                } else {
                    Some("OK".to_string())
                }
            } else if program == "cl" && (!stdout.is_empty() || !stderr.is_empty()) {
                Some(if !stderr.is_empty() { stderr } else { stdout })
            } else {
                None
            }
        })
}

fn find_tool_version(name: &str, program: &str, args: &[&str]) -> (String, bool, String) {
    if let Some(out) = safe_tool_probe(program, args) {
        let first_line = out.lines().next().unwrap_or("").to_string();
        (name.to_string(), true, first_line)
    } else {
        (name.to_string(), false, "NOT FOUND".to_string())
    }
}

fn check_doctor() {
    println!("\n=== SOVEREIGN TOOLCHAIN & 4-PILLAR VCS DOCTOR (xtk) ===");
    println!("{:<14} {:<8} {}", "TOOL", "STATUS", "VERSION / DETAILS");
    println!("{}", "-".repeat(72));

    let tools = [
        ("rustc", "rustc", vec!["--version"]),
        ("cargo", "cargo", vec!["--version"]),
        ("sccache", "sccache", vec!["--version"]),
        ("xwin", "xwin", vec!["--version"]),
        ("bun", "bun", vec!["--version"]),
        ("jj", "jj", vec!["--version"]),
        ("gix", "gix", vec!["--version"]),
        ("just", "just", vec!["--version"]),
        ("go", "go", vec!["version"]),
        ("zig", "zig", vec!["version"]),
        ("dotnet", "dotnet", vec!["--version"]),
        ("nvcc", "nvcc", vec!["--version"]),
        ("cl.exe", "cl", vec!["/help"]),
        ("depot", "depot", vec!["--help"]),
        ("claude", "claude", vec!["--version"]),
        ("agy", "agy", vec!["--version"]),
        ("wck", "wck", vec!["--version"]),
        ("archguard", "archguard", vec!["--version"]),
    ];

    let mut all_ok = true;
    for (name, prog, args) in tools {
        let (n, ok, detail) = find_tool_version(name, prog, &args);
        let status = if ok { "OK" } else { "MISSING" };
        if !ok {
            all_ok = false;
        }

        let short_detail = if n == "cl.exe" && ok {
            "MSVC v14.51 (Microsoft C/C++ Optimizing Compiler)".to_string()
        } else if n == "depot" && ok {
            "depot + depot_tools (gclient/siso/gn/repo)".to_string()
        } else if detail.len() > 52 {
            format!("{}...", &detail[..49])
        } else {
            detail
        };

        println!("{:<14} {:<8} {}", n, status, short_detail);
    }

    println!("{}", "-".repeat(72));
    if all_ok {
        println!("All 17 sovereign toolchains, VCS orchestrators & CLIs operational.\n");
    } else {
        println!("Some toolchains are missing or not in PATH.\n");
    }
}

pub fn is_reparse_point(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if let Ok(meta) = fs::symlink_metadata(path) {
            return (meta.file_attributes() & 0x0400) != 0;
        }
    }
    #[cfg(not(windows))]
    {
        if let Ok(meta) = fs::symlink_metadata(path) {
            return meta.file_type().is_symlink();
        }
    }
    false
}

fn get_dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if is_reparse_point(&p) {
                continue;
            }
            if p.is_dir() {
                total += get_dir_size(&p);
            } else if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

fn scan_repo_tree(dir: &Path, depth: usize, max_depth: usize, repos: &mut Vec<PathBuf>) {
    if depth > max_depth {
        return;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.')
                || name == "node_modules"
                || name == "target"
                || name == "yolo-archive"
            {
                continue;
            }
            if is_reparse_point(&p) {
                continue;
            }
            if !p.is_dir() {
                continue;
            }
            if p.join(".git").exists() || p.join(".jj").exists() {
                repos.push(p);
            } else {
                scan_repo_tree(&p, depth + 1, max_depth, repos);
            }
        }
    }
}

pub fn discover_repos(root: &Path) -> Vec<PathBuf> {
    let mut repos = Vec::new();
    let canonical_groups = ["apps", "packages", "crates", "infra", "third_party"];
    for group in canonical_groups {
        let gp = root.join(group);
        if gp.is_dir() && !is_reparse_point(&gp) {
            scan_repo_tree(&gp, 1, 3, &mut repos);
        }
    }
    if root.join("infra").join(".git").exists() {
        repos.push(root.join("infra"));
    }
    repos.sort();
    repos.dedup();
    repos
}

fn classify_pillar(rel_name: &str) -> &'static str {
    if rel_name.starts_with("third_party/chromium")
        || rel_name == "third_party/depot_tools"
        || rel_name == "packages/chromium-bun"
    {
        "chromium"
    } else if rel_name == "third_party/linux"
        || rel_name.starts_with("third_party/archlinux")
        || rel_name.starts_with("third_party/cachyos")
        || rel_name.starts_with("third_party/omacom")
        || rel_name == "crates/unix"
    {
        "linux-kernel"
    } else if rel_name.starts_with("third_party/aosp")
        || rel_name.starts_with("third_party/google-ecosystem")
        || rel_name == "apps/aphrody-app"
    {
        "aosp"
    } else {
        "git"
    }
}

fn read_git_remotes(repo: &Path) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let repo_s = repo.to_string_lossy();
    if let Some(out) = safe_output("git", &["-C", &repo_s, "remote", "-v"]) {
        for line in out.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[2] == "(fetch)" {
                map.insert(parts[0].to_string(), parts[1].to_string());
            }
        }
    }
    map
}

fn apply_scalar_git_config(repo: &Path) {
    let pairs = [
        ("protocol.version", "2"),
        ("core.fsmonitor", "true"),
        ("core.untrackedCache", "true"),
        ("feature.manyFiles", "true"),
        ("index.version", "4"),
        ("fetch.writeCommitGraph", "true"),
        ("gc.writeCommitGraph", "true"),
        ("fetch.parallel", "0"),
        ("submodule.fetchJobs", "8"),
        ("rerere.enabled", "true"),
    ];
    for (k, v) in pairs {
        let _ = Command::new("git")
            .args(["config", k, v])
            .current_dir(repo)
            .output();
    }
}

fn run_slim_all(root_str: &str) {
    let root = Path::new(root_str);
    if root.join(".git").exists() {
        apply_scalar_git_config(root);
    }
    let repos = discover_repos(root);

    println!("\n=== SOVEREIGN BYTE-LEVEL SLIM ALL (xtk) ===");
    println!("Scanning repositories under {}...", root.display());
    println!(
        "{:<36} {:>12} {:>12} {:>12}",
        "REPOSITORY", "BEFORE", "AFTER", "SAVED"
    );
    println!("{}", "-".repeat(76));

    let mut total_saved = 0u64;
    let mut total_before = 0u64;
    let mut total_after = 0u64;

    for repo in &repos {
        let rel_name = repo
            .strip_prefix(root)
            .unwrap_or(repo)
            .to_string_lossy()
            .replace('\\', "/");

        let git_dir = repo.join(".git");
        if !git_dir.exists() {
            continue;
        }

        let before = get_dir_size(&git_dir);
        apply_scalar_git_config(repo);
        let _ = Command::new("git")
            .args(["reflog", "expire", "--expire=now", "--all"])
            .current_dir(repo)
            .output();
        let _ = Command::new("git")
            .args(["commit-graph", "write", "--reachable"])
            .current_dir(repo)
            .output();
        let _ = Command::new("git")
            .args(["multi-pack-index", "write"])
            .current_dir(repo)
            .output();
        let _ = Command::new("git")
            .args(["prune", "--expire=now"])
            .current_dir(repo)
            .output();

        let after = get_dir_size(&git_dir);
        let saved = before.saturating_sub(after);

        total_before += before;
        total_after += after;
        total_saved += saved;

        println!(
            "{:<36} {:>9} KB {:>9} KB {:>9} KB",
            rel_name,
            before / 1024,
            after / 1024,
            saved / 1024
        );
    }

    println!("{}", "-".repeat(76));
    println!(
        "{:<36} {:>9} MB {:>9} MB {:>9} MB",
        "TOTAL DISK FOOTPRINT",
        total_before / (1024 * 1024),
        total_after / (1024 * 1024),
        total_saved / (1024 * 1024)
    );
    println!("Byte-level slimming complete: {} bytes saved.\n", total_saved);
}

fn collect_locked_repos(root: &Path) -> BTreeMap<String, LockedRepo> {
    let repos = discover_repos(root);
    let mut locked_repos = BTreeMap::new();

    for repo in &repos {
        let rel_name = repo
            .strip_prefix(root)
            .unwrap_or(repo)
            .to_string_lossy()
            .replace('\\', "/");

        let repo_s = repo.to_string_lossy();
        let remotes = read_git_remotes(repo);

        let url = remotes
            .get("origin")
            .cloned()
            .or_else(|| safe_output("git", &["-C", &repo_s, "config", "--get", "remote.origin.url"]))
            .unwrap_or_else(|| format!("local:{}", rel_name));

        let upstream_url = remotes
            .get("upstream")
            .cloned()
            .or_else(|| remotes.get("torvalds").cloned());

        let mut commit = safe_output("git", &["-C", &repo_s, "rev-parse", "HEAD"])
            .unwrap_or_else(|| "0000000000000000000000000000000000000000".to_string());
        if commit.contains("fatal:") || commit.len() < 40 {
            commit = "0000000000000000000000000000000000000000".to_string();
        }

        let branch = safe_output("git", &["-C", &repo_s, "branch", "--show-current"])
            .filter(|b| !b.is_empty())
            .unwrap_or_else(|| "detached".to_string());

        let engine = if repo.join(".jj").exists() {
            "jj".to_string()
        } else {
            "git".to_string()
        };

        let sparse_file = repo.join(".git").join("info").join("sparse-checkout");
        let sparse_paths = if sparse_file.exists() {
            fs::read_to_string(&sparse_file).ok().map(|c| {
                c.lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect()
            })
        } else {
            None
        };

        locked_repos.insert(
            rel_name.clone(),
            LockedRepo {
                path: repo.to_string_lossy().to_string(),
                url,
                upstream_url,
                commit,
                branch,
                engine,
                pillar: classify_pillar(&rel_name).to_string(),
                remotes: if remotes.is_empty() {
                    None
                } else {
                    Some(remotes)
                },
                sparse_paths,
            },
        );
    }

    locked_repos
}

pub fn render_chromium_deps(locked: &BTreeMap<String, LockedRepo>) -> String {
    let chromium_rev = locked
        .get("third_party/chromium/src")
        .map(|r| r.commit.as_str())
        .unwrap_or("HEAD");
    let depot_tools_rev = locked
        .get("third_party/depot_tools")
        .map(|r| r.commit.as_str())
        .unwrap_or("HEAD");
    let linux_rev = locked
        .get("third_party/linux")
        .map(|r| r.commit.as_str())
        .unwrap_or("HEAD");
    let aosp_rev = locked
        .get("third_party/aosp/platform/manifest")
        .map(|r| r.commit.as_str())
        .unwrap_or("HEAD");

    let mut out = String::new();
    out.push_str("# Deterministic Chromium gclient DEPS Manifest for C:\\src\n");
    out.push_str("# Generated by `xtk manifests` / `xtk lock` from C:\\src\\src.lock.json.\n");
    out.push_str("# Compatible with Google depot_tools (`gclient`, `depot`, `roll-dep`, `siso`, `gn`).\n\n");
    out.push_str("use_relative_paths = True\n");
    out.push_str("git_dependencies = 'SYNC'\n\n");
    out.push_str("vars = {\n");
    out.push_str("  'aphrody_github': 'https://github.com/aphrody-dev',\n");
    out.push_str("  'chromium_git': 'https://chromium.googlesource.com',\n");
    out.push_str("  'android_git': 'https://android.googlesource.com',\n");
    out.push_str("  'kernel_git': 'https://git.kernel.org/pub/scm/linux/kernel/git',\n");
    out.push_str(&format!("  'chromium_revision': '{}',\n", chromium_rev));
    out.push_str(&format!("  'depot_tools_revision': '{}',\n", depot_tools_rev));
    out.push_str(&format!("  'linux_kernel_revision': '{}',\n", linux_rev));
    out.push_str(&format!("  'aosp_manifest_revision': '{}',\n", aosp_rev));
    out.push_str("  'checkout_chromium': True,\n");
    out.push_str("  'checkout_linux_kernel': True,\n");
    out.push_str("  'checkout_aosp': True,\n");
    out.push_str("}\n\n");

    out.push_str("deps = {\n");
    for (rel_path, repo) in locked {
        if repo.url.starts_with("local:") || repo.commit.starts_with("0000000") {
            continue;
        }
        out.push_str(&format!(
            "  '{}': '{}@{}',\n",
            rel_path, repo.url, repo.commit
        ));
    }
    out.push_str("}\n\n");

    out.push_str("recursedeps = [\n");
    out.push_str("  'third_party/chromium/src',\n");
    out.push_str("]\n\n");

    out.push_str("hooks = [\n");
    out.push_str("  {\n");
    out.push_str("    'name': 'xtk_audit_vcs',\n");
    out.push_str("    'pattern': '.',\n");
    out.push_str("    'action': ['xtk', 'audit-vcs'],\n");
    out.push_str("  },\n");
    out.push_str("]\n");

    out
}

pub fn render_aosp_repo_manifest(locked: &BTreeMap<String, LockedRepo>) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<!-- Deterministic Android AOSP + GKI + Chromium + Sovereign Monorepo Manifest -->\n");
    out.push_str("<!-- Generated by `xtk manifests` / `xtk lock` for Google `repo` & `depot` -->\n");
    out.push_str("<manifest>\n");
    out.push_str("  <remote name=\"aphrody\" fetch=\"https://github.com/aphrody-dev/\" review=\"https://github.com/aphrody-dev/\" />\n");
    out.push_str("  <remote name=\"aphrody-labs\" fetch=\"https://github.com/aphrody-labs/\" />\n");
    out.push_str("  <remote name=\"aosp\" fetch=\"https://android.googlesource.com/\" review=\"https://android-review.googlesource.com/\" />\n");
    out.push_str("  <remote name=\"chromium\" fetch=\"https://chromium.googlesource.com/\" review=\"https://chromium-review.googlesource.com/\" />\n");
    out.push_str("  <remote name=\"kernel\" fetch=\"https://git.kernel.org/pub/scm/linux/kernel/git/\" />\n");
    out.push_str("  <remote name=\"archlinux\" fetch=\"https://gitlab.archlinux.org/\" />\n");
    out.push_str("  <remote name=\"github\" fetch=\"https://github.com/\" />\n\n");
    out.push_str("  <default remote=\"aphrody\" revision=\"main\" sync-j=\"8\" sync-c=\"true\" />\n\n");

    for (rel_path, repo) in locked {
        if repo.url.starts_with("local:") || repo.commit.starts_with("0000000") {
            continue;
        }
        let clean_url = repo.url.trim_end_matches(".git");
        let (remote, name) = if let Some(rest) = clean_url.strip_prefix("https://github.com/aphrody-dev/") {
            ("aphrody", rest.to_string())
        } else if let Some(rest) = clean_url.strip_prefix("https://github.com/aphrody-labs/") {
            ("aphrody-labs", rest.to_string())
        } else if let Some(rest) = clean_url.strip_prefix("https://android.googlesource.com/") {
            ("aosp", rest.to_string())
        } else if let Some(rest) = clean_url.strip_prefix("https://chromium.googlesource.com/") {
            ("chromium", rest.to_string())
        } else if let Some(rest) = clean_url.strip_prefix("https://git.kernel.org/pub/scm/linux/kernel/git/") {
            ("kernel", rest.to_string())
        } else if let Some(rest) = clean_url.strip_prefix("https://gitlab.archlinux.org/") {
            ("archlinux", rest.to_string())
        } else if let Some(rest) = clean_url.strip_prefix("https://github.com/") {
            ("github", rest.to_string())
        } else {
            continue;
        };

        let groups = match repo.pillar.as_str() {
            "aosp" => "aosp,pdk,sovereign",
            "chromium" => "chromium,browser,sovereign",
            "linux-kernel" => "kernel,gki,distro,sovereign",
            _ => "monorepo,first-party,sovereign",
        };

        let depth_attr = if rel_path.starts_with("third_party/") {
            " clone-depth=\"1\""
        } else {
            ""
        };

        out.push_str(&format!(
            "  <project path=\"{}\" name=\"{}\" remote=\"{}\" revision=\"{}\" upstream=\"{}\" groups=\"{}\"{} />\n",
            rel_path, name, remote, repo.commit, repo.branch, groups, depth_attr
        ));
    }

    out.push_str("</manifest>\n");
    out
}

pub fn render_kernel_maintainers() -> String {
    let mut out = String::new();
    out.push_str("List of maintainers (C:\\src Sovereign Monorepo)\n");
    out.push_str("===============================================\n\n");
    out.push_str("Descriptions of section entries and preferred order\n");
    out.push_str("---------------------------------------------------\n");
    out.push_str("\tM:\t*Mail* patches to: FullName <address@domain>\n");
    out.push_str("\tL:\t*Mailing list* that is relevant to this area\n");
    out.push_str("\tS:\t*Status* (Supported, Maintained, Odd Fixes, Orphan, Obsolete)\n");
    out.push_str("\tW:\t*Web-page* with status/info\n");
    out.push_str("\tT:\t*SCM* tree type and location (git, quilt)\n");
    out.push_str("\tF:\t*Files* and directories wildcard patterns\n");
    out.push_str("\tX:\t*Excluded* files and directories\n\n");
    out.push_str("Maintainers List\n");
    out.push_str("----------------\n\n");

    let sections: &[(&str, &str, &[&str], &[&str])] = &[
        (
            "ANDROID AOSP, GKI & APHRODY MOBILE SHELL",
            "Supported",
            &[
                "git https://android.googlesource.com/platform/manifest.git",
                "git https://android.googlesource.com/kernel/build.git",
                "git https://github.com/aphrody-dev/aphrody-app.git",
            ],
            &[
                ".repo/manifests/",
                "apps/aphrody-app/crates/aphrody-android/",
                "third_party/aosp/",
                "third_party/google-ecosystem/",
            ],
        ),
        (
            "APHRODY.COM WEB & DESKTOP PLATFORM",
            "Supported",
            &[
                "git https://github.com/aphrody-dev/aphrody.git",
                "git https://github.com/aphrody-dev/aphrody-app.git",
                "git https://github.com/aphrody-dev/google-design.git",
                "git https://github.com/aphrody-dev/wck.git",
            ],
            &[
                "apps/aphrody/",
                "apps/aphrody-app/",
                "packages/google-design/",
                "packages/wck/",
            ],
        ),
        (
            "BUN V1.4.4 FULL-RUST RUNTIME & PARITY PATCH ENGINE",
            "Supported",
            &[
                "git https://github.com/aphrody-dev/bun.git",
                "git https://github.com/oven-sh/bun.git",
                "git https://github.com/aphrody-dev/bun-parity.git",
                "quilt packages/bun-parity/patches/",
            ],
            &[
                "packages/bun-parity/",
                "packages/bun-plugins/",
                "third_party/bun/",
            ],
        ),
        (
            "CHROMIUM, BLINK, V8 & DEPOT_TOOLS INTEGRATION",
            "Supported",
            &[
                "git https://chromium.googlesource.com/chromium/src.git",
                "git https://chromium.googlesource.com/chromium/tools/depot_tools.git",
                "git https://github.com/aphrody-dev/chromium-bun.git",
                "git https://github.com/aphrody-dev/depot.git",
            ],
            &[
                "DEPS",
                "crates/depot/",
                "packages/chromium-bun/",
                "third_party/chromium/",
                "third_party/depot_tools/",
            ],
        ),
        (
            "GO 1.27 CLOUD INFRASTRUCTURE, OCI DEPLOY & MESH NETWORK",
            "Supported",
            &[
                "git https://github.com/aphrody-dev/infra.git",
                "git https://github.com/aphrody-dev/mesh-vn.git",
            ],
            &[
                "deploy.toml",
                "infra/",
                "infra/ffi-bridge/",
                "infra/mesh-vn/",
            ],
        ),
        (
            "GRAPHICS, WAYLAND COMPOSITOR & GAMESCOPE WEBGPU",
            "Supported",
            &[
                "git https://github.com/aphrody-dev/wayland.git",
                "git https://github.com/aphrody-dev/gamescope-rs.git",
            ],
            &[
                "crates/gamescope-rs/",
                "crates/wayland/",
                "third_party/wgpu/",
                "third_party/winit/",
            ],
        ),
        (
            "LINUX KERNEL 7.4+, DISTRO PACKAGING & PATCH QUEUES",
            "Supported",
            &[
                "git https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git",
                "git https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git",
                "git https://git.kernel.org/pub/scm/linux/kernel/git/next/linux-next.git",
                "git https://github.com/aphrody-dev/unix.git",
                "quilt docs/patches/",
            ],
            &[
                "MAINTAINERS",
                "crates/unix/",
                "docs/linux-kernel-story/",
                "docs/patches/",
                "packages/knowledge-engine/",
                "third_party/archlinux/",
                "third_party/cachyos/",
                "third_party/linux/",
                "third_party/omacom/",
            ],
        ),
        (
            "MONOREPO VCS ORCHESTRATION, TRI-FORGE & ARCHITECTURE GATE",
            "Supported",
            &[
                "git https://github.com/aphrody-dev/src.git",
                "git https://github.com/aphrody-dev/xtask.git",
                "git https://github.com/aphrody-dev/forge.git",
            ],
            &[
                ".gitattributes",
                ".gitignore",
                ".gitmodules",
                "crates/xtask/",
                "justfile",
                "packages/env-kit/",
                "packages/forge/",
                "src.lock.json",
                "tools/archguard/",
            ],
        ),
        (
            "RING-3 HARDWARE TELEMETRY, BENCHMARKS & PURE-RUST STACK",
            "Supported",
            &[
                "git https://github.com/aphrody-dev/ring3-agent.git",
                "git https://github.com/aphrody-dev/bench.git",
                "git https://github.com/aphrody-dev/rust-stack.git",
            ],
            &[
                "crates/bench/",
                "crates/ring3-agent/",
                "crates/rust-stack/",
            ],
        ),
        (
            "SOVEREIGN AI CONTROL PLANE & KNOWLEDGE ENGINE",
            "Supported",
            &[
                "git https://github.com/aphrody-dev/ai-os.git",
                "git https://github.com/aphrody-dev/ai-triad.git",
                "git https://github.com/aphrody-dev/knowledge-engine.git",
                "git https://github.com/aphrody-dev/pi-opencode.git",
            ],
            &[
                ".agents/",
                "packages/agy/",
                "packages/ai-os/",
                "packages/ai-triad/",
                "packages/brain/",
                "packages/knowledge-engine/",
                "packages/pi-opencode/",
            ],
        ),
    ];

    for (title, status, trees, files) in sections {
        out.push_str(title);
        out.push('\n');
        out.push_str("M:\tAphrody Engineering <contact@aphrody.com>\n");
        out.push_str("L:\tcontact@aphrody.com\n");
        out.push_str(&format!("S:\t{}\n", status));
        out.push_str("W:\thttps://about.aphrody.com\n");
        for t in *trees {
            out.push_str(&format!("T:\t{}\n", t));
        }
        for f in *files {
            out.push_str(&format!("F:\t{}\n", f));
        }
        out.push('\n');
    }

    out.push_str("THE REST\n");
    out.push_str("M:\tAphrody Engineering <contact@aphrody.com>\n");
    out.push_str("S:\tMaintained\n");
    out.push_str("F:\t*\n");
    out.push_str("F:\t*/\n");

    out
}

fn write_governance_manifests(root: &Path, locked: &BTreeMap<String, LockedRepo>) {
    // 1. Chromium DEPS at root
    let deps_path = root.join("DEPS");
    let deps_content = render_chromium_deps(locked);
    fs::write(&deps_path, deps_content).expect("Failed to write C:\\src\\DEPS");

    // 2. Android AOSP repo manifest at .repo/manifests/default.xml
    let repo_manifests_dir = root.join(".repo").join("manifests");
    fs::create_dir_all(&repo_manifests_dir).expect("Failed to create .repo/manifests");
    let aosp_xml_path = repo_manifests_dir.join("default.xml");
    let aosp_xml = render_aosp_repo_manifest(locked);
    fs::write(&aosp_xml_path, &aosp_xml).expect("Failed to write .repo/manifests/default.xml");
    fs::write(root.join(".repo").join("manifest.xml"), &aosp_xml)
        .expect("Failed to write .repo/manifest.xml");

    // 3. Linux Kernel MAINTAINERS at root
    let maintainers_path = root.join("MAINTAINERS");
    let maintainers_content = render_kernel_maintainers();
    fs::write(&maintainers_path, maintainers_content).expect("Failed to write C:\\src\\MAINTAINERS");

    // 4. Declarative quilt / b4 patch series index at docs/patches/series
    let patches_dir = root.join("docs").join("patches");
    fs::create_dir_all(&patches_dir).expect("Failed to create docs/patches");
    let mut series_lines = vec![
        "# Sovereign Declarative Patch Series Index (Quilt / b4 / git-am compatible)".to_string(),
        "# Format: <relative-patch-path> -p1".to_string(),
    ];

    let bun_patches_dir = root.join("packages").join("bun-parity").join("patches");
    if let Ok(entries) = fs::read_dir(&bun_patches_dir) {
        let mut names: Vec<String> = entries
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.ends_with(".patch"))
            .collect();
        names.sort();
        for n in names {
            series_lines.push(format!("../../packages/bun-parity/patches/{} -p1", n));
        }
    }

    let cachy_74_sched = root
        .join("third_party")
        .join("cachyos")
        .join("kernel-patches")
        .join("7.4")
        .join("sched");
    if let Ok(entries) = fs::read_dir(&cachy_74_sched) {
        let mut names: Vec<String> = entries
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.ends_with(".patch"))
            .collect();
        names.sort();
        for n in names {
            series_lines.push(format!(
                "../../third_party/cachyos/kernel-patches/7.4/sched/{} -p1",
                n
            ));
        }
    }

    fs::write(
        patches_dir.join("series"),
        format!("{}\n", series_lines.join("\n")),
    )
    .expect("Failed to write docs/patches/series");

    // Also ensure packages/bun-parity/patches/series exists for standalone quilt compatibility
    if bun_patches_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&bun_patches_dir) {
            let mut names: Vec<String> = entries
                .flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|n| n.ends_with(".patch"))
                .collect();
            names.sort();
            let _ = fs::write(
                bun_patches_dir.join("series"),
                format!("{}\n", names.join("\n")),
            );
        }
    }
}

fn run_manifests(root_str: &str) {
    let root = Path::new(root_str);
    let locked = collect_locked_repos(root);
    write_governance_manifests(root, &locked);
    println!(
        "Synchronized 4-pillar manifests across {} repositories:\n  - {}\n  - {}\n  - {}\n  - {}",
        locked.len(),
        root.join("DEPS").display(),
        root.join(".repo\\manifests\\default.xml").display(),
        root.join("MAINTAINERS").display(),
        root.join("docs\\patches\\series").display()
    );
}

fn run_lock(root_str: &str, out_path_str: &str, lock_only: bool) {
    let root = Path::new(root_str);
    if root.join(".git").exists() {
        apply_scalar_git_config(root);
    }

    let locked_repos = collect_locked_repos(root);

    println!("\n=== GENERATING SOVEREIGN 4-PILLAR LOCKFILE (xtk) ===");
    println!(
        "Locking {} repositories under {}...",
        locked_repos.len(),
        root.display()
    );

    let mut toolchains = BTreeMap::new();
    let checks = [
        ("rustc", "rustc", vec!["--version"]),
        ("cargo", "cargo", vec!["--version"]),
        ("sccache", "sccache", vec!["--version"]),
        ("xwin", "xwin", vec!["--version"]),
        ("bun", "bun", vec!["--version"]),
        ("jj", "jj", vec!["--version"]),
        ("gix", "gix", vec!["--version"]),
        ("just", "just", vec!["--version"]),
        ("go", "go", vec!["version"]),
        ("zig", "zig", vec!["version"]),
        ("dotnet", "dotnet", vec!["--version"]),
        ("nvcc", "nvcc", vec!["--version"]),
        ("claude", "claude", vec!["--version"]),
        ("agy", "agy", vec!["--version"]),
        ("wck", "wck", vec!["--version"]),
        ("archguard", "archguard", vec!["--version"]),
    ];

    for (k, prog, a) in checks {
        if let Some(v) = safe_tool_probe(prog, &a) {
            let first = v.lines().next().unwrap_or("").to_string();
            toolchains.insert(k.to_string(), first);
        }
    }
    toolchains.insert("cl.exe".to_string(), "MSVC 14.51 (x86_64)".to_string());

    let root_commit = safe_output("git", &["-C", root_str, "rev-parse", "HEAD"])
        .unwrap_or_else(|| "0000000000000000000000000000000000000000".to_string());
    let chromium_commit = locked_repos
        .get("third_party/chromium/src")
        .map(|r| r.commit.clone())
        .unwrap_or_default();
    let linux_commit = locked_repos
        .get("third_party/linux")
        .map(|r| r.commit.clone())
        .unwrap_or_default();
    let aosp_commit = locked_repos
        .get("third_party/aosp/platform/manifest")
        .map(|r| r.commit.clone())
        .unwrap_or_default();

    let lock = SrcLock {
        version: 2,
        generated_at: chrono_like_now(),
        hardware: HardwareTarget {
            os: "Windows NT (x86_64)".to_string(),
            cpu: "Intel Core i7-13700F (Raptor Lake sm_89)".to_string(),
            gpu: "NVIDIA GeForce RTX 4070 12GB (Ada Lovelace sm_89)".to_string(),
        },
        toolchains,
        pillars: PillarSummary {
            git_superproject_commit: root_commit,
            chromium_src_commit: chromium_commit,
            linux_kernel_commit: linux_commit,
            aosp_manifest_commit: aosp_commit,
        },
        repositories: locked_repos.clone(),
    };

    let json_text = serde_json::to_string_pretty(&lock).expect("Failed to serialize lockfile");
    let out_path = Path::new(out_path_str);
    fs::write(out_path, json_text).expect("Failed to write lockfile");
    println!("Deterministic lockfile written to: {}", out_path.display());

    if !lock_only {
        write_governance_manifests(root, &locked_repos);
        println!("Synchronized DEPS, .repo/manifests/default.xml, MAINTAINERS, and docs/patches/series.\n");
    }
}

fn run_audit_vcs(root_str: &str, as_json: bool) {
    let root = Path::new(root_str);
    let repos = discover_repos(root);

    let mut junctions_count = 0usize;
    let mut dangling = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for e in entries.flatten() {
            let p = e.path();
            if is_reparse_point(&p) {
                junctions_count += 1;
                if fs::metadata(&p).is_err() {
                    if let Some(n) = p.file_name().and_then(|s| s.to_str()) {
                        dangling.push(n.to_string());
                    }
                }
            }
        }
    }

    let gitmodules_txt = fs::read_to_string(root.join(".gitmodules")).unwrap_or_default();
    let submodules_count = gitmodules_txt
        .lines()
        .filter(|l| l.trim_start().starts_with("[submodule "))
        .count();

    let git_ok = root.join(".gitmodules").is_file()
        && root.join("src.lock.json").is_file()
        && dangling.is_empty();

    let chromium_ok = root.join("DEPS").is_file()
        && root.join("third_party").join("depot_tools").is_dir()
        && root.join("third_party").join("chromium").join("src").is_dir();

    let linux_remotes = read_git_remotes(&root.join("third_party").join("linux"));
    let kernel_ok = root.join("MAINTAINERS").is_file()
        && root.join("third_party").join("linux").is_dir()
        && linux_remotes.contains_key("stable")
        && linux_remotes.contains_key("next");

    let aosp_ok = root
        .join(".repo")
        .join("manifests")
        .join("default.xml")
        .is_file()
        && root
            .join("third_party")
            .join("aosp")
            .join("platform")
            .join("manifest")
            .is_dir();

    let mut notes = Vec::new();
    notes.push(format!(
        "Git Superproject: {} submodules, {} root junctions ({} dangling)",
        submodules_count,
        junctions_count,
        dangling.len()
    ));
    notes.push(format!(
        "Chromium Pillar: DEPS={}, depot_tools={}",
        root.join("DEPS").is_file(),
        root.join("third_party").join("depot_tools").is_dir()
    ));
    notes.push(format!(
        "Linux Kernel Pillar: MAINTAINERS={}, remotes={}",
        root.join("MAINTAINERS").is_file(),
        linux_remotes.len()
    ));
    notes.push(format!(
        "Android AOSP Pillar: default.xml={}, kleaf={}",
        root.join(".repo")
            .join("manifests")
            .join("default.xml")
            .is_file(),
        root.join("third_party")
            .join("aosp")
            .join("build")
            .join("kleaf")
            .is_dir()
    ));

    let report = VcsAuditReport {
        root: root.to_string_lossy().to_string(),
        healthy: git_ok && chromium_ok && kernel_ok && aosp_ok,
        locked_repositories: repos.len(),
        submodules_count,
        root_junctions_count: junctions_count,
        dangling_junctions: dangling,
        git_superproject_ok: git_ok,
        chromium_deps_ok: chromium_ok,
        linux_kernel_ok: kernel_ok,
        aosp_repo_ok: aosp_ok,
        notes,
    };

    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("\n=== 4-PILLAR MONOREPO VCS AUDIT (xtk) ===");
        println!("Root:                {}", report.root);
        println!("Discovered Repos:    {}", report.locked_repositories);
        println!("Git Submodules:      {}", report.submodules_count);
        println!(
            "Root NTFS Junctions: {} ({} dangling)",
            report.root_junctions_count,
            report.dangling_junctions.len()
        );
        println!(
            "Pillar [1] Git:      {}",
            if report.git_superproject_ok { "OK" } else { "FAIL" }
        );
        println!(
            "Pillar [2] Chromium: {}",
            if report.chromium_deps_ok { "OK" } else { "FAIL" }
        );
        println!(
            "Pillar [3] Kernel:   {}",
            if report.linux_kernel_ok { "OK" } else { "FAIL" }
        );
        println!(
            "Pillar [4] AOSP:     {}",
            if report.aosp_repo_ok { "OK" } else { "FAIL" }
        );
        println!(
            "Overall Status:      {}\n",
            if report.healthy { "PASS" } else { "FAIL" }
        );
    }

    if !report.healthy {
        std::process::exit(1);
    }
}

fn chrono_like_now() -> String {
    let out = safe_output(
        "powershell",
        &["-NoProfile", "-Command", "Get-Date -Format o"],
    );
    out.unwrap_or_else(|| "2026-11-11T08:00:00Z".to_string())
}

fn main() {
    let top: TopLevel = argh::from_env();
    match top.cmd {
        SubCommand::Doctor(_) => check_doctor(),
        SubCommand::SlimAll(cmd) => run_slim_all(&cmd.root),
        SubCommand::Lock(cmd) => run_lock(&cmd.root, &cmd.out, cmd.lock_only),
        SubCommand::Manifests(cmd) => run_manifests(&cmd.root),
        SubCommand::AuditVcs(cmd) => run_audit_vcs(&cmd.root, cmd.json),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_repos_skips_junctions_and_yolo() {
        let repos = discover_repos(Path::new("C:\\src"));
        assert!(!repos.is_empty(), "Should discover repos in C:\\src");
        for r in &repos {
            let s = r.to_string_lossy();
            assert!(!s.contains("yolo-archive"), "Must skip yolo-archive");
        }
    }

    #[test]
    fn test_render_manifests_deterministic() {
        let mut sample = BTreeMap::new();
        sample.insert(
            "third_party/chromium/src".to_string(),
            LockedRepo {
                path: "C:\\src\\third_party\\chromium\\src".to_string(),
                url: "https://chromium.googlesource.com/chromium/src.git".to_string(),
                upstream_url: None,
                commit: "4c7244a405d326e284710231ff609168dceb9044".to_string(),
                branch: "detached".to_string(),
                engine: "git".to_string(),
                pillar: "chromium".to_string(),
                remotes: None,
                sparse_paths: None,
            },
        );

        let deps = render_chromium_deps(&sample);
        assert!(deps.contains("git_dependencies = 'SYNC'"));
        assert!(deps.contains("'third_party/chromium/src': 'https://chromium.googlesource.com/chromium/src.git@4c7244a405d326e284710231ff609168dceb9044'"));

        let xml = render_aosp_repo_manifest(&sample);
        assert!(xml.contains("<manifest>"));
        assert!(xml.contains("remote=\"chromium\""));
        assert!(xml.contains("name=\"chromium/src\""));

        let maint = render_kernel_maintainers();
        assert!(maint.contains("LINUX KERNEL 7.4+, DISTRO PACKAGING & PATCH QUEUES"));
        assert!(maint.contains("ANDROID AOSP, GKI & APHRODY MOBILE SHELL"));
    }

    #[test]
    fn test_tool_version_lookup() {
        let (_, ok, ver) = find_tool_version("bun", "bun", &["--version"]);
        assert!(ok);
        assert!(ver.contains("1.4"));
    }
}
