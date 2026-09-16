//! Series membership and coordinate changes are revisions, not rewrites of runs.
use super::*;
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

/// Meaning of explicitly supplied coordinates. Acquisition timestamps must
/// include a timezone; their declared start/midpoint/end meaning is retained.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CoordinateDefinition {
    pub label: String,
    pub unit: String,
    pub source: String,
    pub timestamp_meaning: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendAxis {
    #[default]
    Sequence,
    Coordinate,
    ElapsedAcquisition,
}

/// Natural decimal-token order without converting arbitrarily long integers.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let a = a.to_lowercase();
    let b = b.to_lowercase();
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i].is_ascii_digit() && b[j].is_ascii_digit() {
            let (sa, sb) = (i, j);
            while i < a.len() && a[i].is_ascii_digit() {
                i += 1;
            }
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            let mut na = &a[sa..i];
            let mut nb = &b[sb..j];
            while na.len() > 1 && na[0] == b'0' {
                na = &na[1..];
            }
            while nb.len() > 1 && nb[0] == b'0' {
                nb = &nb[1..];
            }
            let cmp = na.len().cmp(&nb.len()).then_with(|| na.cmp(nb));
            if cmp != Ordering::Equal {
                return cmp;
            }
        } else {
            let cmp = a[i].cmp(&b[j]);
            if cmp != Ordering::Equal {
                return cmp;
            }
            i += 1;
            j += 1;
        }
    }
    (a.len() - i).cmp(&(b.len() - j))
}

fn timestamp(text: Option<&str>) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    chrono::DateTime::parse_from_rfc3339(text?).ok()
}

impl SeriesDefinition {
    /// Commit current membership/metadata. Existing run rows are independent
    /// snapshots and retain their original sequence, coordinates and identities.
    pub fn revise(&mut self) {
        self.revision += 1;
        for (i, frame) in self.frames.iter_mut().enumerate() {
            frame.sequence = i + 1;
        }
    }

    pub fn sort_naturally(&mut self) {
        self.frames.sort_by(|a, b| natural_cmp(&a.label, &b.label));
        self.ordering = "Natural filename/label order".into();
        self.revise();
    }

    pub fn sort_acquisition(&mut self) -> Result<(), String> {
        if !self
            .frames
            .iter()
            .any(|f| timestamp(f.acquired_at.as_deref()).is_some())
        {
            return Err("Add acquisition timestamps with UTC offsets first.".into());
        }
        self.frames.sort_by(|a, b| {
            match (
                timestamp(a.acquired_at.as_deref()),
                timestamp(b.acquired_at.as_deref()),
            ) {
                (Some(a), Some(b)) => a.cmp(&b),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                _ => Ordering::Equal,
            }
        });
        self.ordering = "Acquisition time; missing timestamps last".into();
        self.revise();
        Ok(())
    }

    /// ID-keyed editable sidecar. A blank value is missing, not zero. Row order
    /// does not change membership and labels are informational only.
    pub fn coordinates_csv(&self) -> Result<Vec<u8>, String> {
        let mut writer = csv::Writer::from_writer(Vec::new());
        writer
            .write_record(["frame_id", "group_id", "label", "coordinate", "acquired_at"])
            .map_err(|e| e.to_string())?;
        for frame in &self.frames {
            writer
                .write_record([
                    serde_json::to_value(&frame.id)
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string(),
                    serde_json::to_value(&frame.group)
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string(),
                    frame.label.clone(),
                    frame.coordinate.map(|v| v.to_string()).unwrap_or_default(),
                    frame.acquired_at.clone().unwrap_or_default(),
                ])
                .map_err(|e| e.to_string())?;
        }
        writer.into_inner().map_err(|e| e.to_string())
    }

    /// Validate the whole sidecar before changing anything. Unknown or repeated
    /// frame IDs, mismatched groups, nonfinite values and timezone-free dates fail.
    pub fn import_coordinates(&mut self, bytes: &[u8]) -> Result<usize, String> {
        #[derive(Deserialize)]
        struct Row {
            frame_id: String,
            group_id: String,
            coordinate: Option<f64>,
            acquired_at: Option<String>,
        }
        let mut reader = csv::Reader::from_reader(bytes);
        let mut updated = self.frames.clone();
        let mut seen = BTreeSet::new();
        let indices: BTreeMap<_, _> = updated
            .iter()
            .enumerate()
            .map(|(i, f)| (f.id.clone(), i))
            .collect();
        for row in reader.deserialize::<Row>() {
            let row = row.map_err(|e| e.to_string())?;
            let id: GroupId =
                serde_json::from_value(serde_json::Value::String(row.frame_id)).unwrap();
            if !seen.insert(id.clone()) {
                return Err("Duplicate frame ID in coordinates.".into());
            }
            let index = *indices
                .get(&id)
                .ok_or("Coordinate frame is not in this series")?;
            let frame = &mut updated[index];
            let group: GroupId =
                serde_json::from_value(serde_json::Value::String(row.group_id)).unwrap();
            if frame.group != group {
                return Err("Coordinate group does not match the frame".into());
            }
            if row.coordinate.is_some_and(|v| !v.is_finite()) {
                return Err("Coordinates must be finite or blank".into());
            }
            if row.coordinate.is_some()
                && [
                    self.coordinate.label.as_str(),
                    self.coordinate.unit.as_str(),
                    self.coordinate.source.as_str(),
                ]
                .iter()
                .any(|s| s.trim().is_empty())
            {
                return Err(
                    "Declare the coordinate name, unit and source before importing values".into(),
                );
            }
            if let Some(text) = &row.acquired_at {
                chrono::DateTime::parse_from_rfc3339(text).map_err(|_| "Use RFC 3339 timestamps with UTC offsets, for example 2026-09-16T12:00:00+09:00")?;
            }
            frame.coordinate = row.coordinate;
            frame.acquired_at = row.acquired_at;
        }
        if seen.is_empty() {
            return Err("The coordinate file has no data rows".into());
        }
        self.frames = updated;
        self.revise();
        Ok(seen.len())
    }
}

impl SeriesRun {
    pub fn plot_coordinates(&self, axis: TrendAxis) -> Vec<Option<f64>> {
        let origin = self
            .rows
            .iter()
            .filter_map(|r| timestamp(r.frame.acquired_at.as_deref()))
            .min();
        self.rows
            .iter()
            .map(|row| match axis {
                TrendAxis::Sequence => Some(row.frame.sequence as f64),
                TrendAxis::Coordinate => row.frame.coordinate.filter(|v| v.is_finite()),
                TrendAxis::ElapsedAcquisition => {
                    let t = timestamp(row.frame.acquired_at.as_deref())?;
                    Some(t.signed_duration_since(origin?).num_milliseconds() as f64 / 1000.)
                }
            })
            .collect()
    }

    pub fn coordinate_label(&self, axis: TrendAxis) -> String {
        match axis {
            TrendAxis::Sequence => "Frame sequence".into(),
            TrendAxis::Coordinate => {
                format!("{} ({})", self.coordinate.label, self.coordinate.unit)
            }
            TrendAxis::ElapsedAcquisition => format!(
                "Elapsed acquisition time (s; {})",
                if self.coordinate.timestamp_meaning.is_empty() {
                    "unspecified timestamp meaning"
                } else {
                    &self.coordinate.timestamp_meaning
                }
            ),
        }
    }
}
