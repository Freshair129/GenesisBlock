---
title: "Client Namespace and Schema Contract"
doc_id: "CONTRACT-CLIENT-NAMESPACE-AND-SCHEMA"
status: draft
version: "0.1.0+draft"
updated: "2026-08-03"
owner: "GenesisBlockDB Engineering"
source_of_truth: true
related_issue: 84
---

# Client Namespace and Schema Contract

## 1. Purpose

Define how independent client applications use GenesisBlockDB without importing another client's ontology into the database core.

GoVibe, NotiKeeper, and future clients may use different schemas, labels, relation types, authority rules, and lifecycle models while sharing one unmodified GenesisBlockDB core.

## 2. Core rule

GenesisBlockDB stores and queries generic records. Client applications own the meaning of their schemas.

```text
Client-owned domain model
  -> client namespace and schema reference
  -> generic GenesisBlockDB record
```

## 3. Required record metadata

A client-managed record SHOULD provide:

```yaml
client_contract:
  client_namespace: string
  schema_ref: string
  schema_version: string
  record_kind: string
  client_record_id: string
  client_mutation_id: string | null
```

### 3.1 `client_namespace`

A stable namespace identifying the owning client or domain, for example:

```text
govibe
notikeeper
example.product
```

A namespace does not grant database authorization by itself. Deployment and API security remain separate contracts.

### 3.2 `schema_ref`

A client-controlled identifier for the schema or semantic contract, for example:

```text
govibe://semantic/atom/v2
notikeeper://notification-rule/v1
```

GenesisBlockDB SHALL preserve this reference. Validation MAY be performed by the client, adapter, or a registered validation hook.

### 3.3 `schema_version`

A version used for compatibility and migration decisions. The database SHALL not reinterpret client meaning merely because a newer version exists.

### 3.4 `client_record_id`

The stable identifier supplied by the client. Internal numeric or hashed keys may exist for performance but SHALL NOT silently replace the external client ID.

## 4. Generic node example

```yaml
node:
  id: "atom:req-123"
  client_namespace: "govibe"
  schema_ref: "govibe://semantic/atom/v2"
  schema_version: "2.0.0"
  labels: ["REQ"]
  properties:
    summary: "System shall preserve semantic identity."
  provenance:
    source_ref: "doc:prd-17"
  valid_from: "2026-08-03T00:00:00Z"
```

## 5. Generic edge example

```yaml
edge:
  id: "rel:implements-44"
  client_namespace: "govibe"
  schema_ref: "govibe://semantic/relation/v2"
  relation_type: "IMPLEMENTS"
  source_id: "atom:work-44"
  target_id: "atom:req-123"
  properties: {}
  provenance:
    source_ref: "doc:srs-9"
```

The relation type remains client-defined. GenesisBlockDB may index and filter it without owning its business meaning.

## 6. Namespace isolation requirements

- Queries SHALL be able to scope by namespace where the interface exposes multi-client data.
- Cross-namespace relations SHALL require an explicit client or deployment policy.
- Import/export and backup metadata SHALL preserve namespace and schema references.
- Internal indexes SHALL not merge records solely because two clients use the same label or relation string.
- Vector collections SHALL declare namespace behavior or use separate named collections.

## 7. Schema validation

Supported modes:

1. `client_validated` — the client validates before mutation.
2. `adapter_validated` — an integration adapter validates.
3. `hook_validated` — GenesisBlockDB invokes an optional registered validator.
4. `unvalidated` — the database preserves the record but reports that schema validation was not performed.

The database core SHALL NOT hard-code GoVibe or NotiKeeper schema logic as mandatory behavior.

## 8. Schema evolution

A client schema change SHALL declare:

- old and new schema references/versions;
- compatibility classification;
- migration requirement;
- affected record kinds;
- rollback behavior;
- validation evidence.

GenesisBlockDB SHALL preserve prior schema metadata through supported temporal or version history.

## 9. Error conditions

- `NAMESPACE_REQUIRED`
- `NAMESPACE_FORBIDDEN`
- `SCHEMA_REF_REQUIRED`
- `SCHEMA_INCOMPATIBLE`
- `SCHEMA_VALIDATION_FAILED`
- `CROSS_NAMESPACE_RELATION_FORBIDDEN`
- `CLIENT_ID_CONFLICT`
- `MIGRATION_REQUIRED`

## 10. Conformance cases

- Store GoVibe and NotiKeeper records with independent schema references.
- Reuse the same label string in two namespaces without semantic merging.
- Preserve client IDs through snapshot and restore.
- Reject a forbidden cross-namespace relation.
- Accept a new third-party schema without recompiling the core.
- Upgrade one client's schema without changing another client's records.

## 11. Non-goals

- Defining GoVibe or NotiKeeper schema contents.
- Replacing authentication or authorization contracts.
- Making namespace metadata the sole security mechanism.
- Guaranteeing semantic compatibility between independent client schemas.

## Changelog

| Version | Date | Owner | Summary |
|---|---|---|---|
| 0.1.0+draft | 2026-08-03 | GenesisBlockDB Engineering | Initial client namespace and schema contract. |