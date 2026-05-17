---
id: CONCEPT--MEMORY-SESSIONS-WRITER
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Memory sessions writer — append turns to JSONL session log
tags:
  - msp
  - memory
  - sessions
  - writer
crosslinks: {"references":["CONCEPT--MEMORY-SESSIONS","CONCEPT--MEMORY-SUBSYSTEM"]}
created_at: 2026-05-03T14:16:38.088+07:00
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: "Strategic intent / PRD"
---

# CONCEPT — memory sessions writer

## Problem

`[[CONCEPT--MEMORY-SESSIONS]]` defines the on-disk JSONL shape, but nothing writes to it. Agents currently lose every turn at session end — no replay, no walkthrough handover, no audit trail.

## Hypothesis

A small, append-only writer keyed by `episodicId` will close the loop. Agents call `appendTurn(episodicId, row)` after each turn; the writer enforces the schema, prevents concurrent writers per file, and never rewrites historical rows. With this in place, every session becomes auditable in O(turns) bytes, and the episodic + vector layers have a real source to summarise from.

## Scope

In:
- Open / append / close lifecycle on `.brain/msp/projects/<ns>/sessions/<episodicId>.jsonl`.
- Schema validation per `[[CONCEPT--MEMORY-SESSIONS]]` (sessionId, episodicId, turnId, msgId, speakerId, content, optional learnId).
- Single-writer-per-file lock (refuses concurrent open of the same episodic).

Out:
- Reading sessions (a separate reader; trivial — `readline` over the JSONL).
- Episode summarisation (separate FEAT — `[[FEAT--MEMORY-EPISODIC-WRITER]]`).
- Vector embedding (orchestrator concern).

## Source

Implements `[[CONCEPT--MEMORY-SESSIONS]]`. Spec §7.1.

## Connections
- [[CONCEPT--MEMORY-SUBSYSTEM]]

