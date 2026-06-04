# Functional Specification: Obsidian UI Integration (The Bridge)

## 1. Objective
Build the **Human-to-Machine Bridge** for GenesisDB. This phase implements the Obsidian Plugin components that allow users to interact with the engine's intelligence directly from their Markdown vault.

## 2. UI Components

### 2.1 The Genesis Sidebar (Shadow Graph)
- **View:** A dedicated sidebar panel.
- **Content:** 
    - **Current Node Info:** Displays the K-Impact score and Cluster ID of the active note.
    - **Intelligent Neighbors:** Lists the top 5 neighbors resolved via `INFER(REPORTS_TO)` or physical links.
    - **Similarity Search:** A button to "Find similar notes" across languages (Mark V Step 1).

### 2.2 Status Bar Signaling (R4 Compliance)
- **Ready State:** Shows a green icon: "Brain Ready".
- **Maintenance State (503):** Shows a rotating orange icon: "Building Brain..." (Triggered when the REST API returns 503).
- **Syncing State:** Shows a blue icon: "Syncing Knowledge...".

### 2.3 Query Modal (HQL Interface)
- **Trigger:** Command Palette (`Ctrl+P` -> "GenesisDB: Run HQL Query").
- **Interface:** A simple text input area for HQL commands.
- **Output:** A list of results with clickable links to open the corresponding Obsidian notes.

## 3. The Shadow Sync Watchdog
- **Technology:** `chokidar` (already planned in expansion spec).
- **Logic:**
    1.  Monitor `.md` file changes in the vault.
    2.  On save: Extract Frontmatter (YAML) and Wikilinks.
    3.  Call `storage.add_node()` and `storage.add_edge()` via FFI.
    4.  Handle Axiomatic Guard rejections (e.g., trying to edit a `MASTER` tier note).

## 4. Proposed Architecture
- **Plugin Entry (`main.ts`):** Initializes the `GenesisDatabase` NAPI-RS instance.
- **Event Bus:** Listens for engine events (e.g., `NodeMetadata` updates) and refreshes the sidebar.

## 5. Implementation Roadmap
1.  **Step 1:** Implement the Status Bar indicator and health-check loop.
2.  **Step 2:** Implement the basic HQL Query Modal.
3.  **Step 3:** Implement the Shadow Sync (Markdown -> DB) watchdog.
4.  **Step 4:** Implement the Sidebar Graph View (D3.js or simple list).

Please review and approve this UI Integration Specification. I will generate the bridge code once approved.
