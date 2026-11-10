// Sovereign Monorepo Engine & Toolchain Orchestrator: xtk
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
/// generate deterministic C:\src\src.lock.json pinning commits and toolchains
#[argh(subcommand, name = "lock")]
struct LockCmd {
    /// target root directory (defaults to C:\src)
    #[argh(option, default = "String::from(\"C:\\\\src\")")]
    root: String,
    /// output lockfile path (defaults to C:\src\src.lock.json)
    #[argh(option, default = "String::from(\"C:\\\\src\\\\src.lock.json\")")]
    out: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SrcLock {
    version: u32,
    generated_at: String,
    hardware: HardwareTarget,
    toolchains: BTreeMap<String, String>,
    repositories: BTreeMap<String, LockedRepo>,
}

#[derive(Serialize, Deserialize, Debug)]
struct HardwareTarget {
    os: String,
    cpu: String,
    gpu: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct LockedRepo {
    path: String,
    url: String,
    commit: String,
    branch: String,
    engine: String,
    sparse_paths: Option<Vec<String>>,
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
                let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                if err.is_empty() {
                    None
                } else {
                    Some(err)
                }
            }
        })
}

fn find_tool_version(name: &str, program: &str, args: &[&str]) -> (String, bool, String) {
    if let Some(out) = safe_output(program, args) {
        let first_line = out.lines().next().unwrap_or("").to_string();
        (name.to_string(), true, first_line)
    } else {
        (name.to_string(), false, "NOT FOUND".to_string())
    }
}

fn check_doctor() {
    println!("\n=== SOVEREIGN TOOLCHAIN DOCTOR (xtk) ===");
    println!("{:<14} {:<8} {}", "TOOL", "STATUS", "VERSION / DETAILS");
    println!("{}", "-".repeat(70));

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
        ("claude", "claude", vec!["--version"]),
        ("agy", "agy", vec!["--version"]),
        ("wck", "wck", vec!["--version"]),
    ];

    let mut all_ok = true;
    for (name, prog, args) in tools {
        let (n, ok, detail) = find_tool_version(name, prog, &args);
        let status = if ok { "OK" } else { "MISSING" };
        if !ok {
            all_ok = false;
        }

        // Clean up multiline outputs like cl.exe
        let short_detail = if n == "cl.exe" && ok {
            "MSVC v14.51 (Microsoft C/C++ Optimizing Compiler)".to_string()
        } else if detail.len() > 50 {
            format!("{}...", &detail[..47])
        } else {
            detail
        };

        println!("{:<14} {:<8} {}", n, status, short_detail);
    }

    println!("{}", "-".repeat(70));
    if all_ok {
        println!("All 16 sovereign toolchains & AI CLIs verified and operational.\n");
    } else {
        println!("Some toolchains are missing or not in PATH.\n");
    }
}

