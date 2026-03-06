//! Executable path discovery helpers for local development and deployable layouts.

use std::path::{Path, PathBuf};

const WORKER_ENV_VARS: [&str; 2] = ["MLVINSPECTOR_WORKER_PATH", "ILINSPECTOR_WORKER_PATH"];
const INSPECTOR_ENV_VARS: [&str; 2] = ["MLVINSPECTOR_CLI_PATH", "ILINSPECTOR_PATH"];

#[cfg(target_os = "windows")]
const WORKER_BINARY_NAME: &str = "ILInspector.Worker.exe";
#[cfg(not(target_os = "windows"))]
const WORKER_BINARY_NAME: &str = "ILInspector.Worker";

#[cfg(target_os = "windows")]
const INSPECTOR_BINARY_NAME: &str = "ILInspector.exe";
#[cfg(not(target_os = "windows"))]
const INSPECTOR_BINARY_NAME: &str = "ILInspector";

pub fn resolve_worker_path() -> PathBuf {
    resolve_path(&WORKER_ENV_VARS, worker_candidates(), WORKER_BINARY_NAME)
}

pub fn resolve_inspector_path() -> PathBuf {
    resolve_path(
        &INSPECTOR_ENV_VARS,
        inspector_candidates(),
        INSPECTOR_BINARY_NAME,
    )
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
        WORKER_BINARY_NAME,
        &[
            &[
                "MLVInspector.Worker",
                "bin",
                "Debug",
                "net8.0",
                WORKER_BINARY_NAME,
            ],
            &[
                "MLVInspector.Worker",
                "bin",
                "Release",
                "net8.0",
                WORKER_BINARY_NAME,
            ],
            &["MLVInspector.Worker", WORKER_BINARY_NAME],
            &["tools", WORKER_BINARY_NAME],
        ],
    )
}

fn inspector_candidates() -> Vec<PathBuf> {
    executable_candidates(
        INSPECTOR_BINARY_NAME,
        &[
            &[
                "MLVInspector.CLI",
                "bin",
                "Debug",
                "net8.0",
                INSPECTOR_BINARY_NAME,
            ],
            &[
                "MLVInspector.CLI",
                "bin",
                "Release",
                "net8.0",
                INSPECTOR_BINARY_NAME,
            ],
            &["MLVInspector.CLI", INSPECTOR_BINARY_NAME],
            &["tools", INSPECTOR_BINARY_NAME],
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
    use super::{dedupe_paths, executable_candidates};
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

        assert_eq!(
            deduped,
            vec![PathBuf::from("a"), PathBuf::from("b"), PathBuf::from("c")]
        );
    }

    #[test]
    fn executable_candidates_include_non_windows_layout() {
        let candidates = executable_candidates(
            "ILInspector.Worker",
            &[[
                "MLVInspector.Worker",
                "bin",
                "Debug",
                "net8.0",
                "ILInspector.Worker",
            ]
            .as_slice()],
        );

        assert!(candidates.iter().any(|p| p.ends_with("ILInspector.Worker")));
    }
}
