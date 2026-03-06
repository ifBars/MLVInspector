use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::AppError;
use crate::ipc::DecompileParams;
use crate::services::worker_client::WorkerClient;
use crate::types::{AnalysisResult, OpenAssembly};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportManifest {
    assembly_name: String,
    assembly_path: String,
    exported_at_unix: u64,
    exported_files: Vec<String>,
    project_file: String,
    type_count: usize,
    namespace_count: usize,
    explore_summary: Option<ExploreSummary>,
    scan_summary: Option<ScanSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExploreSummary {
    namespace_count: usize,
    type_count: usize,
    method_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanSummary {
    finding_count: usize,
    triggered_rules: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExportTypeFile {
    full_type_name: String,
    relative_path: PathBuf,
}

pub async fn export_project_bundle(
    worker: WorkerClient,
    assembly: OpenAssembly,
    analysis: Option<AnalysisResult>,
    destination_root: PathBuf,
) -> Result<PathBuf, AppError> {
    let export_dir = unique_export_dir(&destination_root, &assembly.name);
    tokio::fs::create_dir_all(&export_dir).await?;

    let project_name = sanitized_stem(&assembly.name);
    let project_file_name = format!("{project_name}.csproj");
    let export_plan = build_export_type_files(analysis.as_ref(), &project_name);
    let mut exported_files = Vec::new();

    for type_file in &export_plan {
        let destination = export_dir.join(&type_file.relative_path);
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let decompiled = worker
            .decompile(DecompileParams {
                assembly: assembly.path.clone(),
                type_name: Some(type_file.full_type_name.clone()),
                method_name: None,
            })
            .await?;

        tokio::fs::write(&destination, decompiled.csharp_source).await?;
        exported_files.push(path_to_manifest_string(&type_file.relative_path));
    }

    let project_contents = render_project_file(&project_name);
    tokio::fs::write(export_dir.join(&project_file_name), project_contents).await?;
    exported_files.push(project_file_name.clone());

    if let Some(result) = analysis.as_ref() {
        if let Some(explore) = result.explore.as_ref() {
            let json = serde_json::to_string_pretty(explore).map_err(|err| {
                AppError::Parse(format!("failed to serialize explore export: {err}"))
            })?;
            tokio::fs::write(export_dir.join("explore.json"), json).await?;
            exported_files.push("explore.json".to_string());
        }

        if let Some(scan) = result.scan.as_ref() {
            let json = serde_json::to_string_pretty(scan).map_err(|err| {
                AppError::Parse(format!("failed to serialize scan export: {err}"))
            })?;
            tokio::fs::write(export_dir.join("scan.json"), json).await?;
            exported_files.push("scan.json".to_string());
        }
    }

    exported_files.sort();

    let manifest = ExportManifest {
        assembly_name: assembly.name,
        assembly_path: assembly.path,
        exported_at_unix: now_ts(),
        exported_files,
        project_file: project_file_name,
        type_count: export_plan.len(),
        namespace_count: export_plan
            .iter()
            .map(|entry| namespace_from_type(&entry.full_type_name))
            .collect::<BTreeSet<_>>()
            .len(),
        explore_summary: analysis.as_ref().and_then(build_explore_summary),
        scan_summary: analysis.as_ref().and_then(build_scan_summary),
    };

    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|err| AppError::Parse(format!("failed to serialize export manifest: {err}")))?;
    tokio::fs::write(export_dir.join("manifest.json"), manifest_json).await?;

    Ok(export_dir)
}

fn build_export_type_files(
    analysis: Option<&AnalysisResult>,
    project_name: &str,
) -> Vec<ExportTypeFile> {
    let Some(result) = analysis else {
        return Vec::new();
    };
    let Some(explore) = result.explore.as_ref() else {
        return Vec::new();
    };

    let type_names = discover_type_names(result);
    let mut used_paths = BTreeMap::<PathBuf, usize>::new();
    let mut files = Vec::new();

    for type_name in type_names {
        let relative_path = unique_type_relative_path(&type_name, project_name, &mut used_paths);
        files.push(ExportTypeFile {
            full_type_name: type_name,
            relative_path,
        });
    }

    if files.is_empty() && !explore.methods.is_empty() {
        for type_name in discover_type_names(result) {
            let relative_path =
                unique_type_relative_path(&type_name, project_name, &mut used_paths);
            files.push(ExportTypeFile {
                full_type_name: type_name,
                relative_path,
            });
        }
    }

    files
}

fn discover_type_names(result: &AnalysisResult) -> Vec<String> {
    let Some(explore) = result.explore.as_ref() else {
        return Vec::new();
    };

    let mut type_names = BTreeSet::new();

    for entry in &explore.types {
        type_names.insert(entry.type_name.clone());
    }

    for method in &explore.methods {
        type_names.insert(method.type_name.clone());
    }

    type_names.into_iter().collect()
}

fn unique_type_relative_path(
    type_name: &str,
    project_name: &str,
    used_paths: &mut BTreeMap<PathBuf, usize>,
) -> PathBuf {
    let (namespace, leaf_name) = split_type_name(type_name);
    let mut path = PathBuf::new();

    if namespace == "(global)" {
        path.push("Global");
    } else {
        for segment in namespace.split('.') {
            path.push(sanitize_path_segment(segment));
        }
    }

    let base_file_name = sanitize_path_segment(&leaf_name);
    let candidate_name = if base_file_name.is_empty() {
        project_name.to_string()
    } else {
        base_file_name
    };

    let initial_path = path.join(format!("{candidate_name}.cs"));
    let count = used_paths.entry(initial_path.clone()).or_insert(0);
    let final_path = if *count == 0 {
        initial_path
    } else {
        path.join(format!("{candidate_name}.{}.cs", *count + 1))
    };
    *count += 1;

    final_path
}

fn render_project_file(project_name: &str) -> String {
    format!(
        r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <AssemblyName>{project_name}</AssemblyName>
    <RootNamespace>{project_name}</RootNamespace>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>disable</ImplicitUsings>
    <Nullable>disable</Nullable>
    <LangVersion>latest</LangVersion>
    <EnableDefaultCompileItems>true</EnableDefaultCompileItems>
    <GenerateAssemblyInfo>false</GenerateAssemblyInfo>
  </PropertyGroup>
</Project>
"#
    )
}

fn build_explore_summary(result: &AnalysisResult) -> Option<ExploreSummary> {
    let explore = result.explore.as_ref()?;
    let type_count = discover_type_names(result).len();

    let namespace_count = discover_type_names(result)
        .into_iter()
        .map(|type_name| namespace_from_type(&type_name))
        .collect::<BTreeSet<_>>()
        .len();

    Some(ExploreSummary {
        namespace_count,
        type_count,
        method_count: explore.methods.len(),
    })
}

fn build_scan_summary(result: &AnalysisResult) -> Option<ScanSummary> {
    let scan = result.scan.as_ref()?;
    Some(ScanSummary {
        finding_count: scan.findings.len(),
        triggered_rules: scan.summary.triggered_rules.len(),
    })
}

fn split_type_name(type_name: &str) -> (String, String) {
    type_name
        .rsplit_once('.')
        .map(|(namespace, leaf_name)| (namespace.to_string(), leaf_name.to_string()))
        .unwrap_or_else(|| ("(global)".to_string(), type_name.to_string()))
}

fn namespace_from_type(type_name: &str) -> String {
    split_type_name(type_name).0
}

fn sanitize_path_segment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ if ch.is_ascii_control() => '_',
            _ => ch,
        })
        .collect::<String>()
        .trim_matches([' ', '.'])
        .replace('`', "_");

    if sanitized.is_empty() {
        "Item".to_string()
    } else {
        sanitized
    }
}

