---
id: AUDIT--LIVE-CALL-GRAPH-FIX
phase: 6
type: audit
status: stable
vault_id: GKS-CORE
tier: process
source_type: episodic
title: Root Cause Analysis — Layout and Security Issues in codev_dashboard
created_at: 2026-05-28T15:35:00.000+07:00
tags: [audit, rca, ui, hotfix]
aliases:
  - AUDIT
  - ops
  - Test results / quality report
cluster: ops
role: Test results / quality report
crosslinks:
  references: []
linked_symbols: []
attributes:
  domain: general
---

# Root Cause Analysis & Proposed Fix: Layout and Security Issues

## [ROOT CAUSE]

1. **Race Condition in UI Update:** In Turn 4, two parallel `replace` calls were made to `codev_dashboard.html`. The script injection completed after the button addition but was based on the original file content, effectively reverting the button addition. This left `#btn-view-callgraph` missing from the DOM.
2. **Layout Inconsistency (Ratio):** The `main` tag currently has `p-4 gap-4` and `h-[calc(100vh-3.5rem)]`, which deviates from the full-bleed design found in the project's backup files. This causes the central panel to appear as a floating card with margins, breaking the intended display ratio.
3. **Security Origin Error:** The `file://` protocol treats each file as a unique origin. The error "Unsafe attempt to load URL..." usually occurs when an iframe (even a hidden one used by a library) tries to access a parent or navigate. While no explicit iframes were added, the missing button might have caused a script error that triggered unexpected browser behavior or a navigation attempt.

## Proposed Changes

### 1. UI Restoration (codev_dashboard.html)
- Restore the `#btn-view-callgraph` button in the view switcher header.

### 2. Layout Correction (codev_dashboard.html)
- Remove `p-4 gap-4` from the `<main>` tag.
- Adjust the `<main>` height to `h-[calc(100vh-3.5rem)]` (preserving the calculated height for the header).
- Ensure the `<section>` (center panel) takes full width/height correctly.

### 3. JavaScript Robustness
- Ensure `switchMainView` and other functions handle potential `null` elements without crashing.

---

Please review and approve this fix. I will apply the changes once approved.
*(Note: I will apply the changes sequentially in separate turns to avoid another race condition.)*