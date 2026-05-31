---
proposed_id: ADR--GENESISDB-SEGREGATION-STRATEGY
type: adr
status: candidate
aliases:
  - ADR
phase: 2
tier: process
cluster: implementation_flow
role: Architecture decision record
enforcement_state: inactive
proposed_at: 2026-05-30T22:15:22.193Z
proposed_by: agent
rationale: Defining the roadmap for decouplings GenesisDB from the monorepo.
---

# ADR: GenesisDB Repository Segregation Strategy

## 1. Context\nAs GenesisDB matures into a high-performance hybrid semantic-graph engine, the question arises whether it should remain within the `cognitive_system` monorepo or be moved to a dedicated repository.\n\n## 2. Decision\nWe will maintain GenesisDB within the current monorepo until the completion of **Wave 4.5: Standalone Server**. \n\n## 3. Rationale\n- **Development Velocity:** Keeping the engine co-located with its primary consumer (GKS) allows for rapid iteration of FFI and protocol contracts without the overhead of cross-repo versioning.\n- **Contract Hardening:** The Standalone Server development will likely reveal necessary changes in the API to support networked interactions (TCP/gRPC). It is more efficient to apply these changes in a single turn.\n- **Single Source of Truth:** This ensures that AI agents (T1-T3) maintain high visibility across the entire vertical stack during the critical 7P Forward Path phase.\n\n## 4. Separation Criteria (The Trigger)\nThe migration to a dedicated repository (`google/genesis-db`) shall occur when:\n1. The Standalone Server binary is functional and passes all integration tests.\n2. A formal performance audit (P7) confirms stability under stress.\n3. The API/Contract is declared 'Stable' (v1.0.0-rc).\n\n## 5. Migration Strategy\n- **History Preservation:** Use `git filter-repo` or `git subtree` to extract `packages/gks/native/genesis-block` while preserving the commit history of the engine's development.\n- **Dependency Model:** GKS will transition from a workspace link to a standard dependency model (`npm install`, `cargo add`) referencing the new repo.\n\n## 6. Consequences\n- **Short-term:** Slightly larger monorepo context.\n- **Long-term:** A clean, decoupled, and reusable core engine that can serve multiple clients beyond GKS.
