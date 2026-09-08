//! Immutable import interpretations and the exact groups created by each use.
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use rexafs::prelude::XdiHeader;
use serde::{Deserialize, Serialize};

use crate::{
    group_identity::GroupId,
    import_mapping::{AxisConversion, LayoutKey, MappingDraft},
    params::{DetectionMode, ImportConfig, ImportPreview},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeRef {
    pub id: String,
    pub version: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecipeScope {
    FolderTree(PathBuf),
    Instrument(BTreeMap<String, String>),
    AllSources,
}

impl RecipeScope {
    pub fn for_source(path: &Path, header: Option<&XdiHeader>) -> Self {
        let identity = instrument(header);
        if identity.is_empty() {
            Self::FolderTree(path.parent().unwrap_or(path).to_path_buf())
        } else {
            Self::Instrument(identity)
        }
    }

    pub fn contains(&self, path: &Path, header: Option<&XdiHeader>) -> bool {
        match self {
            Self::FolderTree(root) => path.starts_with(root),
            Self::Instrument(identity) => *identity == instrument(header),
            Self::AllSources => true,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::FolderTree(root) => format!("Folder tree: {}", root.display()),
            Self::Instrument(identity) => identity
                .iter()
                .map(|(key, value)| format!("{key}: {value}"))
                .collect::<Vec<_>>()
                .join(" · "),
            Self::AllSources => "All source locations".into(),
        }
    }
}

fn instrument(header: Option<&XdiHeader>) -> BTreeMap<String, String> {
    header
        .into_iter()
        .flat_map(|h| &h.metadata)
        .filter(|(key, _)| {
            ["beamline.name", "facility.name", "instrument.name"].contains(&key.as_str())
        })
        .map(|(key, value)| (key.clone(), value.trim().to_string()))
        .collect()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecipeVersion {
    pub reference: RecipeRef,
    pub name: String,
    pub layout: LayoutKey,
    pub scope: RecipeScope,
    pub primary: DetectionMode,
    pub channels: Vec<ImportConfig>,
    /// Explicit conversions for axis columns whose source units were absent.
    pub assumed_axes: BTreeMap<usize, AxisConversion>,
    #[serde(default = "enabled")]
    pub reuse: bool,
}

fn enabled() -> bool {
    true
}

pub fn new_id(kind: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "{kind}:{}:{}:{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

impl RecipeVersion {
    pub fn label(&self) -> String {
        format!("{} · v{}", self.name, self.reference.version)
    }

    pub fn from_review(
        name: &str,
        scope: RecipeScope,
        preview: &ImportPreview,
        primary: DetectionMode,
        channels: &[ImportConfig],
        confirm_missing_units: bool,
    ) -> Result<Self, String> {
        if name.trim().is_empty() {
            return Err("Give this import recipe a name.".into());
        }
        let layout = LayoutKey::from_preview(preview);
        let mut assumed_axes = BTreeMap::new();
        let mut resolved = Vec::new();
        for config in channels {
            let table = preview.for_mapping(config);
            let draft = MappingDraft::new(&table, config);
            draft.validate()?;
            let mut config = draft.config().clone();
            let axis = config.energy_col.ok_or("Choose an energy column.")?;
            let units = layout.units.get(axis).and_then(|u| u.as_deref());
            if units.is_none() && !confirm_missing_units {
                return Err(
                    "Confirm the axis-unit assumption for columns without declared units.".into(),
                );
            }
            if config.axis == AxisConversion::Auto {
                config.axis = match units {
                    None | Some("eV") => AxisConversion::EnergyEv,
                    Some("keV") => AxisConversion::EnergyKev,
                    Some(unit @ ("degrees" | "radians")) => {
                        let d_spacing = layout
                            .conversion_and_signal_metadata
                            .get("mono.d_spacing")
                            .and_then(|value| value.parse::<f64>().ok())
                            .filter(|value| value.is_finite() && *value > 0.)
                            .ok_or("Confirm positive d spacing for the angle axis.")?;
                        if unit == "degrees" {
                            AxisConversion::AngleDegrees { d_spacing }
                        } else {
                            AxisConversion::AngleRadians { d_spacing }
                        }
                    }
                    Some(_) => {
                        return Err(
                            "Choose an explicit conversion for the declared axis units.".into()
                        );
                    }
                };
            }
            if units.is_none() {
                if assumed_axes
                    .get(&axis)
                    .is_some_and(|previous| *previous != config.axis)
                {
                    return Err(
                        "Output channels disagree on the units of the same axis column.".into(),
                    );
                }
                assumed_axes.insert(axis, config.axis);
            }
            resolved.push(config);
        }
        if primary == DetectionMode::Auto || !resolved.iter().any(|c| c.mode == primary) {
            return Err("Include the explicit primary channel in the output set.".into());
        }
        if resolved
            .iter()
            .enumerate()
            .any(|(i, c)| resolved[..i].iter().any(|p| p.mode == c.mode))
        {
            return Err("Choose distinct output channels.".into());
        }
        Ok(Self {
            reference: RecipeRef {
                id: new_id("recipe"),
                version: 1,
            },
            name: name.trim().into(),
            layout,
            scope,
            primary,
            channels: resolved,
            assumed_axes,
            reuse: true,
        })
    }

    pub fn same_interpretation(&self, other: &Self) -> bool {
        self.layout == other.layout
            && self.primary == other.primary
            && self.channels == other.channels
            && self.assumed_axes == other.assumed_axes
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RecipeLibrary {
    pub versions: Vec<RecipeVersion>,
}

impl RecipeLibrary {
    pub fn get(&self, reference: &RecipeRef) -> Option<&RecipeVersion> {
        self.versions.iter().find(|v| v.reference == *reference)
    }

    /// A named edit creates a successor; existing references stay immutable.
    pub fn commit_review(&mut self, mut recipe: RecipeVersion) -> RecipeVersion {
        if let Some(previous) = self
            .versions
            .iter()
            .filter(|v| v.name == recipe.name && v.scope == recipe.scope)
            .max_by_key(|v| v.reference.version)
        {
            if previous.same_interpretation(&recipe) && previous.reuse {
                return previous.clone();
            }
            recipe.reference = RecipeRef {
                id: previous.reference.id.clone(),
                version: previous.reference.version + 1,
            };
        }
        self.versions.push(recipe.clone());
        recipe
    }

    pub fn remember(&mut self, recipe: RecipeVersion) -> Result<(), String> {
        if let Some(existing) = self
            .versions
            .iter_mut()
            .find(|v| v.reference == recipe.reference)
        {
            if !existing.same_interpretation(&recipe)
                || existing.name != recipe.name
                || existing.scope != recipe.scope
            {
                return Err(
                    "This recipe ID/version already has a different saved interpretation.".into(),
                );
            }
            existing.reuse = recipe.reuse;
        } else {
            self.versions.push(recipe);
        }
        Ok(())
    }

    pub fn stop_reusing(&mut self, id: &str) {
        for recipe in self.versions.iter_mut().filter(|v| v.reference.id == id) {
            recipe.reuse = false;
        }
    }

    pub fn eligible(
        &self,
        key: &LayoutKey,
        path: &Path,
        header: Option<&XdiHeader>,
    ) -> Vec<&RecipeVersion> {
        self.versions
            .iter()
            .filter(|v| {
                v.reuse
                    && v.layout == *key
                    && v.scope.contains(path, header)
                    && !self.versions.iter().any(|newer| {
                        newer.reference.id == v.reference.id
                            && newer.reference.version > v.reference.version
                    })
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApplicationMember {
    pub source_id: GroupId,
    pub path: PathBuf,
    pub group: GroupId,
    pub channel: DetectionMode,
    pub mapping_revision: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportApplication {
    pub id: String,
    pub batch: usize,
    pub recipe: RecipeRef,
    pub members: Vec<ApplicationMember>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectImports {
    pub recipes: RecipeLibrary,
    pub applications: Vec<ImportApplication>,
}

#[derive(Clone, Default)]
pub struct DispatchContext {
    pub batch: usize,
    pub project: RecipeLibrary,
    pub machine: RecipeLibrary,
}

pub enum Dispatch {
    Detection,
    Recipe(RecipeVersion),
    Review {
        reason: String,
        suggestion: Option<RecipeVersion>,
    },
}

impl DispatchContext {
    pub fn resolve(&self, key: &LayoutKey, path: &Path, header: Option<&XdiHeader>) -> Dispatch {
        let project = self.project.eligible(key, path, header);
        let candidates = if project.is_empty() {
            self.machine.eligible(key, path, header)
        } else {
            project
        };
        if let Some(first) = candidates.first() {
            if candidates
                .iter()
                .any(|other| !first.same_interpretation(other))
            {
                return Dispatch::Review { reason: "Conflicting recipes match this layout; choose its interpretation explicitly.".into(), suggestion: None };
            }
            if key.names.is_none() {
                return Dispatch::Review { reason: "Unnamed columns need one representative confirmation in each new batch; the previous mapping is a suggestion.".into(), suggestion: Some((*first).clone()) };
            }
            return Dispatch::Recipe((*first).clone());
        }
        // A familiar column structure with different units or conversion data
        // must not silently fall back to otherwise plausible detector aliases.
        let changed = self
            .project
            .versions
            .iter()
            .chain(&self.machine.versions)
            .any(|recipe| {
                recipe.reuse
                    && recipe.scope.contains(path, header)
                    && recipe.layout.parser == key.parser
                    && recipe.layout.column_count == key.column_count
                    && recipe.layout.names == key.names
            });
        if changed {
            Dispatch::Review { reason: "A previous recipe uses different units, dialect or conversion metadata; confirm this layout separately.".into(), suggestion: None }
        } else {
            Dispatch::Detection
        }
    }
}

impl ProjectImports {
    pub fn application_for(&self, group: &GroupId) -> Option<&ImportApplication> {
        self.applications
            .iter()
            .rev()
            .find(|a| a.members.iter().any(|m| m.group == *group))
    }
}

pub fn mapping_revision(mapping: &ImportConfig) -> u64 {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(
        serde_json::to_vec(mapping).expect("validated import mapping is serializable"),
    );
    u64::from_le_bytes(hash[..8].try_into().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_prefers_project_and_reviews_ambiguity_or_changed_interpretation() {
        let root = std::env::temp_dir().join(new_id("dispatch").replace(':', "-"));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("sample.xdi");
        std::fs::write(&path, "# XDI/1.0\n# Column.1: energy eV\n# Column.2: i0\n# Column.3: it\n# ---\n8900 10 5\n9000 12 5\n9100 14 5\n").unwrap();
        let preview = crate::params::preview_import(&path, &ImportConfig::default()).unwrap();
        let mapping = MappingDraft::new(&preview, &ImportConfig::default())
            .config()
            .clone();
        let recipe = RecipeVersion::from_review(
            "Local",
            RecipeScope::for_source(&path, preview.xdi.as_ref()),
            &preview,
            mapping.mode,
            &[mapping],
            false,
        )
        .unwrap();
        let key = recipe.layout.clone();
        let mut context = DispatchContext::default();
        assert!(matches!(
            context.resolve(&key, &path, None),
            Dispatch::Detection
        ));
        context.machine.remember(recipe.clone()).unwrap();
        assert!(matches!(context.resolve(&key, &path, None), Dispatch::Recipe(r) if r == recipe));

        let mut project = recipe.clone();
        project.reference.id = new_id("project");
        project.channels[0].it_col = Some(1);
        project.name = "Project".into();
        context.project.remember(project.clone()).unwrap();
        assert!(matches!(context.resolve(&key, &path, None), Dispatch::Recipe(r) if r == project));
        context.project.remember(recipe.clone()).unwrap();
        assert!(matches!(
            context.resolve(&key, &path, None),
            Dispatch::Review {
                suggestion: None,
                ..
            }
        ));
        context.project.stop_reusing(&project.reference.id);
        assert!(matches!(context.resolve(&key, &path, None), Dispatch::Recipe(r) if r == recipe));

        let mut changed = key.clone();
        changed.units[0] = Some("keV".into());
        assert!(matches!(
            context.resolve(&changed, &path, None),
            Dispatch::Review {
                suggestion: None,
                ..
            }
        ));
        changed = key.clone();
        changed
            .conversion_and_signal_metadata
            .insert("mono.d_spacing".into(), "3.1356".into());
        assert!(matches!(
            context.resolve(&changed, &path, None),
            Dispatch::Review { .. }
        ));
        assert!(matches!(
            context.resolve(&key, Path::new("/unrelated/sample.xdi"), None),
            Dispatch::Detection
        ));

        context.project.stop_reusing(&recipe.reference.id);
        context.machine.stop_reusing(&recipe.reference.id);
        assert!(matches!(
            context.resolve(&key, &path, None),
            Dispatch::Detection
        ));
        assert_eq!(
            context.project.get(&recipe.reference).unwrap().channels,
            recipe.channels
        );

        let mut unnamed = recipe.clone();
        unnamed.layout.names = None;
        context.project = RecipeLibrary {
            versions: vec![unnamed.clone()],
        };
        assert!(
            matches!(context.resolve(&unnamed.layout, &path, None), Dispatch::Review { suggestion: Some(r), .. } if r == unnamed)
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn review_requires_units_confirmation_and_versions_do_not_rewrite_previous_users() {
        let path = std::env::temp_dir().join(format!("rexafs-recipes-{}.dat", std::process::id()));
        std::fs::write(
            &path,
            "# coordinate first second\n8900 10 5\n9000 12 5\n9100 14 5\n",
        )
        .unwrap();
        let preview = crate::params::preview_import(&path, &ImportConfig::default()).unwrap();
        let mapping = MappingDraft::new(&preview, &ImportConfig::default())
            .config()
            .clone();
        let scope = RecipeScope::FolderTree(path.parent().unwrap().into());
        assert!(
            RecipeVersion::from_review(
                "Beamline",
                scope.clone(),
                &preview,
                mapping.mode,
                &[mapping.clone()],
                false
            )
            .is_err()
        );
        let first = RecipeVersion::from_review(
            "Beamline",
            scope.clone(),
            &preview,
            mapping.mode,
            &[mapping.clone()],
            true,
        )
        .unwrap();
        assert_eq!(first.assumed_axes.get(&0), Some(&AxisConversion::EnergyEv));
        assert_eq!(first.channels[0].axis, AxisConversion::EnergyEv);
        let mut library = RecipeLibrary::default();
        let first = library.commit_review(first);
        let mut changed = mapping;
        changed.axis = AxisConversion::EnergyKev;
        let next =
            RecipeVersion::from_review("Beamline", scope, &preview, changed.mode, &[changed], true)
                .unwrap();
        let next = library.commit_review(next);
        assert_eq!(first.reference.id, next.reference.id);
        assert_eq!(next.reference.version, 2);
        assert_eq!(
            library.get(&first.reference).unwrap().channels[0].axis,
            AxisConversion::EnergyEv
        );
        assert_eq!(library.eligible(&first.layout, &path, None), vec![&next]);
        assert!(
            library
                .eligible(&first.layout, Path::new("/outside/source.dat"), None)
                .is_empty()
        );
        library.stop_reusing(&next.reference.id);
        assert!(library.eligible(&first.layout, &path, None).is_empty());
        assert_eq!(
            library.get(&first.reference).unwrap().channels[0].axis,
            AxisConversion::EnergyEv
        );
        std::fs::remove_file(path).unwrap();
    }
}
