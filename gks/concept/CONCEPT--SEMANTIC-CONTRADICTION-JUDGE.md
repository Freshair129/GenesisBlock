---
id: CONCEPT--SEMANTIC-CONTRADICTION-JUDGE
phase: 1
type: concept
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: "Semantic Contradiction Judge — leveraging T3 reasoning to catch logic drift"
tags:
  - msp
  - validator
  - contradiction
  - judge
  - governance
crosslinks: {"references":["ADR--CONTRADICTION-DETECTION-STACK","CONCEPT--ATOM-CONTRADICTION-DETECTION"]}
created_at: 2026-05-18T10:45:00+07:00
cluster: implementation_flow
role: "Strategic intent / PRD"
---

# CONCEPT — Semantic Contradiction Judge

## Intent

To provide a high-assurance, automated reasoning layer that detects logical conflicts between newly proposed knowledge and the existing stable canon. This "Machine Judge" uses semantically aware agents (T3) to look beyond schema matching and identify deep inconsistencies in body claims.

## North Star

Every PR adding an atom to the knowledge base is automatically audited for logical tensions against its neighboring graph. Contradictions are cited with specific evidence, allowing humans to make informed supersession decisions before the conflict reaches stable canon.

## Guiding Principles

1. **Graph-Augmented Reasoning:** Don't feed the LLM every atom. Use the Genesis Graph to find the most relevant neighbors (by domain, links, or similarity) to minimize token cost and maximize precision.
2. **Evidence-Based Citation:** The judge must cite the specific claims in both the old and new atoms that are in tension.
3. **Advisory, Not Blocking:** LLM judgment is probabilistic. The output is a reviewer hint, not a CI gate, maintaining human sovereignty.

## Connections

- `[[ADR--CONTRADICTION-DETECTION-STACK]]` — defines this as Layer 4 of the detection system.
- `[[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]` — provides the traversal engine for context gathering.
