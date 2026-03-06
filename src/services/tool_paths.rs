//! Executable path discovery helpers for local development and deployable layouts.

use std::path::{Path, PathBuf};

const WORKER_ENV_VARS: [&str; 2] = ["MLVINSPECTOR_WORKER_PATH", "ILINSPECTOR_WORKER_PATH"];
const INSPECTOR_ENV_VARS: [&str; 2] = ["MLVINSPECTOR_CLI_PATH", "ILINSPECTOR_PATH"];

pub fn resolve_worker_path() -> PathBuf {
    resolve_path(
        &WORKER_ENV_VARS,
        worker_candidates(),
        "ILInspector.Worker.exe",
    )
}

pub fn resolve_inspector_path() -> PathBuf {
    resolve_path(&INSPECTOR_ENV_VARS, inspector_candidates(), "ILInspector.exe")
}

fn resolve_path(env_vars: &[&str], candidates: Vec<PathBuf>, fallback: &str) -> PathBuf {
    for env_var in env_vars {
        if let Some(value) = std::env::var_os(env_var) {
            let path = PathBuf::from(value);
            if path.exists() {
                return path;
            }
        }
    }

    for candidate in candidates {
        if candidate.exists() {
            return candidate;
        }
    }

    PathBuf::from(fallback)
}

fn worker_candidates() -> Vec<PathBuf> {
    executable_candidates(
        "ILInspector.Worker.exe",
        &[
            &[
                "MLVInspector.Worker",
                "bin",
                "Debug",
                "net8.0",
                "ILInspector.Worker.exe",
            ],
            &[
                "MLVInspector.Worker",
                "bin",
                "Release",
                "net8.0",
                "ILInspector.Worker.exe",
            ],
            &["MLVInspector.Worker", "ILInspector.Worker.exe"],
            &["tools", "ILInspector.Worker.exe"],
        ],
    )
}

fn inspector_candidates() -> Vec<PathBuf> {
    executable_candidates(
        "ILInspector.exe",
        &[
            &["MLVInspector.CLI", "bin", "Debug", "net8.0", "ILInspector.exe"],
            &[
                "MLVInspector.CLI",
                "bin",
                "Release",
                "net8.0",
                "ILInspector.exe",
            ],
            &["MLVInspector.CLI", "ILInspector.exe"],
            &["tools", "ILInspector.exe"],
        ],
    )
}

fn executable_candidates(file_name: &str, relative_paths: &[&[&str]]) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(current_dir) = std::env::current_dir() {
        push_candidate_set(&mut candidates, &current_dir, file_name, relative_paths);
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            push_candidate_set(&mut candidates, exe_dir, file_name, relative_paths);

            if let Some(target_dir) = exe_dir.parent() {
                push_candidate_set(&mut candidates, target_dir, file_name, relative_paths);

                if let Some(repo_root) = target_dir.parent() {
                    push_candidate_set(&mut candidates, repo_root, file_name, relative_paths);
                }
            }
        }
    }

    dedupe_paths(candidates)
}

fn push_candidate_set(
    candidates: &mut Vec<PathBuf>,
    base: &Path,
    file_name: &str,
    relative_paths: &[&[&str]],
) {
    candidates.push(base.join(file_name));

    for relative in relative_paths {
        let mut candidate = base.to_path_buf();
        for segment in *relative {
            candidate.push(segment);
        }
        candidates.push(candidate);
    }
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut unique = Vec::new();
    for path in paths {
        if !unique.iter().any(|existing| existing == &path) {
            unique.push(path);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::dedupe_paths;
    use std::path::PathBuf;

    #[test]
    fn dedupe_paths_preserves_first_instance() {
        let paths = vec![
            PathBuf::from("a"),
            PathBuf::from("b"),
            PathBuf::from("a"),
            PathBuf::from("c"),
        ];

        let deduped = dedupe_paths(paths);

        assert_eq!(deduped, vec![PathBuf::from("a"), PathBuf::from("b"), PathBuf::from("c")]);
    }
}
