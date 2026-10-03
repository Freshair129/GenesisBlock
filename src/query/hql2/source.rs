//! Opaque identities and cursors for storage-backed HQL2 sources.
use super::{
    error::QueryErrorV2,
    result::ColumnV2,
    value::{QueryValueV2, VectorValueV2},
};
use crate::uee_v2::{RecordKindV2, RecordRefV2};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BoundSourceV2 {
    Node(Option<String>),
    Edge(Option<String>),
    Row(String),
    Annotation,
    History { kind: RecordKindV2, id: String },
    Changes { after_seq: u64 },
}

impl BoundSourceV2 {
    pub(crate) fn kind_name(&self) -> &'static str {
        match self {
            Self::Node(_) => "node",
            Self::Edge(_) => "edge",
            Self::Row(_) => "row",
            Self::Annotation => "annotation",
            Self::History { .. } => "history",
            Self::Changes { .. } => "change",
        }
    }

    pub(crate) fn record_kind(&self) -> Option<RecordKindV2> {
        match self {
            Self::Node(_) => Some(RecordKindV2::Node),
            Self::Edge(_) => Some(RecordKindV2::Edge),
            Self::Row(_) => Some(RecordKindV2::Row),
            Self::Annotation => Some(RecordKindV2::Annotation),
            Self::History { kind, .. } => Some(kind.clone()),
            Self::Changes { .. } => None,
        }
    }
}

/// A source cursor cannot be decoded from a request and is valid only for the
/// exact lease and source that created it.
#[derive(Clone, Debug)]
pub(crate) struct SourceKeyV2 {
    source: BoundSourceV2,
    owner_token: String,
    generation_id: u64,
    wal_frontier: u64,
    acl_revision: u64,
    fencing_epoch: u64,
    valid_at: String,
    principal: String,
    namespace: String,
    record: RecordRefV2,
}

