---
atom_id: HANDOFF--M7b-CONSOLIDATOR-CONFIG
created_at: 2026-05-16T12:00:00+07:00
type: handoff
tags: [msp, consolidator, config, types, handoff, m7b]
---

# HANDOFF: M7b Consolidator Configuration and Type Alignment

## 1. Recipient: T3 Agent - Opus Claude

## 2. Sender: Gemini CLI

## 3. Context

During the recent M7b Consolidator milestone, a user requested to "fix the config" of the newly implemented Consolidator module. This request was ambiguous, and my attempts to interpret and address it (initially by refactoring hardcoded constants and then by attempting to enable CLI configurability) led to a cascade of unmanageable type errors across the entire `packages/msp` workspace.

The underlying issues appear to be deep-seated type inconsistencies and potential misconfigurations in TypeScript paths, exacerbated by my iterative and sometimes erroneous attempts to resolve them. I have reverted my local working state to `main` and deleted the problematic development branch. I am unable to proceed with this task.

## 4. Problem Statement

The `packages/msp` codebase currently suffers from several critical type and configuration issues that prevent typecheck from passing cleanly when attempting to centralize configuration or enable dynamic provider selection:

-   **Inconsistent `Turn` Type Definition:** There is a fundamental mismatch in how `Turn` objects are expected and consumed across different modules. The canonical `SessionTurn` from `../../memory/sessions/types.js` uses `speakerId` and `content`, while many `consolidator` and `compressor` modules (and their tests) were expecting `speaker` and `text`. My attempts to standardize either way introduced widespread errors.
-   **Missing `SlmClient` Interface Definition:** The `SlmClient` interface in `packages/msp/src/codegen/slm/types.ts` is incomplete. It's missing the `generate(prompt: string): Promise<{ content: string }>` method, leading to `Property 'generate' does not exist on type 'SlmClient'` errors. This prevents proper LLM interaction.
-   **Hardcoded Providers in Consolidator CLI:** The `packages/msp/src/orchestrator/consolidator/cli.ts` (which I briefly created and then removed during debugging, but its intent remains) hardcodes the LLM provider (`ollama`) and Embedder provider (`nomic`). These need to be configurable.
-   **Scattered Configuration Constants:** Essential configuration values (e.g., scoring `WEIGHTS`, `DEFAULT_THRESHOLDS`, `DEFAULT_LLM_CALL_TIMEOUT_MS`, `DEFAULT_MAX_LLM_CALLS_PER_SESSION`) are currently spread across multiple files (`score.ts`, `llm.ts`, `index.ts`), making them difficult to manage and tune.

## 5. Instructions for Claude

**Goal:** Centralize Consolidator configuration and fully resolve type inconsistencies across the `packages/msp` workspace to enable robust and configurable module interactions.

**Specific Tasks:**

1.  **Standardize `Turn` Type Globally:**
    *   **Decision:** You need to decide on a single canonical interface for `Turn` objects across *all* `packages/msp` modules. Options are:
        *   Option A: Use `speaker` and `text` (as `consolidator` initially preferred). This would require modifying `SessionTurn` and all code consuming it.
        *   Option B: Use `speakerId` and `content` (as `SessionTurn` is defined). This would require modifying all `consolidator` and `compressor` modules (and their tests) to use these properties.
    *   **Action:** Implement your chosen canonical `Turn` interface and refactor *all* affected `msp` modules and test files to use it consistently. Ensure `packages/msp/src/orchestrator/consolidator/types.ts` defines/re-exports this canonical `Turn`.

2.  **Define Complete `SlmClient` Interface:**
    *   **Action:** Ensure `packages/msp/src/codegen/slm/types.ts` accurately defines the `SlmClient` interface, including the `generate(prompt: string): Promise<{ content: string }>` method and any other properties (`provider`, `model`) used in `createSlmClient` implementations.

3.  **Centralize Consolidator Configuration:**
    *   **Action:** Create or update `packages/msp/src/orchestrator/consolidator/config.ts`.
    *   **Action:** Move all hardcoded constants related to Consolidator configuration (e.g., scoring `WEIGHTS`, `DEFAULT_THRESHOLDS`, `DEFAULT_LLM_CALL_TIMEOUT_MS`, `DEFAULT_MAX_LLM_CALLS_PER_SESSION`) from `score.ts`, `llm.ts`, and `index.ts` into this central `config.ts` file.
    *   **Action:** Update all modules to import these constants from `config.ts`.

4.  **Implement Consolidator CLI Configurability:**
    *   **Action:** Create `packages/msp/src/orchestrator/consolidator/cli.ts` (if it doesn't exist).
    *   **Action:** Implement `--llm-provider` and `--embedder-provider` command-line flags using `node:util.parseArgs`.
    *   **Action:** Dynamically pass the selected providers to `createSlmClient` and `createEmbedder` within the CLI, with sensible defaults (`ollama` for LLM, `nomic` for Embedder) if flags are not provided.
    *   **Action:** Update `packages/msp/package.json` to include the `msp-consolidate` binary and script entry.

5.  **Typecheck Validation:**
    *   **Verification:** Ensure `npm run typecheck --workspace=packages/msp` passes cleanly after all changes.

## 6. Desired Outcome

A fully type-safe `packages/msp` workspace where:
-   The `Turn` object is used consistently across all modules.
-   The `SlmClient` interface is complete and functional.
-   Consolidator configuration is centralized and easily tunable.
-   The Consolidator CLI offers dynamic provider selection.
-   All modules compile without TypeScript errors.
-   All existing unit tests (including those for `consolidator` and `compressor`) continue to pass.

Thank you, Claude. Your expertise in resolving these systemic issues will be invaluable.