fn unique_export_dir(destination_root: &Path, assembly_name: &str) -> PathBuf {
    let base_name = sanitized_stem(assembly_name);
    let mut candidate = destination_root.join(&base_name);
    let mut suffix = 2usize;

    while candidate.exists() {
        candidate = destination_root.join(format!("{base_name}-{suffix}"));
        suffix += 1;
    }

    candidate
}

fn path_to_manifest_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn sanitized_stem(assembly_name: &str) -> String {
    let stem = Path::new(assembly_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("assembly-export");

    let sanitized = stem
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => ch,
        })
        .collect::<String>()
        .trim_matches([' ', '.'])
        .to_string();

    if sanitized.is_empty() {
        "assembly-export".to_string()
    } else {
        sanitized
    }
}

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::ipc::{ExplorePayload, MethodEntry, ScanPayload, ScanSummaryEntry};

    use super::{
        build_export_type_files, discover_type_names, namespace_from_type, path_to_manifest_string,
        render_project_file, sanitize_path_segment, sanitized_stem, AnalysisResult,
    };

    #[test]
    fn sanitized_stem_replaces_windows_invalid_characters() {
        assert_eq!(sanitized_stem("Bad:Name?.dll"), "Bad_Name_");
    }

    #[test]
    fn namespace_from_type_returns_global_when_missing_separator() {
        assert_eq!(namespace_from_type("Program"), "(global)");
        assert_eq!(namespace_from_type("Example.Loader"), "Example");
    }

    #[test]
    fn build_export_type_files_creates_namespace_layout() {
        let files = build_export_type_files(Some(&sample_result()), "Sample");

        assert_eq!(files.len(), 3);
        assert_eq!(
            path_to_manifest_string(&files[0].relative_path),
            "Example/Core/Entry.cs"
        );
        assert_eq!(
            path_to_manifest_string(&files[1].relative_path),
            "Example/Core/Entry_1.cs"
        );
        assert_eq!(
            path_to_manifest_string(&files[2].relative_path),
            "Global/Program.cs"
        );
    }

    #[test]
    fn render_project_file_uses_sdk_style_project() {
        let csproj = render_project_file("Sample");

        assert!(csproj.contains("<Project Sdk=\"Microsoft.NET.Sdk\">"));
        assert!(csproj.contains("<AssemblyName>Sample</AssemblyName>"));
        assert!(csproj.contains("<TargetFramework>net8.0</TargetFramework>"));
    }

    #[test]
    fn sanitize_path_segment_removes_bad_filename_characters() {
        assert_eq!(sanitize_path_segment("Type`1<Name>"), "Type_1_Name_");
    }

    #[test]
    fn discover_type_names_deduplicates_types_from_methods_and_types() {
        let names = discover_type_names(&sample_result());

        assert_eq!(names.len(), 3);
        assert!(names.contains(&"Example.Core.Entry".to_string()));
        assert!(names.contains(&"Example.Core.Entry`1".to_string()));
        assert!(names.contains(&"Program".to_string()));
    }

    fn sample_result() -> AnalysisResult {
        AnalysisResult {
            assembly_path: "C:/sample.dll".to_string(),
            mode: "combined".to_string(),
            explore: Some(ExplorePayload {
                assembly_path: "C:/sample.dll".to_string(),
                methods: vec![
                    sample_method("Example.Core.Entry", "Run"),
                    sample_method("Example.Core.Entry`1", "Handle"),
                    sample_method("Program", "Main"),
                ],
                types: vec![
                    crate::ipc::TypeEntry {
                        type_name: "Example.Core.Entry".to_string(),
                        methods: Vec::new(),
                    },
                    crate::ipc::TypeEntry {
                        type_name: "Example.Core.Entry`1".to_string(),
                        methods: Vec::new(),
                    },
                ],
            }),
            scan: Some(ScanPayload {
                assembly_path: "C:/sample.dll".to_string(),
                schema_version: "1".to_string(),
                metadata: crate::ipc::ScanMetaEntry {
                    scanner_version: "1".to_string(),
                    timestamp: "now".to_string(),
                    scan_mode: "full".to_string(),
                    platform: "windows".to_string(),
                },
                input: crate::ipc::ScanInputEntry {
                    file_name: "sample.dll".to_string(),
                    size_bytes: 1,
                    sha256_hash: None,
                },
                summary: ScanSummaryEntry {
                    total_findings: 0,
                    count_by_severity: HashMap::new(),
                    triggered_rules: vec!["RULE1".to_string()],
                },
                findings: Vec::new(),
                call_chains: None,
                data_flows: None,
            }),
            stderr: String::new(),
        }
    }

    fn sample_method(type_name: &str, method_name: &str) -> MethodEntry {
        MethodEntry {
            type_name: type_name.to_string(),
            method_name: method_name.to_string(),
            signature: format!("void {method_name}()"),
            has_body: Some(true),
            instructions: Vec::new(),
            p_invoke: None,
        }
    }
}