impl SourceKeyV2 {
    // Each argument contributes an independent cache identity dimension.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        source: BoundSourceV2,
        owner_token: &str,
        generation_id: u64,
        wal_frontier: u64,
        acl_revision: u64,
        fencing_epoch: u64,
        valid_at: &str,
        principal: &str,
        namespace: &str,
        record: RecordRefV2,
    ) -> Self {
        Self {
            source,
            owner_token: owner_token.to_owned(),
            generation_id,
            wal_frontier,
            acl_revision,
            fencing_epoch,
            valid_at: valid_at.to_owned(),
            principal: principal.to_owned(),
            namespace: namespace.to_owned(),
            record,
        }
    }

    // Validation compares the same independent source, lease, ACL, and time dimensions.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn validate(
        &self,
        source: &BoundSourceV2,
        owner_token: &str,
        generation_id: u64,
        wal_frontier: u64,
        acl_revision: u64,
        fencing_epoch: u64,
        valid_at: &str,
        principal: &str,
        namespace: &str,
        database_id: &str,
    ) -> Option<&RecordRefV2> {
        (&self.source == source
            && self.owner_token == owner_token
            && self.generation_id == generation_id
            && self.wal_frontier == wal_frontier
            && self.acl_revision == acl_revision
            && self.fencing_epoch == fencing_epoch
            && self.valid_at == valid_at
            && self.principal == principal
            && self.namespace == namespace
            && self.record.database_id == database_id
            && self.record.namespace == namespace
            && source.record_kind() == Some(self.record.kind.clone()))
        .then_some(&self.record)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SourceBatchV2 {
    pub(crate) records: Vec<RecordRefV2>,
    pub(crate) next: Option<SourceKeyV2>,
    /// Count includes records hidden by target/evidence ACL filtering.
    pub(crate) examined: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct VectorBatchV2 {
    pub(crate) entries: Vec<Option<VectorValueV2>>,
}

#[derive(Clone, Debug)]
pub(crate) struct GraphEdgeV2 {
    pub(crate) edge: RecordRefV2,
    pub(crate) source: RecordRefV2,
    pub(crate) target: RecordRefV2,
    pub(crate) relation: String,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct GraphSnapshotV2 {
    pub(crate) nodes: BTreeMap<String, RecordRefV2>,
    pub(crate) node_labels: BTreeMap<String, BTreeSet<String>>,
    pub(crate) edges: Vec<GraphEdgeV2>,
}

/// Binder-issued property slot. Runtime names are attached only after their
/// typed expression is evaluated for a particular input row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FieldIdV2 {
    index: u32,
    name: Option<String>,
    has_property: bool,
}

impl FieldIdV2 {
    pub(super) fn new(index: u32, has_property: bool) -> Self {
        Self {
            index,
            name: None,
            has_property,
        }
    }

    pub(crate) fn with_name(&self, name: &str) -> Self {
        Self {
            index: self.index,
            name: Some(name.to_owned()),
            has_property: self.has_property,
        }
    }

    pub(crate) fn index(&self) -> u32 {
        self.index
    }

    pub(crate) fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub(crate) fn has_property(&self) -> bool {
        self.has_property
    }

    pub(crate) fn column(&self) -> ColumnV2 {
        ColumnV2 {
            name: format!("__hql2_field_{}", self.index),
            data_type: if self.has_property {
                "Bool".into()
            } else {
                "Nullable<Json>".into()
            },
            nullable: !self.has_property,
        }
    }

    fn accepts(&self, value: &QueryValueV2) -> bool {
        if self.has_property {
            matches!(value, QueryValueV2::Bool(_))
        } else {
            matches!(value, QueryValueV2::Null | QueryValueV2::Json(_))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StableRowKeyV2 {
    Record(RecordRefV2),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ExecBatchV2 {
    pub(crate) columns: Vec<ColumnV2>,
    pub(crate) rows: Vec<Vec<QueryValueV2>>,
    pub(crate) row_keys: Vec<StableRowKeyV2>,
}

impl ExecBatchV2 {
    pub(crate) fn validate(
        &self,
        records: &[RecordRefV2],
        fields: &[FieldIdV2],
    ) -> Result<(), QueryErrorV2> {
        let mut ids = BTreeSet::new();
        if self.columns.len() != fields.len()
            || self.rows.len() != records.len()
            || self.row_keys.len() != records.len()
            || fields.iter().any(|field| {
                field.name.is_none()
                    || !ids.insert(field.index)
                    || self.columns.get(ids.len() - 1) != Some(&field.column())
            })
        {
            return Err(QueryErrorV2::new(
                "DATA_CORRUPTION",
                "execute",
                "hydration_alignment",
            ));
        }
        for ((row, key), record) in self.rows.iter().zip(&self.row_keys).zip(records) {
            if row.len() != fields.len()
                || key != &StableRowKeyV2::Record(record.clone())
                || row
                    .iter()
                    .zip(fields)
                    .any(|(value, field)| !field.accepts(value))
            {
                return Err(QueryErrorV2::new(
                    "DATA_CORRUPTION",
                    "execute",
                    "hydration_alignment",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RecordKeyV2(String, String, String, String, String);

impl From<&RecordRefV2> for RecordKeyV2 {
    fn from(record: &RecordRefV2) -> Self {
        let kind = match &record.kind {
            RecordKindV2::Node => "node",
            RecordKindV2::Edge => "edge",
            RecordKindV2::Row => "row",
            RecordKindV2::Vector => "vector",
            RecordKindV2::Annotation => "annotation",
            RecordKindV2::Artifact => "artifact",
        };
        Self(
            record.database_id.clone(),
            record.namespace.clone(),
            kind.into(),
            record.id.clone(),
            record.revision.clone(),
        )
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct HydratedValuesV2 {
    values: BTreeMap<(RecordKeyV2, u32, String), QueryValueV2>,
}

impl HydratedValuesV2 {
    pub(crate) fn get(
        &self,
        record: &RecordRefV2,
        field: &FieldIdV2,
        name: &str,
    ) -> Option<&QueryValueV2> {
        self.values
            .get(&(RecordKeyV2::from(record), field.index, name.to_owned()))
    }

    pub(crate) fn add_batch(
        &mut self,
        records: &[RecordRefV2],
        fields: &[FieldIdV2],
        batch: ExecBatchV2,
    ) -> Result<(), QueryErrorV2> {
        batch.validate(records, fields)?;
        for ((row, key), record) in batch.rows.into_iter().zip(batch.row_keys).zip(records) {
            let StableRowKeyV2::Record(key_record) = key;
            if &key_record != record {
                return Err(QueryErrorV2::new(
                    "DATA_CORRUPTION",
                    "execute",
                    "hydration_alignment",
                ));
            }
            for (value, field) in row.into_iter().zip(fields) {
                let name = field.name.as_ref().ok_or_else(|| {
                    QueryErrorV2::new("DATA_CORRUPTION", "execute", "hydration_alignment")
                })?;
                if self
                    .values
                    .insert(
                        (RecordKeyV2::from(record), field.index, name.clone()),
                        value,
                    )
                    .is_some()
                {
                    return Err(QueryErrorV2::new(
                        "DATA_CORRUPTION",
                        "execute",
                        "hydration_alignment",
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod hydration_batch_tests {
    use super::{ExecBatchV2, FieldIdV2, QueryValueV2, StableRowKeyV2};
    use crate::uee_v2::{RecordKindV2, RecordRefV2};
    use serde_json::json;

    fn record() -> RecordRefV2 {
        RecordRefV2 {
            database_id: "a".repeat(64),
            namespace: "default".into(),
            kind: RecordKindV2::Node,
            id: "node:1".into(),
            revision: "1b6f53f4-2ca1-41f8-9a4c-22eeab37b5d8".into(),
        }
    }

    #[test]
    fn exec_batch_checks_typed_columns_rows_and_stable_keys() {
        let record = record();
        let field = FieldIdV2::new(0, false).with_name("title");
        let fields = [field.clone()];
        let records = [record.clone()];
        let valid = ExecBatchV2 {
            columns: vec![field.column()],
            rows: vec![vec![QueryValueV2::Json(json!("title"))]],
            row_keys: vec![StableRowKeyV2::Record(record.clone())],
        };
        assert!(valid.validate(&records, &fields).is_ok());

        let wrong_type = ExecBatchV2 {
            columns: vec![field.column()],
            rows: vec![vec![QueryValueV2::Bool(true)]],
            row_keys: vec![StableRowKeyV2::Record(record.clone())],
        };
        assert_eq!(
            wrong_type.validate(&records, &fields).unwrap_err().code,
            "DATA_CORRUPTION"
        );

        let wrong_key = ExecBatchV2 {
            columns: vec![field.column()],
            rows: vec![vec![QueryValueV2::Json(json!("title"))]],
            row_keys: vec![],
        };
        assert_eq!(
            wrong_key.validate(&records, &fields).unwrap_err().code,
            "DATA_CORRUPTION"
        );
    }
}
