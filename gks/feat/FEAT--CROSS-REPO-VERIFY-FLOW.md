---
id: FEAT--CROSS-REPO-VERIFY-FLOW
phase: 2
type: feat
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: FEAT — Cross-repo verify-flow — honouring ADRs across repo boundaries
tags: [msp, governance, verification, cross-repo, m9c]
aliases: [FEAT, implementation_flow, Feature specification]
cluster: implementation_flow
role: Feature specification
crosslinks:
  references:
    - CONCEPT--MSP-ROADMAP
    - FRAMEWORK--PHASE-GOVERNANCE
created_at: 2026-05-21T11:00:00+07:00
---

# FEAT — Cross-repo verify-flow

## 1. Summary

This feature extends the `gks verify-flow` command to support cross-repository verification. It allows a project (Repo B) to reference atoms in an external repository (Repo A) and verify that those remote dependencies are in a stable/approved state, ensuring governance consistency across distributed projects.

## 2. Motivation

In a microservices or multi-repository architecture, architectural decisions (ADRs) made in a central "platform" repo often govern the behavior and implementation of "consumer" repos. Currently, `verify-flow` only works within a single local vault. To maintain a unified governance chain, Repo B must be able to assert that the ADRs it depends on in Repo A are officially approved, preventing implementation against draft or contested decisions.

## 3. Requirements

### 3.1 Remote Index Loading
-   Support a `--remote=<repo-path-or-url>` flag in `gks verify-flow`.
-   Ability to load and parse the `atomic_index.jsonl` from the remote source.
-   Support for local filesystem paths (sibling directories) and eventually git URLs or authenticated HTTP endpoints.

### 3.2 Unified Atom Resolution
-   The verification engine must merge the local and remote atom indices into a single resolution map.
-   Prevent ID collisions by prioritizing local atoms or using a namespace prefix for remote atoms (e.g., `repoA:ADR--XYZ`).

### 3.3 Status Assertion
-   The existing `isApprovedStatus` check must apply equally to remote atoms.
-   If a local atom IMPLEMENTS a remote ADR that is still in `draft` status, the verification must fail.

### 3.4 Caching (Optional but Recommended)
-   Implement a local cache for remote indices to avoid expensive network calls during every CI run.
-   Provide a `--refresh` flag to force an update of the cached index.

## 4. Acceptance Criteria

-   [ ] `gks verify-flow ID --remote=../repoA` successfully walks a chain that spans both repos.
-   [ ] Verification fails if a remote dependency is not in a stable/approved state.
-   [ ] Clear error messages identifying whether the failed atom is local or remote.
-   [ ] Unit tests covering the multi-index merge and resolution logic.

## 5. Connections
-   `[[CONCEPT--MSP-ROADMAP]]` §3 M9c.
-   `[[FRAMEWORK--PHASE-GOVERNANCE]]` — the policy being enforced across boundaries.
