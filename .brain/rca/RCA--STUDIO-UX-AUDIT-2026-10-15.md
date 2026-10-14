---
version: "0.1.0b"
created_at: "2026-10-15T05:53:12+07:00,Codex,working-tree"
last_update: "2026-10-15T05:53:12+07:00,Codex"
status: open
superseded_by: null
attributes:
  doc_type: "rca"
  domain: "genesis-studio-desktop"
  scope: "Studio local-root setup, graph accessibility, scene label density"
---

# RCA - Genesis Studio UX Audit

Audit findings from the presentation fixture are tracked in [#223](https://github.com/Freshair129/GenesisBlock/issues/223), [#224](https://github.com/Freshair129/GenesisBlock/issues/224), [#225](https://github.com/Freshair129/GenesisBlock/issues/225), and [#226](https://github.com/Freshair129/GenesisBlock/issues/226).
They describe reproducible product defects; this audit did not establish a production outage.

## Incident 1 - Machine-specific local-root presets (#223)

### Symptom

Local sample presets put absolute `G:\GenesisBlock_Dev\GenesisBlock\...` paths into the data-root field. Those paths do not exist in the audited checkout, while Connect remains enabled.

### Evidence

- `studio/src/App.tsx` defines the sample roots as absolute G: paths and renders them as preset buttons.
- The Connections screen has no directory picker or path-existence preflight.
- `studio/src/transports/backend.ts` forwards the path to `studio_open_local`; `studio/src-tauri/src/lib.rs` passes it to read-only `Storage::open`.
- `docs/SPEC--GENESIS-STUDIO-DESKTOP.md` already requires a native file picker.

### Root Cause

The presets encode one developer machine's filesystem layout, and the local connection UI does not validate or browse for a usable data root.

### Why It Escaped Detection

Presentation mode uses a mock transport, so its successful walkthrough did not exercise the local-root presets or native open command.

### Prevention

- Keep mock presentation separate from local storage selection.
- Use a native directory picker and preflight the selected path as an existing directory.
- Never ship local presets that depend on a developer-machine absolute path.

## Incident 2 - Graph nodes unavailable to keyboard and screen-reader users (#224)

### Symptom

Tab navigation reaches only the five Spotlight buttons, skips the graph canvas, and provides no per-node or relationship semantics.

### Evidence

- `studio/src/features/graph/GraphCanvas.tsx` handles pointer `clickNode` events and returns a generic graph `div`.
- `studio/src/App.tsx` renders Spotlight controls for only the first five scene nodes.
- The audit's keyboard traversal left the graph after those controls.

### Root Cause

The canvas renderer has no semantic node list or keyboard selection layer; its accessible name describes the graph container but not its contents.

### Why It Escaped Detection

A pointer-driven visual walkthrough demonstrated clickable nodes but did not exercise keyboard or assistive-technology navigation.

### Prevention

- Provide a filterable semantic list for the bounded scene, synchronized with canvas and inspector selection.
- Expose node identity and relevant relation direction in that semantic representation.
- Keep keyboard and assistive-technology navigation in the Studio UI acceptance criteria.

## Incident 3 - Initial graph labels overlap (#225)

### Symptom

At 1280x720, labels in the 240-node presentation scene overlap in the central cluster and cannot be scanned reliably.

### Evidence

- The audit screenshot showed overlapping labels at the initial graph zoom.
- `studio/src/features/graph/GraphCanvas.tsx` sets `labelRenderedSizeThreshold` to `7`.
- The Studio graph workspace requests a 240-node scene, below the resource ceiling but visually dense.

### Root Cause

The renderer applies a count/size threshold without a visual-density policy for labels.

### Why It Escaped Detection

Scene node ceilings constrain resource use but do not measure label collisions or readability at a normal desktop viewport.

### Prevention

- Make default labels density-aware and emphasize focused or selected nodes.
- Keep all nodes available through the semantic node list when canvas labels are reduced.
- Evaluate label readability separately from node/edge budget compliance.

## Incident 4 - Graph filter and HQL editor lack accessible names (#226)

### Symptom

The graph filter and HQL editor appear as unnamed editable controls in the inspected accessibility tree.

### Evidence

- The graph filter label contains an icon and input but no text label.
- The visible `READ-ONLY HQL` text is a separate span from the textarea.

### Root Cause

These controls rely on icon-only or nearby visual text instead of a programmatic accessible name.

### Why It Escaped Detection

The controls are visually recognizable in a mouse-driven walkthrough; only accessibility-tree inspection exposed the missing names.

### Prevention

- Give the graph filter and HQL editor stable semantic names associated with their purpose.
- Preserve accessible names when visual labels are compact or icon-only.

## Evidence Limits

- The graph and accessibility findings came from the front-end presentation preview; the native Tauri accessibility tree was not separately checked.
- No real local data root or remote server was opened during the audit.
- No production outage or full WCAG conformance claim is asserted.

## CHANGELOG

| Version | Date | Status | Summary | Commit Hash | Agent |
|---------|------|--------|---------|-------------|-------|
| 0.1.0b | 2026-10-15 | open | Recorded the four Studio UX/accessibility audit findings with root causes and prevention actions. | working-tree | Codex |
