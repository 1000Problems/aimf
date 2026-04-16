use std::path::Path;
use crate::types::*;
use crate::scanner::ScannedFile;
use crate::classifier;

/// Result of partitioning: classified resources with group assignments and hot/cold decisions.
pub struct PartitionResult {
    pub groups: Vec<Group>,
    pub resources: Vec<Resource>,
    pub hot_ids: Vec<String>,       // ResourceIDs that should be in @M
    pub hot_strategies: Vec<(String, HotStrategy)>, // ResourceID -> strategy
}

/// Default group definitions for when no custom groups are provided.
fn default_groups() -> Vec<Group> {
    vec![
        Group { id: "G1".into(), label: "core".into(), patterns: vec!["src/lib/**".into(), "src/core/**".into()] },
        Group { id: "G2".into(), label: "api".into(), patterns: vec!["src/api/**".into(), "src/routes/**".into(), "src/handlers/**".into()] },
        Group { id: "G3".into(), label: "cfg".into(), patterns: vec!["*.toml".into(), "*.yaml".into(), "*.yml".into(), "*.json".into()] },
        Group { id: "G4".into(), label: "test".into(), patterns: vec!["tests/**".into(), "test/**".into(), "__tests__/**".into()] },
        Group { id: "G5".into(), label: "doc".into(), patterns: vec!["docs/**".into(), "*.md".into()] },
        Group { id: "G6".into(), label: "ui".into(), patterns: vec!["src/ui/**".into(), "src/components/**".into(), "src/pages/**".into()] },
        Group { id: "G7".into(), label: "build".into(), patterns: vec!["Cargo.*".into(), "Makefile".into(), "Dockerfile".into(), "package.json".into()] },
        Group { id: "G8".into(), label: "other".into(), patterns: vec!["*".into()] },
    ]
}

/// Assign a file to a group based on glob pattern matching.
fn assign_group(path: &Path, groups: &[Group]) -> String {
    let path_str = path.to_string_lossy();
    for group in groups {
        for pattern in &group.patterns {
            if let Ok(matcher) = glob::Pattern::new(pattern) {
                if matcher.matches(&path_str) {
                    return group.id.clone();
                }
            }
        }
    }
    "-".to_string()
}

/// Decide the LoadWhen policy for a resource.
fn decide_load_when(resource_type: &ResourceType, path: &Path, size: u64) -> LoadWhen {
    // Binaries are never loaded into context
    if *resource_type == ResourceType::BIN {
        return LoadWhen::Never;
    }

    // Very large files default to on_request
    if size > 50_000 {
        return LoadWhen::OnRequest;
    }

    match resource_type {
        ResourceType::CFG => {
            // Small configs are always hot
            if size < 4096 {
                LoadWhen::Always
            } else {
                LoadWhen::OnRequest
            }
        }
        ResourceType::SRC => {
            if classifier::is_entry_point(path) {
                LoadWhen::OnEdit
            } else {
                LoadWhen::OnRequest
            }
        }
        ResourceType::DOC => {
            let filename = path.file_name()
                .map(|f| f.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if filename.starts_with("readme") {
                LoadWhen::Always
            } else {
                LoadWhen::OnRequest
            }
        }
        ResourceType::TEST => LoadWhen::OnRequest,
        ResourceType::MRM => LoadWhen::OnRequest,
        ResourceType::META => LoadWhen::OnGroup,
        _ => LoadWhen::OnRequest,
    }
}

/// Decide the hot payload strategy for a resource.
fn decide_hot_strategy(resource_type: &ResourceType, size: u64, config: &CompilerConfig) -> HotStrategy {
    if size <= config.max_hot_file_size {
        return HotStrategy::Full;
    }

    match resource_type {
        ResourceType::SRC => {
            if size <= config.summary_threshold {
                HotStrategy::Signature
            } else {
                HotStrategy::Head(50) // first 50 lines
            }
        }
        ResourceType::DOC => {
            if size <= config.summary_threshold {
                HotStrategy::Head(100)
            } else {
                HotStrategy::Summary
            }
        }
        _ => HotStrategy::Head(30),
    }
}

/// Main partitioning pipeline: classify, group, and decide hot/cold for all scanned files.
pub fn partition(files: &[ScannedFile], config: &CompilerConfig) -> PartitionResult {
    let groups = match &config.custom_groups {
        Some(custom) => custom.iter().enumerate().map(|(i, g)| Group {
            id: format!("G{}", i + 1),
            label: g.label.clone(),
            patterns: g.patterns.clone(),
        }).collect(),
        None => default_groups(),
    };

    let mut resources = Vec::new();
    let mut hot_candidates: Vec<(usize, u64)> = Vec::new(); // (index, size) for scoring

    for (i, file) in files.iter().enumerate() {
        let resource_type = classifier::classify(&file.path);
        let load_when = decide_load_when(&resource_type, &file.path, file.size);
        let group_id = assign_group(&file.path, &groups);
        let hint = classifier::generate_hint(&file.path, &resource_type);
        let id = format!("F{}", i + 1);

        let resource = Resource {
            id: id.clone(),
            resource_type: resource_type.clone(),
            load_when: load_when.clone(),
            path: file.path.clone(),
            size: file.size,
            hash: file.hash.clone(),
            group_id,
            hint,
        };

        // Collect hot candidates
        if matches!(load_when, LoadWhen::Always | LoadWhen::OnEdit) {
            hot_candidates.push((i, file.size));
        }

        resources.push(resource);
    }

    // Sort hot candidates by priority: Always first, then OnEdit, then by size (smaller first)
    hot_candidates.sort_by(|a, b| {
        let a_res = &resources[a.0];
        let b_res = &resources[b.0];
        let a_priority = match a_res.load_when {
            LoadWhen::Always => 0,
            LoadWhen::OnEdit => 1,
            _ => 2,
        };
        let b_priority = match b_res.load_when {
            LoadWhen::Always => 0,
            LoadWhen::OnEdit => 1,
            _ => 2,
        };
        a_priority.cmp(&b_priority).then(a.1.cmp(&b.1))
    });

    // Limit hot entries to config max
    hot_candidates.truncate(config.max_hot_files);

    let hot_ids: Vec<String> = hot_candidates.iter()
        .map(|(i, _)| resources[*i].id.clone())
        .collect();

    let hot_strategies: Vec<(String, HotStrategy)> = hot_candidates.iter()
        .map(|(i, _)| {
            let r = &resources[*i];
            let strategy = decide_hot_strategy(&r.resource_type, r.size, config);
            (r.id.clone(), strategy)
        })
        .collect();

    PartitionResult {
        groups,
        resources,
        hot_ids,
        hot_strategies,
    }
}
