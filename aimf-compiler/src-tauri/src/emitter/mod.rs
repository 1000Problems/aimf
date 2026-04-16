use std::fmt::Write;

use crate::types::*;
use crate::partitioner::PartitionResult;
use crate::hot_strategy;
use crate::scanner::ScannedFile;

/// Emit a complete AIMF document as a string.
pub fn emit(
    config: &CompilerConfig,
    files: &[ScannedFile],
    partition: &PartitionResult,
) -> String {
    let mut out = String::with_capacity(64 * 1024);

    emit_header(&mut out, config, &partition);
    emit_groups(&mut out, &partition.groups);
    emit_index(&mut out, &partition.resources);
    emit_hot_memory(&mut out, config, files, &partition);

    out
}

/// Emit the @H header section.
fn emit_header(out: &mut String, config: &CompilerConfig, partition: &PartitionResult) {
    let _ = writeln!(out, "@H");
    let _ = writeln!(out, "V:1");
    let _ = writeln!(out, "R:{}", partition.resources.len());
    let _ = writeln!(out, "HOT:{}", partition.hot_ids.len());

    if !partition.groups.is_empty() {
        let _ = writeln!(out, "GRP:{}", partition.groups.len());
    }

    if let Some(ref repo) = config.repo {
        let _ = writeln!(out, "REPO:{}", repo);
    }
    if let Some(ref branch) = config.branch {
        let _ = writeln!(out, "BRANCH:{}", branch);
    }

    // Timestamp
    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let _ = writeln!(out, "TS:{}", ts);
    let _ = writeln!(out, "ROOT:{}", config.root.display());

    // Sharding marker
    if partition.resources.len() > config.shard_threshold {
        let _ = writeln!(out, "SHARD:1/1");
    }

    let _ = writeln!(out);
}

/// Emit the @G groups section.
fn emit_groups(out: &mut String, groups: &[Group]) {
    if groups.is_empty() {
        return;
    }

    let _ = writeln!(out, "@G");
    for group in groups {
        let patterns = group.patterns.join(",");
        let _ = writeln!(out, "{}|{}|{}", group.id, group.label, patterns);
    }
    let _ = writeln!(out);
}

/// Emit the @I index section.
fn emit_index(out: &mut String, resources: &[Resource]) {
    let _ = writeln!(out, "@I");
    for r in resources {
        let _ = writeln!(
            out,
            "{}|{}|{}|{}|{}|{}|{}|{}",
            r.id,
            r.resource_type.as_str(),
            r.load_when.as_str(),
            r.path.display(),
            r.size,
            r.hash,
            r.group_id,
            r.hint,
        );
    }
    let _ = writeln!(out);
}

/// Emit the @M hot memory section.
fn emit_hot_memory(
    out: &mut String,
    config: &CompilerConfig,
    files: &[ScannedFile],
    partition: &PartitionResult,
) {
    // Emit CTX entry
    let _ = writeln!(out, "@M CTX");
    let _ = writeln!(out, "<<task=initial compilation;focus=all>>");
    let _ = writeln!(out);

    // Emit ARCH summary
    let _ = writeln!(out, "@M ARCH");
    let _ = write!(out, "<<");
    emit_architecture_summary(out, partition);
    let _ = writeln!(out, ">>");
    let _ = writeln!(out);

    // Emit DEPS summary
    let _ = writeln!(out, "@M DEPS");
    let _ = write!(out, "<<");
    emit_deps_summary(out, partition, files, config);
    let _ = writeln!(out, ">>");
    let _ = writeln!(out);

    // Emit hot file entries
    for (hot_id, strategy) in &partition.hot_strategies {
        // Find the corresponding scanned file
        let resource = partition.resources.iter().find(|r| &r.id == hot_id);
        if let Some(resource) = resource {
            let scanned = files.iter().find(|f| f.path == resource.path);
            if let Some(scanned) = scanned {
                let payload = hot_strategy::apply_strategy(&scanned.abs_path, strategy);
                let _ = writeln!(out, "@M {}", hot_id);
                let _ = writeln!(out, "<<{}>>\n", payload);
            }
        }
    }
}

/// Generate a simple architecture summary from the resource structure.
fn emit_architecture_summary(out: &mut String, partition: &PartitionResult) {
    let _ = write!(out, "Project structure: {} resources in {} groups.\n",
        partition.resources.len(), partition.groups.len());

    // Count by type
    let mut type_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for r in &partition.resources {
        *type_counts.entry(r.resource_type.as_str()).or_insert(0) += 1;
    }

    let _ = write!(out, "Composition: ");
    let counts: Vec<String> = type_counts.iter()
        .map(|(k, v)| format!("{} {}", v, k))
        .collect();
    let _ = write!(out, "{}.\n", counts.join(", "));

    // List groups with resource counts
    for group in &partition.groups {
        let count = partition.resources.iter()
            .filter(|r| r.group_id == group.id)
            .count();
        if count > 0 {
            let _ = write!(out, "{} ({}): {} files.\n", group.label, group.id, count);
        }
    }

    // List entry points
    let entries: Vec<&Resource> = partition.resources.iter()
        .filter(|r| matches!(r.load_when, LoadWhen::Always | LoadWhen::OnEdit))
        .collect();
    if !entries.is_empty() {
        let _ = write!(out, "Entry points: ");
        let names: Vec<String> = entries.iter()
            .map(|r| format!("{} ({})", r.path.display(), r.id))
            .collect();
        let _ = write!(out, "{}.", names.join(", "));
    }
}

/// Generate a dependency summary from config files.
fn emit_deps_summary(
    out: &mut String,
    partition: &PartitionResult,
    files: &[ScannedFile],
    config: &CompilerConfig,
) {
    // Look for common dependency files
    let dep_files = ["Cargo.toml", "package.json", "requirements.txt", "go.mod", "Gemfile", "pyproject.toml"];

    for dep_file in &dep_files {
        let resource = partition.resources.iter().find(|r| {
            r.path.file_name()
                .map(|f| f.to_string_lossy().to_lowercase() == dep_file.to_lowercase())
                .unwrap_or(false)
        });

        if let Some(resource) = resource {
            let scanned = files.iter().find(|f| f.path == resource.path);
            if let Some(scanned) = scanned {
                if let Ok(content) = std::fs::read_to_string(&scanned.abs_path) {
                    // For now, include the first 20 lines of dependency files
                    let lines: Vec<&str> = content.lines().take(20).collect();
                    let _ = write!(out, "[{}]\n{}\n", dep_file, lines.join("\n"));
                }
            }
        }
    }

    if out.ends_with("<<") {
        let _ = write!(out, "no dependency files detected");
    }
}
