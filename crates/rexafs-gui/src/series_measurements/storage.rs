//! Compact per-run storage. Released projects without runs and prototype inline
//! rows remain readable. Shared settings never change when another row is edited.
use super::*;
use serde::{
    Deserializer, Serializer,
    de::{self, MapAccess, SeqAccess, Visitor},
    ser::SerializeStruct,
};
use std::collections::{BTreeMap, HashMap};

#[derive(Default)]
pub(super) struct SettingsPool(BTreeMap<Vec<u8>, Vec<Arc<PipelineParams>>>);

impl SettingsPool {
    pub fn intern(&mut self, settings: &PipelineParams) -> Arc<PipelineParams> {
        let key = serde_json::to_vec(settings).expect("serializable processing settings");
        let candidates = self.0.entry(key).or_default();
        if let Some(shared) = candidates.iter().find(|p| ***p == *settings) {
            return shared.clone();
        }
        let shared = Arc::new(settings.clone());
        candidates.push(shared.clone());
        shared
    }
}

#[derive(Serialize)]
struct StoredRow<'a> {
    frame: &'a SeriesFrame,
    source_digest: &'a Option<String>,
    input_revision: &'a Option<String>,
    settings: usize,
    status: FrameStatus,
    result: &'a Option<MeasurementResult>,
    reason: &'a Option<String>,
    preparation: &'a serde_json::Value,
}

pub fn serialize<S: Serializer>(rows: &[MetricRow], serializer: S) -> Result<S::Ok, S::Error> {
    let mut settings = Vec::new();
    let mut pointers = HashMap::new();
    let mut content: BTreeMap<Vec<u8>, Vec<usize>> = BTreeMap::new();
    let mut indices = Vec::with_capacity(rows.len());
    for row in rows {
        let pointer = Arc::as_ptr(&row.settings);
        let index = if let Some(&index) = pointers.get(&pointer) {
            index
        } else {
            let key = serde_json::to_vec(&row.settings).map_err(serde::ser::Error::custom)?;
            let candidates = content.entry(key).or_default();
            let index = if let Some(&index) = candidates
                .iter()
                .find(|&&index| settings[index] == &row.settings)
            {
                index
            } else {
                settings.push(&row.settings);
                let index = settings.len() - 1;
                candidates.push(index);
                index
            };
            pointers.insert(pointer, index);
            index
        };
        indices.push(index);
    }
    struct Rows<'a>(&'a [MetricRow], &'a [usize]);
    impl Serialize for Rows<'_> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            use serde::ser::SerializeSeq;
            let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
            for (row, &settings) in self.0.iter().zip(self.1) {
                seq.serialize_element(&StoredRow {
                    frame: &row.frame,
                    source_digest: &row.source_digest,
                    input_revision: &row.input_revision,
                    settings,
                    status: row.status,
                    result: &row.result,
                    reason: &row.reason,
                    preparation: &row.preparation,
                })?;
            }
            seq.end()
        }
    }
    let mut value = serializer.serialize_struct("MeasurementRows", 3)?;
    value.serialize_field("schema", &1u32)?;
    value.serialize_field("settings", &settings)?;
    value.serialize_field("values", &Rows(rows, &indices))?;
    value.end()
}

#[derive(Deserialize)]
struct OwnedRow {
    frame: SeriesFrame,
    source_digest: Option<String>,
    input_revision: Option<String>,
    settings: usize,
    status: FrameStatus,
    result: Option<MeasurementResult>,
    reason: Option<String>,
    preparation: serde_json::Value,
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<MetricRow>, D::Error> {
    struct RowsVisitor;
    impl<'de> Visitor<'de> for RowsVisitor {
        type Value = Vec<MetricRow>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("inline prototype rows or versioned compact measurement rows")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut rows = Vec::new();
            let mut pool = SettingsPool::default();
            while let Some(mut row) = seq.next_element::<MetricRow>()? {
                row.settings = pool.intern(&row.settings);
                rows.push(row);
            }
            Ok(rows)
        }
        fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
            #[derive(Deserialize)]
            struct Compact {
                schema: u32,
                settings: Vec<Arc<PipelineParams>>,
                values: Vec<OwnedRow>,
            }
            let stored = Compact::deserialize(de::value::MapAccessDeserializer::new(map))?;
            if stored.schema != 1 {
                return Err(de::Error::custom("Unsupported measurement-row schema"));
            }
            stored
                .values
                .into_iter()
                .map(|row| {
                    let settings = stored
                        .settings
                        .get(row.settings)
                        .ok_or_else(|| de::Error::custom("Invalid measurement settings reference"))?
                        .clone();
                    Ok(MetricRow {
                        frame: row.frame,
                        source_digest: row.source_digest,
                        input_revision: row.input_revision,
                        settings,
                        status: row.status,
                        result: row.result,
                        reason: row.reason,
                        preparation: row.preparation,
                    })
                })
                .collect()
        }
    }
    deserializer.deserialize_any(RowsVisitor)
}
