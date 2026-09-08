use super::*;

pub(crate) fn load(path: &Path) -> Result<ProjectFile, String> {
    super::load_with_cache_root(path, || {
        Ok(std::env::temp_dir().join(format!("rexafs-project-test-cache-{}", std::process::id())))
    })
}

use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rexafs-project-tests-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn join(&self, path: &str) -> PathBuf {
        self.0.join(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/projects")
        .join(name)
}
fn json_file(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn copy_inputs(dir: &Path) {
    for name in ["data/cu_150k.xmu", "data/second.xmu", "feff/feff0001.dat"] {
        let dest = dir.join(name);
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::fs::copy(fixture(name), dest).unwrap();
    }
}
fn specimen(dir: &Path) -> ProjectFile {
    copy_inputs(dir);
    std::fs::copy(fixture("rexafs-0.1.0-links.rxs"), dir.join("session.rxs")).unwrap();
    load(&dir.join("session.rxs")).unwrap()
}

#[test]
fn embedded_restore_uses_the_injected_cache_and_links_need_no_cache() {
    let temp = Temp::new();
    let project = specimen(&temp.join("source"));
    super::load_with_cache_root(&temp.join("source/session.rxs"), || {
        panic!("linked projects must not initialize a cache")
    })
    .unwrap();
    let portable = temp.join("portable.rxs");
    save_with_storage(&portable, &project, DataStorage::Embedded).unwrap();
    let mut restored_paths = Vec::new();
    for name in ["cache-a", "cache-b"] {
        let root = temp.join(name);
        let restored = super::load_with_cache_root(&portable, || Ok(root.clone())).unwrap();
        let source = restored.spectrum_file.unwrap();
        assert!(source.starts_with(root));
        assert_eq!(
            std::fs::read(&source).unwrap(),
            std::fs::read(project.spectrum_file.as_ref().unwrap()).unwrap()
        );
        restored_paths.push(source);
    }
    assert_ne!(restored_paths[0], restored_paths[1]);
}

#[test]
fn parser_totals_and_line_examples_survive_linked_and_embedded_reopen() {
    let temp = Temp::new();
    let mut project = specimen(&temp.join("source"));
    project.assign_group_ids();
    let path = project.spectrum_file.clone().unwrap();
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str(&"\nmalformed row".repeat(9));
    std::fs::write(&path, text).unwrap();
    let preview = crate::params::preview_import(&path, &project.params.import).unwrap();
    assert_eq!(preview.diagnostics.malformed_rows.count, 9);
    assert_eq!(preview.diagnostics.malformed_rows.examples.len(), 5);
    let group = project
        .source_groups
        .iter()
        .find(|g| g.path == path)
        .unwrap()
        .id
        .clone();
    project.parser_evidence.insert(
        group.clone(),
        crate::source_evidence::ParserRecord {
            path: path.clone(),
            channel: preview.resolved.mode,
            mapping_revision: crate::import_recipes::mapping_revision(&project.params.import),
            diagnostics: preview.diagnostics.clone(),
            declared_edge: None,
        },
    );
    for mode in [DataStorage::Paths, DataStorage::Embedded] {
        let saved = temp.join(if mode == DataStorage::Paths {
            "linked.rxs"
        } else {
            "embedded.rxs"
        });
        save_with_storage(&saved, &project, mode).unwrap();
        let restored = load(&saved).unwrap();
        let record = &restored.parser_evidence[&group];
        assert_eq!(record.diagnostics, preview.diagnostics);
        assert_eq!(record.path, *restored.spectrum_file.as_ref().unwrap());
        assert!(record.matches(&restored.params.import));
        let mut edited = restored.params.import.clone();
        edited.axis = crate::import_mapping::AxisConversion::EnergyKev;
        assert!(
            !record.matches(&edited),
            "old diagnostics cannot describe another mapping"
        );
    }
}
fn state(project: &ProjectFile) -> Value {
    let mut project = project.clone();
    let origins = project.source_origins.clone();
    storage::map_paths(&mut project, &mut |p| {
        let source = origins.get(p).cloned().unwrap_or_else(|| p.to_owned());
        // Embedded reloads may spell the same original file through a directory
        // alias (notably /tmp vs /private/tmp on macOS). Compare file identity
        // after undoing the cache mapping. Resolve an existing ancestor when
        // the portability test has deliberately removed the original files.
        Ok(storage::resolved_location(&source))
    })
    .unwrap();
    let mut value = serde_json::to_value(project).unwrap();
    for key in ["header", "embedded"] {
        value.as_object_mut().unwrap().remove(key);
    }
    value
}

#[test]
fn recipes_exact_applications_and_pending_sources_survive_conflicting_machine_library() {
    use crate::{
        app::{
            import_review::PendingSource,
            import_state::{IntakeBatch, IntakeState, SourceOutcome},
        },
        group_identity::{GroupId, SourceGroup},
        import_recipes::*,
        params::{DetectionMode, ImportConfig},
    };
    for mode in [DataStorage::Paths, DataStorage::Embedded] {
        let dir = Temp::new();
        let source = dir.join("sample.dat");
        std::fs::write(
            &source,
            "# energy i0 it ir\n8900 10 5 2\n9000 12 5 2\n9100 14 5 2\n",
        )
        .unwrap();
        let source = source.canonicalize().unwrap();
        let pending = source.parent().unwrap().join("unreadable.dat");
        let table = crate::params::preview_import(&source, &ImportConfig::default()).unwrap();
        let recipe = RecipeVersion::from_review(
            "Saved beamline",
            RecipeScope::for_source(&source, None),
            &table,
            DetectionMode::Transmission,
            &[
                ImportConfig {
                    mode: DetectionMode::Transmission,
                    ..Default::default()
                },
                ImportConfig {
                    mode: DetectionMode::Reference,
                    ..Default::default()
                },
            ],
            true,
        )
        .unwrap();
        let primary = GroupId::source(&source, DetectionMode::Transmission);
        let reference = GroupId::source(&source, DetectionMode::Reference);
        let mappings = recipe.channels.clone();
        let params = PipelineParams {
            import: mappings[0].clone(),
            ..Default::default()
        };
        let mut project = ProjectFile {
            spectrum_file: Some(source.clone()),
            raw_files: vec![source.clone()],
            params: params.clone(),
            source_groups: vec![SourceGroup {
                id: primary.clone(),
                path: source.clone(),
                channel: DetectionMode::Transmission,
            }],
            overrides: vec![ParamOverride {
                path: source.clone(),
                params,
            }],
            derived: vec![DerivedSpectrum {
                id: 1,
                group_id: Some(reference.clone()),
                source: Some(source.clone()),
                params: Some(PipelineParams {
                    import: mappings[1].clone(),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            import_history: vec![IntakeBatch {
                paths: vec![source.parent().unwrap().into()],
                stopped: false,
                finished: true,
                dropped_queue: 0,
                sources: std::collections::BTreeMap::from([
                    (
                        source.clone(),
                        SourceOutcome {
                            created: vec![primary.clone(), reference.clone()],
                            ..Default::default()
                        },
                    ),
                    (
                        pending.clone(),
                        SourceOutcome {
                            pending: Some(PendingSource {
                                detection: None,
                                suggestion: None,
                                reason: "Locate source".into(),
                            }),
                            ..Default::default()
                        },
                    ),
                ]),
            }],
            ..Default::default()
        };
        project.imports.recipes.remember(recipe.clone()).unwrap();
        project.imports.applications.push(ImportApplication {
            id: "application:original-batch".into(),
            batch: 0,
            recipe: recipe.reference.clone(),
            members: [primary.clone(), reference.clone()]
                .into_iter()
                .zip(&mappings)
                .map(|(group, mapping)| ApplicationMember {
                    source_id: primary.clone(),
                    path: source.clone(),
                    group,
                    channel: mapping.mode,
                    mapping_revision: mapping_revision(mapping),
                })
                .collect(),
        });
        let saved = dir.join("recipe.rxs");
        save_with_storage(&saved, &project, mode).unwrap();

        let mut machine = crate::settings::UserSettings::default();
        machine.import_recipes.remember(recipe.clone()).unwrap();
        let mut conflict = recipe.clone();
        conflict.channels[0].axis = crate::import_mapping::AxisConversion::EnergyKev;
        machine.import_recipes.commit_review(conflict);
        let machine: crate::settings::UserSettings =
            serde_json::from_value(serde_json::to_value(machine).unwrap()).unwrap();
        assert_eq!(machine.import_recipes.versions.len(), 2);

        let mut reopened = load(&saved).unwrap();
        reopened.assign_group_ids();
        assert_eq!(
            reopened.imports.recipes.get(&recipe.reference).unwrap(),
            &recipe
        );
        assert!(reopened.overrides[0].params.import == mappings[0]);
        assert!(reopened.derived[0].params.as_ref().unwrap().import == mappings[1]);
        let application = reopened.imports.application_for(&reference).unwrap();
        assert_eq!(application.batch, 0);
        assert_eq!(application.members.len(), 2);
        assert_eq!(application.members[0].group, primary);
        assert_eq!(
            application.members[1].mapping_revision,
            mapping_revision(&mappings[1])
        );
        assert_eq!(application.members[0].path, reopened.overrides[0].path);
        let history = IntakeState::from_history(reopened.import_history.clone());
        assert!(history.is_pending(&pending));
        assert_eq!(
            history.history[0].sources[&reopened.overrides[0].path].created,
            vec![primary, reference]
        );
        assert_eq!(history.receipt, Some(0));
    }
}

#[test]
fn assistant_conversations_roundtrip_in_both_storage_modes_and_limit_at_save() {
    use super::assistant::{Conversation, ConversationEntry, ConversationMode};
    for mode in [DataStorage::Paths, DataStorage::Embedded] {
        let dir = Temp::new();
        let mut project = specimen(&dir.0);
        for n in 0..2 {
            let mut conversation = Conversation::new(&format!("Check fit {n}"), n == 1);
            conversation.id = format!("conversation-{n}");
            conversation.updated_at = format!("2026-09-08T00:0{n}:00Z");
            conversation.thread_id = Some(format!("server-thread-{n}"));
            conversation.entries = vec![
                ConversationEntry::User {
                    text: format!("Check fit {n}"),
                    edit: n == 1,
                },
                ConversationEntry::Thinking {
                    id: "reason".into(),
                    text: "Inspect normalization first.".into(),
                },
                ConversationEntry::Assistant {
                    id: "answer".into(),
                    text: "The ranges are consistent.".into(),
                },
            ];
            project.assistant.upsert(conversation);
        }
        let path = dir.join("conversations.rxs");
        save_with_storage(&path, &project, mode).unwrap();
        let restored = load(&path).unwrap();
        assert_eq!(
            restored.assistant.conversations,
            project.assistant.conversations
        );
        assert_eq!(
            restored.assistant.conversations[0].mode,
            ConversationMode::Edit
        );
        assert_eq!(
            json_file(&path)["assistant"]["conversations"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        project.assistant.limit = Some(1);
        save_with_storage(&path, &project, mode).unwrap();
        let restored = load(&path).unwrap();
        assert_eq!(restored.assistant.conversations.len(), 1);
        assert_eq!(restored.assistant.conversations[0].id, "conversation-1");
        project.assistant.limit = Some(0);
        save_with_storage(&path, &project, mode).unwrap();
        assert!(load(&path).unwrap().assistant.conversations.is_empty());
    }
}

/// Explicit maintainer operation; never modifies a retained fixture by default.
#[test]
#[ignore = "requires REXAFS_FIXTURE_OUTPUT; writes a new release fixture pair"]
fn write_release_compatibility_fixtures() {
    use super::assistant::{Conversation, ConversationEntry, SavedActivityState};
    let output = std::env::var_os("REXAFS_FIXTURE_OUTPUT")
        .map(PathBuf::from)
        .expect("set REXAFS_FIXTURE_OUTPUT to an empty fixture output directory");
    std::fs::create_dir_all(&output).unwrap();
    let mut project = load(&fixture("rexafs-0.1.3-links.rxs")).unwrap();
    if let Some(path) = project.fit_paths.first_mut() {
        path.degen = "4".into();
    }
    for (index, edit) in [false, true].into_iter().enumerate() {
        let mut conversation = Conversation::new("Synthetic saved-conversation fixture", edit);
        conversation.id = format!("fixture-conversation-{index}");
        conversation.started_at = "2026-09-08T00:00:00Z".into();
        conversation.updated_at = format!("2026-09-08T00:0{index}:00Z");
        conversation.entries = vec![
            ConversationEntry::User {
                text: "Demonstrate retained transcript entries.".into(),
                edit,
            },
            ConversationEntry::Thinking {
                id: "thinking-1".into(),
                text: "Synthetic persistence example.".into(),
            },
            ConversationEntry::Activity {
                id: "activity-1".into(),
                label: "Read project state".into(),
                tool: "xray_get_state".into(),
                state: SavedActivityState::Done,
            },
            ConversationEntry::Receipt {
                header: "Synthetic recorded change".into(),
                lines: vec!["N = 4 (persistence example)".into()],
                scope: "This spectrum".into(),
                state: "Recorded".into(),
                navigation: json!({}),
            },
            ConversationEntry::Assistant {
                id: "answer-1".into(),
                text: "This is fixture content, not a scientific fit recommendation.".into(),
            },
            ConversationEntry::Status {
                text: "Completed".into(),
            },
        ];
        project.assistant.upsert(conversation);
    }
    for (suffix, mode) in [
        ("links", DataStorage::Paths),
        ("embedded", DataStorage::Embedded),
    ] {
        let path = output.join(format!("rexafs-{}-{suffix}.rxs", env!("CARGO_PKG_VERSION")));
        assert!(!path.exists(), "never overwrite a retained release fixture");
        save_with_storage(&path, &project, mode).unwrap();
        let restored = load(&path).unwrap();
        assert_eq!(
            restored.assistant.conversations,
            project.assistant.conversations
        );
        assert_eq!(restored.fit_paths[0].degen, "4");
    }
}

#[test]
fn format_one_defaults_keep_their_released_meaning() {
    fn preserved(expected: &Value, actual: &Value) -> bool {
        match expected.as_object() {
            Some(fields) => {
                actual.is_object()
                    && fields.iter().all(|(key, value)| {
                        actual
                            .get(key)
                            .is_some_and(|actual| preserved(value, actual))
                    })
            }
            None => expected == actual,
        }
    }
    let actual = state(&load(&fixture("minimal-v1.rxs")).unwrap());
    let expected = json_file(&fixture("format-v1-defaults.json"));
    assert!(
        preserved(&expected, &actual),
        "Format 1 defaults changed: migrate existing files instead of reinterpreting omitted fields. Current defaults: {}",
        serde_json::to_string(&actual).unwrap()
    );
}

#[test]
fn every_release_fixture_loads_saves_and_reopens_without_losing_state() {
    let manifest = json_file(&fixture("manifest.json"));
    let invalid = manifest["invalid_projects"].as_array().unwrap();
    let temp = Temp::new();
    for name in manifest["sha256"]
        .as_object()
        .unwrap()
        .keys()
        .filter(|n| n.ends_with(".rxs"))
    {
        let original = std::fs::read(fixture(name)).unwrap();
        if invalid.contains(&json!(name)) {
            assert!(load(&fixture(name)).is_err(), "{name}");
        } else {
            let project = load(&fixture(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
            let saved = temp.join(name);
            save(&saved, &project).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(std::fs::read(&saved).unwrap().starts_with(b"{\"header\":"));
            let reopened = load(&saved).unwrap();
            assert_eq!(state(&project), state(&reopened), "{name}");
            assert_eq!(
                project.header.as_ref().unwrap().created_utc,
                reopened.header.as_ref().unwrap().created_utc
            );
            assert_eq!(project.data_storage, reopened.data_storage);
        }
        assert_eq!(
            original,
            std::fs::read(fixture(name)).unwrap(),
            "fixture was modified"
        );
    }
}

#[test]
fn linked_project_moves_with_data_and_save_as_rebases_every_owned_path() {
    let temp = Temp::new();
    let old = temp.join("old");
    let project = specimen(&old);
    save(&old.join("session.rxs"), &project).unwrap();
    let disk = json_file(&old.join("session.rxs"));
    assert_eq!(disk["header"]["storage"], "paths");
    assert_eq!(disk["header"]["path_base"], "project_directory");
    assert_eq!(disk["spectrum_file"], "data/cu_150k.xmu");
    assert!(disk.get("embedded").is_none());
    let moved = temp.join("moved");
    std::fs::rename(old, &moved).unwrap();
    let reopened = load(&moved.join("session.rxs")).unwrap();
    let mut paths = Vec::new();
    storage::map_paths(&mut reopened.clone(), &mut |p| {
        paths.push(p.to_owned());
        Ok(p.to_owned())
    })
    .unwrap();
    assert!(paths.len() >= 8);
    assert!(paths.iter().all(|p| p.starts_with(&moved) && p.exists()));
    std::fs::create_dir(moved.join("projects")).unwrap();
    let save_as = moved.join("projects/new.rxs");
    save(&save_as, &reopened).unwrap();
    assert_eq!(json_file(&save_as)["spectrum_file"], "../data/cu_150k.xmu");
    assert_eq!(state(&reopened), state(&load(&save_as).unwrap()));
}

#[test]
#[cfg(unix)]
fn saving_through_a_directory_alias_keeps_links_portable() {
    use std::os::unix::fs::symlink;
    let temp = Temp::new();
    let real = temp.join("real");
    let project = specimen(&real);
    let alias = temp.join("alias");
    symlink(&real, &alias).unwrap();
    let mut opened_through_alias = project.clone();
    storage::map_paths(&mut opened_through_alias, &mut |p| {
        Ok(alias.join(p.strip_prefix(&real).unwrap()))
    })
    .unwrap();
    let file = real.join("native-dialog.rxs");
    save(&file, &opened_through_alias).unwrap();
    assert_eq!(json_file(&file)["source_dir"], "data");
    assert_eq!(json_file(&file)["spectrum_file"], "data/cu_150k.xmu");
    std::fs::remove_file(alias).unwrap();
    let moved = temp.join("moved");
    std::fs::rename(real, &moved).unwrap();
    let restored = load(&moved.join("native-dialog.rxs")).unwrap();
    assert!(restored.spectrum_file.unwrap().is_file());
}

#[test]
fn embedded_project_is_lossless_self_contained_and_can_be_saved_again() {
    let temp = Temp::new();
    let source = temp.join("original");
    let mut project = specimen(&source);
    // These workspace files must travel with the paths, including raw bytes
    // that are not representable as UTF-8 JSON text.
    let engine = b"engine: refeff\r\n\xff\x00";
    std::fs::write(source.join("feff/engine.txt"), engine).unwrap();
    std::fs::write(source.join("feff/crystal.json"), b"{\"name\":\"Cu\"}").unwrap();
    // Cover nested path-map keys as well as values, in current and historical fits.
    project.joint.datasets[0].expressions.insert(
        project.fit_paths[0].file.clone(),
        project.fit_paths[0].clone(),
    );
    project.fit_history[0].joint = Some(project.joint.clone());
    let expected = state(&project);
    let before =
        crate::params::process_file(project.spectrum_file.as_ref().unwrap(), &project.params)
            .unwrap();
    let portable = temp.join("portable.rxs");
    save_with_storage(&portable, &project, DataStorage::Embedded).unwrap();
    let stored = json_file(&portable);
    assert_eq!(stored["header"]["storage"], "embedded");
    assert_eq!(
        stored["embedded"].as_object().unwrap().len(),
        4,
        "identical raw files deduplicate"
    );
    let file_count = stored["header"]["files"].as_array().unwrap().len();
    assert_eq!(file_count, 5);
    std::fs::remove_dir_all(&source).unwrap();
    let reopened = load(&portable).unwrap();
    assert_eq!(state(&reopened), expected);
    assert_eq!(
        std::fs::read(reopened.spectrum_file.as_ref().unwrap()).unwrap(),
        std::fs::read(fixture("data/cu_150k.xmu")).unwrap()
    );
    assert_eq!(
        std::fs::read(&reopened.fit_paths[0].file).unwrap(),
        std::fs::read(fixture("feff/feff0001.dat")).unwrap()
    );
    assert_eq!(
        std::fs::read(reopened.feff_workspace.as_ref().unwrap().join("engine.txt")).unwrap(),
        engine
    );
    assert_eq!(
        reopened.fit_paths[0].file.parent(),
        reopened.feff_workspace.as_deref()
    );
    let after =
        crate::params::process_file(reopened.spectrum_file.as_ref().unwrap(), &reopened.params)
            .unwrap();
    assert_eq!(before.e0(), after.e0());
    assert_eq!(
        before
            .chi()
            .unwrap()
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        after
            .chi()
            .unwrap()
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>()
    );
    let resaved = temp.join("resaved.rxs");
    save(&resaved, &reopened).unwrap();
    assert_eq!(state(&load(&resaved).unwrap()), expected);
    assert_eq!(
        stored["header"]["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| (&f["path"], &f["modified_unix_seconds"]))
            .collect::<Vec<_>>(),
        json_file(&resaved)["header"]["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| (&f["path"], &f["modified_unix_seconds"]))
            .collect::<Vec<_>>()
    );
}

#[test]
fn default_paths_can_record_missing_sources_but_embedding_cannot_drop_them() {
    let temp = Temp::new();
    let project = ProjectFile {
        spectrum_file: Some(temp.join("missing.xmu")),
        ..Default::default()
    };
    assert_eq!(project.data_storage, DataStorage::Paths);
    let path = temp.join("session.rxs");
    save(&path, &project).unwrap();
    let before = std::fs::read(&path).unwrap();
    let header = json_file(&path)["header"].clone();
    assert_eq!(header["files"][0]["path"], "missing.xmu");
    assert!(header["files"][0].get("sha256").is_none());
    assert!(load(&path).is_ok());
    assert!(save_with_storage(&path, &project, DataStorage::Embedded).is_err());
    assert_eq!(before, std::fs::read(path).unwrap());
}

#[test]
fn metadata_records_source_comments_checksums_and_writer() {
    let temp = Temp::new();
    let project = specimen(&temp.join("source"));
    let path = temp.join("session.rxs");
    let header = save_with_storage(&path, &project, DataStorage::Paths).unwrap();
    assert_eq!(header.software, "rexafs");
    assert_eq!(header.software_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(header.format_version, PROJECT_VERSION);
    assert!(chrono::DateTime::parse_from_rfc3339(&header.saved_utc).is_ok());
    let raw = header
        .files
        .iter()
        .find(|f| f.path.ends_with("cu_150k.xmu"))
        .unwrap();
    assert_eq!(raw.bytes, Some(20737));
    assert_eq!(
        raw.sha256.as_deref(),
        Some("c309e53ec6b681024718d5c25694c6426818725974afd7b124a2f076be618cf2")
    );
    assert!(raw.source_header.iter().any(|l| l.contains("Cu foil 150K")));
    assert!(raw.modified_unix_seconds.is_some());
}

#[test]
fn malformed_headers_payloads_and_unsafe_archive_paths_are_rejected() {
    let temp = Temp::new();
    let original = json_file(&fixture("rexafs-0.1.0-embedded.rxs"));
    let mut bad = vec![];
    for path in [
        "../escape.xmu",
        "/tmp/escape.xmu",
        "raw/../../escape.xmu",
        "raw\\..\\escape.xmu",
        "C:/escape.xmu",
    ] {
        let mut v = original.clone();
        v["header"]["files"][0]["archive_path"] = json!(path);
        bad.push(v);
    }
    let mut v = original.clone();
    v["header"]["files"][1]["archive_path"] = v["header"]["files"][0]["archive_path"].clone();
    bad.push(v);
    let mut v = original.clone();
    v["header"]["files"][0]["bytes"] = json!(1);
    bad.push(v);
    let mut v = original.clone();
    v["header"]["files"][0]["bytes"] = json!(u64::MAX);
    bad.push(v);
    let mut v = original.clone();
    v["header"]["files"].as_array_mut().unwrap().remove(0);
    bad.push(v);
    let mut v = original.clone();
    v["embedded"] = json!({});
    bad.push(v);
    let mut v = original.clone();
    v["embedded"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .for_each(|v| *v = json!("invalid!"));
    bad.push(v);
    let mut v = original.clone();
    v["header"]["storage"] = json!("paths");
    bad.push(v);
    let mut v = original.clone();
    v["header"]["path_base"] = json!("cwd");
    bad.push(v);
    let mut v = original.clone();
    v.as_object_mut().unwrap().remove("header");
    bad.push(v);
    let mut v = original.clone();
    v["header"]["format_version"] = json!(2);
    bad.push(v);
    for (i, v) in bad.into_iter().enumerate() {
        let path = temp.join(&format!("bad-{i}.rxs"));
        let bytes = serde_json::to_vec(&v).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        assert!(load(&path).is_err(), "accepted malformed case {i}");
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    assert!(!temp.join("escape.xmu").exists());
}

#[test]
fn only_rxs_is_supported_and_future_formats_cannot_be_overwritten() {
    let temp = Temp::new();
    for name in ["session.rxs", "session.RXS"] {
        assert!(is_project(Path::new(name)));
    }
    for name in ["session.xtproj", "session.xproj", "session.json", "rxs"] {
        assert!(!is_project(Path::new(name)));
        assert!(save(&temp.join(name), &ProjectFile::default()).is_err());
        assert!(load(&temp.join(name)).is_err());
    }
    let path = temp.join("future.rxs");
    let bytes = std::fs::read(fixture("future-version.rxs")).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    assert!(
        save(&path, &ProjectFile::default())
            .unwrap_err()
            .contains("newer")
    );
    assert_eq!(bytes, std::fs::read(path).unwrap());
    for invalid in [
        "{}",
        "[]",
        "{\"version\":0}",
        "{\"version\":-1}",
        "{\"version\":1.5}",
    ] {
        assert!(parse(invalid).is_err());
    }
}

#[test]
fn replacements_keep_exact_backup_and_partial_writes_leave_previous_file() {
    use std::io::Write;
    let temp = Temp::new();
    let path = temp.join("session.rxs");
    save(&path, &ProjectFile::default()).unwrap();
    let original = std::fs::read(&path).unwrap();
    let project = ProjectFile {
        derived: vec![DerivedSpectrum {
            label: "retained".into(),
            energy: vec![1.0],
            mu: vec![2.0],
            ..Default::default()
        }],
        ..Default::default()
    };
    save(&path, &project).unwrap();
    assert_eq!(
        std::fs::read(temp.join("session.rxs.bak")).unwrap(),
        original
    );
    let current = std::fs::read(&path).unwrap();
    let error = replace_with(&path, |file| {
        file.write_all(b"partial")?;
        Err(std::io::Error::other("injected failure"))
    });
    assert!(error.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), current);
    assert_eq!(
        std::fs::read_dir(&temp.0).unwrap().count(),
        2,
        "temporary write leaked"
    );
}

#[test]
fn compact_writer_preserves_all_finite_double_bits_and_opaque_metadata() {
    let mut samples = vec![
        -0.0,
        f64::from_bits(1),
        f64::MIN_POSITIVE,
        1e-200,
        std::f64::consts::PI,
        f64::MAX,
    ];
    let mut seed = 7u64;
    for _ in 0..512 {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let f = f64::from_bits(seed);
        if f.is_finite() {
            samples.push(f);
        }
    }
    let project = ProjectFile {
        version: 1,
        derived: vec![DerivedSpectrum {
            label: "Cu μ\n  keep spacing ".into(),
            energy: samples.clone(),
            mu: samples.clone(),
            ..Default::default()
        }],
        extensions: [(
            "metadata".into(),
            json!({"null": null, "zero": -0.0, "empty": {}, "text": " ../a + b "}),
        )]
        .into(),
        ..Default::default()
    };
    let full = serde_json::to_value(&project).unwrap();
    let compact = compact::encode(full.clone()).unwrap();
    let reopened = parse(std::str::from_utf8(&compact).unwrap()).unwrap();
    assert_eq!(serde_json::to_value(&reopened).unwrap(), full);
    for values in [&reopened.derived[0].energy, &reopened.derived[0].mu] {
        assert_eq!(
            values.iter().map(|f| f.to_bits()).collect::<Vec<_>>(),
            samples.iter().map(|f| f.to_bits()).collect::<Vec<_>>()
        );
    }
    assert!(compact.len() < serde_json::to_vec(&full).unwrap().len());
}

#[test]
fn compact_defaults_preserve_explicit_fit_range_and_publication_settings() {
    for explicit in [false, true] {
        let mut project = load(&fixture("rexafs-0.1.0-links.rxs")).unwrap();
        project.fit_ranges.follow_transform = explicit;
        let value = serde_json::to_value(&project).unwrap();
        let bytes = compact::encode(value.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(parse(std::str::from_utf8(&bytes).unwrap()).unwrap()).unwrap(),
            value
        );
        assert!(bytes.len() < serde_json::to_vec_pretty(&value).unwrap().len() * 3 / 4);
    }
}

#[test]
fn fixed_lambda_persists_while_missing_legacy_clamp_fields_keep_their_meaning() {
    use rexafs::prelude::AUTOBKClampScalePolicy;
    for json in [r#"{"version":1}"#, r#"{"version":1,"params":{}}"#] {
        assert_eq!(
            parse(json).unwrap().params.bkg_clamp_policy,
            AUTOBKClampScalePolicy::Fixed
        );
    }
    let modern = PipelineParams::default();
    assert_eq!(
        modern.bkg_clamp_policy,
        AUTOBKClampScalePolicy::FixedPenalty
    );
    assert_ne!(
        modern.fingerprint(),
        PipelineParams::legacy_defaults().fingerprint()
    );
    for lambda in [None, Some(0.0), Some(0.001), Some(0.1)] {
        let params = PipelineParams {
            bkg_clamp_lambda: lambda,
            ..modern.clone()
        };
        let original = ProjectFile {
            version: 1,
            params,
            ..Default::default()
        };
        let bytes = compact::encode(serde_json::to_value(&original).unwrap()).unwrap();
        let restored = parse(std::str::from_utf8(&bytes).unwrap()).unwrap();
        assert_eq!(restored.params.bkg_clamp_lambda, lambda);
        assert_eq!(
            restored.params.bkg_clamp_policy,
            AUTOBKClampScalePolicy::FixedPenalty
        );
        assert_eq!(restored.params.fingerprint(), original.params.fingerprint());
        assert_ne!(
            PipelineParams {
                bkg_clamp_lambda: Some(0.2),
                ..modern.clone()
            }
            .fingerprint(),
            original.params.fingerprint()
        );
    }
}

#[test]
fn independent_channels_roundtrip_linked_and_embedded_with_one_raw_source() {
    use crate::params::{DetectionMode, ImportConfig};
    for mode in [DataStorage::Paths, DataStorage::Embedded] {
        let temp = Temp::new();
        let source = temp.join("channels.dat");
        let raw = b"# energy i0 it ir\n100 100 50 10\n101 100 40 8\n102 100 30 6\n";
        std::fs::write(&source, raw).unwrap();
        let reference = PipelineParams {
            import: ImportConfig {
                mode: DetectionMode::Reference,
                ..Default::default()
            },
            rbkg: Some(1.3),
            ..Default::default()
        };
        let project = ProjectFile {
            version: 1,
            spectrum_file: Some(source.clone()),
            raw_files: vec![source.clone()],
            active_derived: Some(12),
            derived: vec![DerivedSpectrum {
                id: 12,
                label: "reference".into(),
                source: Some(source.clone()),
                params: Some(reference.clone()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let path = temp.join("channels.rxs");
        save_with_storage(&path, &project, mode).unwrap();
        if mode == DataStorage::Embedded {
            std::fs::remove_file(source).unwrap();
        }
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.active_derived, Some(12));
        assert_eq!(loaded.derived[0].id, 12);
        assert!(loaded.derived[0].params.as_ref() == Some(&reference));
        assert_eq!(
            std::fs::read(loaded.derived[0].source.as_ref().unwrap()).unwrap(),
            raw
        );
        assert_eq!(loaded.raw_files.len(), 1);
        assert_eq!(loaded.derived[0].source, loaded.spectrum_file);
    }
}

#[test]
fn invalid_channel_ids_are_rejected_and_legacy_groups_get_stable_ids() {
    let base = json!({"version":1,"derived":[{"label":"A","energy":[],"mu":[]},{"label":"B","energy":[],"mu":[]}]});
    let mut parsed = parse(&base.to_string()).unwrap();
    let mut repeat = parsed.clone();
    parsed.assign_group_ids();
    repeat.assign_group_ids();
    assert_eq!(parsed.derived[0].group_id, repeat.derived[0].group_id);
    assert_ne!(parsed.derived[0].group_id, parsed.derived[1].group_id);
    let mut duplicate = serde_json::to_value(&parsed).unwrap();
    duplicate["derived"][1]["group_id"] = duplicate["derived"][0]["group_id"].clone();
    assert!(parse(&duplicate.to_string()).is_err());
    assert_ne!(parsed.derived[0].id, parsed.derived[1].id);
    assert!(parsed.derived.iter().all(|d| d.id > 0));
    for id in [1, u64::MAX] {
        let mut bad = base.clone();
        bad["derived"][0]["id"] = id.into();
        bad["derived"][1]["id"] = id.into();
        assert!(parse(&bad.to_string()).is_err());
    }
}

#[test]
fn typed_outputs_difference_calibration_and_merge_roundtrip_linked_and_embedded() {
    use crate::params::{Operation, OperationInput, Quantity, process_file};
    use crate::publication::{SpectrumInput, figures};
    let temp = Temp::new();
    let mut project = specimen(&temp.join("original"));
    let source = project.spectrum_file.clone().unwrap();
    let params = PipelineParams {
        e0: Some(8979.0),
        bkg_ek0: Some(8979.0),
        ..Default::default()
    };
    let sp = process_file(&source, &params).unwrap();
    let difference =
        rexafs::xafs::tools::difference(&sp, &sp, rexafs::xafs::tools::DiffSpace::Norm).unwrap();
    let baseline = DerivedSpectrum {
        id: 1,
        label: "baseline".into(),
        energy: sp.energy.as_ref().unwrap().as_slice().to_vec(),
        mu: sp.mu.as_ref().unwrap().as_slice().to_vec(),
        params: Some(params.clone()),
        ..Default::default()
    };
    let input = OperationInput {
        group_id: None,
        label: "Cu".into(),
        path: source,
        derived_id: None,
        fingerprint: params.fingerprint(),
        size: Some(20737),
    };
    let diff = DerivedSpectrum {
        id: 2,
        label: "arbitrary renamed result".into(),
        energy: difference.energy.unwrap().as_slice().to_vec(),
        mu: difference.mu.unwrap().as_slice().to_vec(),
        quantity: Quantity::NormalizedDifference,
        params: Some(params.clone()),
        operation: Some(Operation {
            tool: "Difference spectrum".into(),
            parameters: json!({"space": "NormalizedMu"}),
            inputs: vec![
                input.clone(),
                OperationInput {
                    group_id: None,
                    label: "baseline".into(),
                    path: PathBuf::new(),
                    derived_id: Some(1),
                    fingerprint: baseline.fingerprint(&params),
                    size: None,
                },
            ],
            applied_energy_shift_ev: 0.0,
        }),
        ..Default::default()
    };
    let mut shifted = sp.clone();
    shifted.shift_energy(3.25);
    let calibrated = DerivedSpectrum {
        id: 3,
        label: "calibrated Cu".into(),
        energy: shifted.energy.unwrap().as_slice().to_vec(),
        mu: shifted.mu.unwrap().as_slice().to_vec(),
        params: Some(params.for_materialized(3.25)),
        operation: Some(Operation {
            tool: "Calibrate energy".into(),
            parameters: json!({"expected_energy_ev": 8982.25}),
            inputs: vec![input],
            applied_energy_shift_ev: 3.25,
        }),
        ..Default::default()
    };
    project.derived = vec![baseline, diff.clone(), calibrated.clone()];
    let mut merged = calibrated.clone();
    merged.id = 4;
    merged.label = "calibrated Cu · merge 2".into();
    merged.operation = Some(Operation {
        tool: "merge".into(),
        parameters: json!({"template": "calibrated Cu", "count": 2}),
        inputs: [2, 0]
            .map(|i| OperationInput {
                group_id: None,
                label: project.derived[i].label.clone(),
                path: PathBuf::new(),
                derived_id: Some(project.derived[i].id),
                fingerprint: project.derived[i].fingerprint(&params),
                size: None,
            })
            .to_vec(),
        applied_energy_shift_ev: 0.0,
    });
    project.derived.push(merged.clone());
    project.active_derived = Some(2);
    for mode in [DataStorage::Paths, DataStorage::Embedded] {
        let saved = temp.join(&format!("typed-{mode:?}.rxs"));
        save_with_storage(&saved, &project, mode).unwrap();
        let loaded = load(&saved).unwrap();
        assert_eq!(state(&project), state(&loaded));
        let mean = &loaded.derived[3];
        assert_eq!(mean.quantity, Quantity::RawMu);
        assert_eq!(mean.operation, merged.operation);
        assert_eq!(mean.params.as_ref().unwrap().e0, Some(8982.25));
        assert_eq!(
            mean.raw(mean.params.as_ref().unwrap()).unwrap(),
            (merged.energy.clone(), merged.mu.clone())
        );
        let result = &loaded.derived[1];
        assert_eq!(result.quantity, Quantity::NormalizedDifference);
        assert!(!result.quantity_unconfirmed);
        assert!(result.display_label().contains("Δμnorm"));
        assert!(
            result
                .process(&params)
                .unwrap_err()
                .contains("normalization/AUTOBK disabled")
        );
        let display = result.for_display(&params).unwrap();
        assert_eq!(display.mu.as_ref().unwrap().as_slice(), diff.mu);
        assert!(display.normalization.is_none() && display.background.is_none());
        let input = SpectrumInput {
            group: Some(result.clone()),
            params: params.clone(),
            data: Some(std::sync::Arc::new(display)),
            ..Default::default()
        };
        assert!(
            input.process().is_err(),
            "cached display data cannot enter fitting"
        );
        let plots = figures::quantity_figures(
            input.for_display().unwrap(),
            &result.display_label(),
            Some(result.quantity),
        );
        assert_eq!(plots.len(), 1);
        assert_eq!(plots[0].series[0].x, diff.energy);
        assert_eq!(plots[0].series[0].y, diff.mu);
        let csv = plots[0].csv(&Default::default()).unwrap();
        assert!(
            csv.lines()
                .next()
                .unwrap()
                .contains("Δμnorm (dimensionless)")
        );
        assert_eq!(csv.lines().count(), diff.energy.len() + 1);
        let result = &loaded.derived[2];
        let settings = result.params.as_ref().unwrap();
        assert_eq!(settings.e0, Some(8982.25));
        assert_eq!(settings.bkg_ek0, Some(8982.25));
        assert_eq!(
            result.operation.as_ref().unwrap().applied_energy_shift_ev,
            3.25
        );
        for _ in 0..2 {
            let processed = result.process(settings).unwrap();
            assert_eq!(processed.energy.unwrap().as_slice(), calibrated.energy);
            assert_eq!(processed.e0, settings.e0);
        }
        // A second save/reopen must preserve provenance and never add ΔE again.
        save(&saved, &loaded).unwrap();
        assert_eq!(state(&loaded), state(&load(&saved).unwrap()));
    }
}

#[test]
fn typed_outputs_legacy_quantity_hints_require_explicit_confirmation() {
    use crate::params::Quantity;
    for invalid in [json!(5), json!("invalid"), json!(null)] {
        assert!(parse(&json!({"version": 1, "derived": [invalid]}).to_string()).is_err());
    }
    for (label, expected) in [
        ("unknown", Quantity::RawMu),
        ("diff: A − B", Quantity::NormalizedDifference),
        ("A − B · Δμnorm", Quantity::NormalizedDifference),
    ] {
        let value = json!({"version": 1, "derived": [{"label": label, "energy": [1., 2.], "mu": [0., 0.]}]});
        let mut project = parse(&value.to_string()).unwrap();
        let group = &mut project.derived[0];
        assert_eq!(group.quantity, expected);
        assert!(group.quantity_unconfirmed);
        assert!(
            group
                .process(&PipelineParams::default())
                .unwrap_err()
                .contains("Quantity unconfirmed")
        );
        group.label = "renamed again".into();
        let encoded = compact::encode(serde_json::to_value(&project).unwrap()).unwrap();
        let mut reopened = parse(std::str::from_utf8(&encoded).unwrap()).unwrap();
        assert_eq!(reopened.derived[0].quantity, expected);
        assert!(reopened.derived[0].quantity_unconfirmed);
        reopened.derived[0].confirm_quantity(Quantity::NormalizedDifference);
        let encoded = compact::encode(serde_json::to_value(&reopened).unwrap()).unwrap();
        let confirmed = parse(std::str::from_utf8(&encoded).unwrap()).unwrap();
        assert!(!confirmed.derived[0].quantity_unconfirmed);
        assert_eq!(
            confirmed.derived[0].quantity,
            Quantity::NormalizedDifference
        );
    }
    let channel = parse(r#"{"version":1,"derived":[{"label":"diff: editable name","source":"scan.dat","energy":[],"mu":[]}]}"#).unwrap();
    assert_eq!(channel.derived[0].quantity, Quantity::RawMu);
    assert!(!channel.derived[0].quantity_unconfirmed);
}

#[test]
fn group_identity_roundtrip_relocation_channels_results_and_three_more_imports() {
    use crate::app::DERIVED_BASE;
    use crate::group_identity::{GroupId, GroupRegistry};
    use crate::params::{DetectionMode, Operation, OperationInput};
    use std::collections::{BTreeMap, BTreeSet};

    let temp = Temp::new();
    for mode in [DataStorage::Paths, DataStorage::Embedded] {
        let old = temp.join(&format!("old-{mode:?}"));
        let mut project = specimen(&old);
        let old = storage::resolved_location(&old);
        let source = storage::resolved_location(project.spectrum_file.as_ref().unwrap());
        project.derived = vec![
            DerivedSpectrum {
                id: 1,
                source: Some(source.clone()),
                params: Some(PipelineParams {
                    import: crate::params::ImportConfig {
                        mode: DetectionMode::Reference,
                        ..Default::default()
                    },
                    e0: Some(8000.),
                    ..Default::default()
                }),
                ..Default::default()
            },
            DerivedSpectrum {
                id: 2,
                group_id: Some(GroupId::new_result()),
                label: "tool result".into(),
                energy: vec![1., 2.],
                mu: vec![3., 4.],
                ..Default::default()
            },
        ];
        project.assign_group_ids();
        let mut files = project.raw_files.clone();
        files.sort();
        files.dedup();
        let build = |project: &mut ProjectFile, files: &[PathBuf]| {
            GroupRegistry::rebuild(
                files
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(ix, path)| (ix, path, DetectionMode::Auto)),
                &mut project.derived,
                &mut project.source_groups,
                &project.source_origins,
            )
        };
        let registry = build(&mut project, &files);
        let source_ix = files.iter().position(|p| *p == source).unwrap();
        let marked = BTreeSet::from([source_ix, DERIVED_BASE, DERIVED_BASE + 1]);
        let frozen = BTreeSet::from([source_ix, DERIVED_BASE]);
        let custom = PipelineParams {
            e0: Some(9001.),
            ..Default::default()
        };
        project.group_state.capture(
            &registry,
            &marked,
            &frozen,
            &BTreeMap::from([(source_ix, custom.clone())]),
            Some(DERIVED_BASE),
        );
        for ix in &marked {
            let id = registry.id(*ix).unwrap();
            project
                .group_state
                .labels
                .insert(id.clone(), format!("Renamed {ix}"));
            project.group_state.colors.insert(id, (*ix % 8) as u8);
        }
        let inputs: Vec<_> = marked
            .iter()
            .filter(|&&ix| ix != DERIVED_BASE + 1)
            .map(|&ix| OperationInput {
                group_id: registry.id(ix),
                label: format!("input {ix}"),
                path: if ix == source_ix {
                    source.clone()
                } else {
                    PathBuf::new()
                },
                derived_id: ix.checked_sub(DERIVED_BASE).map(|i| project.derived[i].id),
                fingerprint: 123,
                size: None,
            })
            .collect();
        project.derived[1].operation = Some(Operation {
            tool: "test".into(),
            parameters: json!({}),
            inputs: inputs.clone(),
            applied_energy_shift_ev: 0.,
        });
        let identities = serde_json::to_value(&project.group_state).unwrap();
        save_with_storage(&old.join("durable.rxs"), &project, mode).unwrap();
        let moved = temp.join(&format!("moved-{mode:?}"));
        std::fs::rename(&old, &moved).unwrap();
        let mut reopened = load(&moved.join("durable.rxs")).unwrap();
        reopened.assign_group_ids();
        assert_eq!(
            serde_json::to_value(&reopened.group_state).unwrap(),
            identities
        );
        assert_eq!(reopened.derived[0].params.as_ref().unwrap().e0, Some(8000.));
        assert_eq!(
            reopened.derived[1]
                .operation
                .as_ref()
                .unwrap()
                .inputs
                .iter()
                .map(|i| &i.group_id)
                .collect::<Vec<_>>(),
            inputs.iter().map(|i| &i.group_id).collect::<Vec<_>>()
        );
        // The real catalog can reopen in a different order, then append files.
        let mut files: Vec<_> = files
            .iter()
            .map(|path| {
                let tail = path.strip_prefix(&old).unwrap();
                let moved_source = storage::resolved_location(&moved.join(tail));
                reopened
                    .source_origins
                    .iter()
                    .find(|(_, original)| **original == moved_source)
                    .map(|(cache, _)| cache.clone())
                    .unwrap_or(moved_source)
            })
            .collect();
        files.reverse();
        let before = build(&mut reopened, &files);
        for i in 0..3 {
            let path = moved.join(format!("added-{i}.xmu"));
            std::fs::copy(fixture("data/cu_150k.xmu"), &path).unwrap();
            files.push(path);
        }
        let after = build(&mut reopened, &files);
        assert!(!before.indices_changed(&after));
        assert_eq!(
            before.indices(&reopened.group_state.marked),
            after.indices(&reopened.group_state.marked)
        );
        assert_eq!(
            after.indices(&reopened.group_state.marked).len(),
            3,
            "{mode:?}"
        );
        assert_eq!(after.indices(&reopened.group_state.frozen).len(), 2);
        assert!(
            reopened
                .group_state
                .resolved_overrides(&after)
                .values()
                .any(|p| *p == custom)
        );
        for input in &reopened.derived[1].operation.as_ref().unwrap().inputs {
            assert!(after.index(input.group_id.as_ref().unwrap()).is_some());
        }
        reopened.raw_files = files;
        let again = moved.join("again.rxs");
        save_with_storage(&again, &reopened, mode).unwrap();
        assert_eq!(state(&reopened), state(&load(&again).unwrap()));
    }
}

#[test]
fn standalone_processing_lock_roundtrip_linked_and_embedded() {
    use crate::group_identity::GroupRegistry;
    use std::collections::BTreeMap;
    let temp = Temp::new();
    for mode in [DataStorage::Paths, DataStorage::Embedded] {
        let dir = temp.join(&format!("standalone-lock-{mode:?}"));
        let mut project = specimen(&dir);
        let registry = GroupRegistry::default();
        let source = storage::resolved_location(project.spectrum_file.as_ref().unwrap());
        let id =
            registry.register_source(None, source, project.params.import.mode, &BTreeMap::new());
        project
            .group_state
            .capture_standalone_lock(&registry, &id, true);
        let path = dir.join("locked.rxs");
        save_with_storage(&path, &project, mode).unwrap();
        let mut reopened = load(&path).unwrap();
        assert!(reopened.group_state.frozen.contains(&id));
        reopened
            .group_state
            .capture_standalone_lock(&registry, &id, false);
        save_with_storage(&path, &reopened, mode).unwrap();
        assert!(!load(&path).unwrap().group_state.frozen.contains(&id));
    }
}

#[test]
fn removal_exclusions_roundtrip_linked_and_embedded_without_resurrection() {
    use crate::group_identity::{GroupId, GroupRegistry};
    use crate::params::{DetectionMode, Operation, OperationInput};
    use std::collections::BTreeSet;
    for mode in [DataStorage::Paths, DataStorage::Embedded] {
        let temp = Temp::new();
        let source = temp.join("input.dat");
        std::fs::write(&source, "1 2\n2 3\n").unwrap();
        let mut project = ProjectFile {
            version: 1,
            raw_files: vec![source.clone()],
            spectrum_file: Some(source.clone()),
            ..Default::default()
        };
        project.assign_group_ids();
        let id = project.source_groups[0].id.clone();
        project.group_state.excluded.insert(id.clone());
        let retired_channel = GroupId::source(&source, DetectionMode::Reference);
        project.group_state.excluded.insert(retired_channel.clone());
        project.derived.push(DerivedSpectrum {
            group_id: Some(GroupId::new_result()),
            id: 1,
            operation: Some(Operation {
                tool: "merge".into(),
                inputs: vec![OperationInput {
                    group_id: Some(id.clone()),
                    label: "Input".into(),
                    path: source.clone(),
                    derived_id: None,
                    fingerprint: 0,
                    size: None,
                }],
                parameters: Value::Null,
                applied_energy_shift_ev: 0.,
            }),
            ..Default::default()
        });
        let path = temp.join("excluded.rxs");
        save_with_storage(&path, &project, mode).unwrap();
        let mut loaded = load(&path).unwrap();
        loaded.assign_group_ids();
        assert_eq!(
            loaded.group_state.excluded,
            BTreeSet::from([id.clone(), retired_channel.clone()])
        );
        let registry = GroupRegistry::from_sources(loaded.source_groups.clone());
        registry.set_excluded(&loaded.group_state.excluded);
        let location = loaded.source_groups[0].path.clone();
        registry.register_source(
            Some(0),
            location,
            DetectionMode::Auto,
            &loaded.source_origins,
        );
        assert_eq!(registry.id(0), None);
        assert_eq!(registry.index(&id), None);
        let mut replacement = DerivedSpectrum {
            source: Some(source.clone()),
            params: Some(PipelineParams::default()),
            ..Default::default()
        };
        replacement.params.as_mut().unwrap().import.mode = DetectionMode::Reference;
        registry.assign_group(&mut replacement, &Default::default());
        assert_ne!(replacement.group_id, Some(retired_channel));
        assert_eq!(
            loaded.derived[0].operation.as_ref().unwrap().inputs[0].group_id,
            Some(id.clone())
        );
        let location = registry.sources()[0].path.clone();
        let fresh = registry.reimport_source(&location, Some(0)).unwrap();
        loaded.source_groups = registry.sources();
        save_with_storage(&path, &loaded, mode).unwrap();
        let mut reimported = load(&path).unwrap();
        reimported.assign_group_ids();
        let reopened = GroupRegistry::from_sources(reimported.source_groups.clone());
        reopened.set_excluded(&reimported.group_state.excluded);
        reopened.register_source(
            Some(0),
            reimported.source_groups[0].path.clone(),
            DetectionMode::Auto,
            &reimported.source_origins,
        );
        assert_eq!(reopened.id(0), Some(fresh));
        assert!(reopened.is_excluded(&id));
        assert_eq!(
            reimported.derived[0].operation.as_ref().unwrap().inputs[0].group_id,
            Some(id)
        );
        assert!(source.is_file());
    }
}