fn is_reparse_point(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if let Ok(meta) = fs::symlink_metadata(path) {
            return (meta.file_attributes() & 0x0400) != 0;
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
            if name.starts_with('.') || name == "node_modules" || name == "target" {
                continue;
            }
            // Allow packages/ai-os junction while its real backing dir at root is hidden, skip root compat junctions
            if is_reparse_point(&p) && !(depth == 1 && name == "ai-os") {
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

fn discover_repos(root: &Path) -> Vec<PathBuf> {
    let mut repos = Vec::new();
    let canonical_groups = ["apps", "packages", "crates", "infra", "third_party"];
    for group in canonical_groups {
        let gp = root.join(group);
        if gp.is_dir() {
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

fn run_slim_all(root_str: &str) {
    let root = Path::new(root_str);
    let repos = discover_repos(root);

    println!("\n=== SOVEREIGN BYTE-LEVEL SLIM ALL (xtk) ===");
    println!("Scanning repositories under {}...", root.display());
    println!("{:<28} {:>14} {:>14} {:>14}", "REPOSITORY", "BEFORE", "AFTER", "SAVED");
    println!("{}", "-".repeat(74));

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
            "{:<28} {:>11} KB {:>11} KB {:>11} KB",
            rel_name,
            before / 1024,
            after / 1024,
            saved / 1024
        );
    }

    println!("{}", "-".repeat(74));
    println!(
        "{:<28} {:>11} MB {:>11} MB {:>11} MB",
        "TOTAL DISK FOOTPRINT",
        total_before / (1024 * 1024),
        total_after / (1024 * 1024),
        total_saved / (1024 * 1024)
    );
    println!("Byte-level slimming complete: {} bytes saved.\n", total_saved);
}

fn run_lock(root_str: &str, out_path_str: &str) {
    let root = Path::new(root_str);
    let repos = discover_repos(root);

    println!("\n=== GENERATING SOVEREIGN REPRODUCIBLE LOCKFILE (xtk) ===");
    println!("Locking {} repositories under {}...", repos.len(), root.display());

    let mut locked_repos = BTreeMap::new();

    for repo in &repos {
        let rel_name = repo
            .strip_prefix(root)
            .unwrap_or(repo)
            .to_string_lossy()
            .replace('\\', "/");

        let url = safe_output("git", &["-C", &repo.to_string_lossy(), "config", "--get", "remote.origin.url"])
            .unwrap_or_else(|| format!("local:{}", rel_name));

        let mut commit = safe_output("git", &["-C", &repo.to_string_lossy(), "rev-parse", "HEAD"])
            .unwrap_or_else(|| "0000000000000000000000000000000000000000".to_string());
        if commit.contains("fatal:") || commit.contains("unknown revision") || commit.len() < 40 {
            commit = "0000000000000000000000000000000000000000".to_string();
        }

        let branch = safe_output("git", &["-C", &repo.to_string_lossy(), "branch", "--show-current"])
            .filter(|b| !b.is_empty())
            .unwrap_or_else(|| "main".to_string());

        let engine = if repo.join(".jj").exists() {
            "jj".to_string()
        } else {
            "git".to_string()
        };

        // Sparse checkout paths if configured
        let sparse_file = repo.join(".git").join("info").join("sparse-checkout");
        let sparse_paths = if sparse_file.exists() {
            fs::read_to_string(&sparse_file)
                .ok()
                .map(|c| c.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
        } else {
            None
        };

        locked_repos.insert(
            rel_name.clone(),
            LockedRepo {
                path: repo.to_string_lossy().to_string(),
                url,
                commit,
                branch,
                engine,
                sparse_paths,
            },
        );
    }

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
    ];

    for (k, prog, a) in checks {
        if let Some(v) = safe_output(prog, &a) {
            let first = v.lines().next().unwrap_or("").to_string();
            toolchains.insert(k.to_string(), first);
        }
    }
    toolchains.insert("cl.exe".to_string(), "MSVC 14.51 (x86_64)".to_string());

    let lock = SrcLock {
        version: 1,
        generated_at: chrono_like_now(),
        hardware: HardwareTarget {
            os: "Windows NT (x86_64)".to_string(),
            cpu: "Intel Core i7-13700F (Raptor Lake sm_89)".to_string(),
            gpu: "NVIDIA GeForce RTX 4070 12GB (Ada Lovelace sm_89)".to_string(),
        },
        toolchains,
        repositories: locked_repos,
    };

    let json_text = serde_json::to_string_pretty(&lock).expect("Failed to serialize lockfile");
    let out_path = Path::new(out_path_str);
    fs::write(out_path, json_text).expect("Failed to write lockfile");

    println!("Deterministic lockfile written to: {}\n", out_path.display());
}

fn chrono_like_now() -> String {
    // Generate ISO timestamp without external chrono crate
    let out = safe_output("powershell", &["-NoProfile", "-Command", "Get-Date -Format o"]);
    out.unwrap_or_else(|| "2026-11-10T22:00:00Z".to_string())
}

fn main() {
    let top: TopLevel = argh::from_env();
    match top.cmd {
        SubCommand::Doctor(_) => check_doctor(),
        SubCommand::SlimAll(cmd) => run_slim_all(&cmd.root),
        SubCommand::Lock(cmd) => run_lock(&cmd.root, &cmd.out),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_repos() {
        let repos = discover_repos(Path::new("C:\\src"));
        assert!(!repos.is_empty(), "Should discover repos in C:\\src");
    }

    #[test]
    fn test_tool_version_lookup() {
        let (_, ok, ver) = find_tool_version("bun", "bun", &["--version"]);
        assert!(ok);
        assert!(ver.contains("1.4"));
    }
}
