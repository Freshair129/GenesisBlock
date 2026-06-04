import { App, Plugin, Modal, Notice, TFile, ItemView, WorkspaceLeaf } from 'obsidian';

const VIEW_TYPE_GENESIS = "genesis-sidebar-view";

interface DatabaseStatus {
    open: boolean;
    readOnly: boolean;
    pageCacheMb: number;
}

export default class GenesisShadowSync extends Plugin {
    statusBarIcon: HTMLElement;
    healthCheckInterval: number;

    async onload() {
        console.log('Loading Genesis Shadow Sync...');

        // Step 1: Initialize Status Bar
        this.statusBarIcon = this.addStatusBarItem();
        this.updateStatusBar('loading');

        // Step 2: Register HQL Query Command
        this.addCommand({
            id: 'run-hql-query',
            name: 'Run HQL Query',
            callback: () => {
                new HqlQueryModal(this.app).open();
            }
        });

        // Step 3: Initialize Shadow Sync Watchdog
        this.registerEvent(
            this.app.vault.on('modify', (file) => {
                if (file instanceof TFile && file.extension === 'md') {
                    this.syncNoteToDb(file);
                }
            })
        );

        // Step 4: Register Sidebar View
        this.registerView(
            VIEW_TYPE_GENESIS,
            (leaf) => new GenesisSidebarView(leaf)
        );

        this.addCommand({
            id: "open-genesis-sidebar",
            name: "Open Genesis Sidebar",
            callback: () => this.activateView(),
        });

        // Start Health Check Loop
        this.startHealthCheck();
    }

    async activateView() {
        const { workspace } = this.app;
        let leaf: WorkspaceLeaf | null = null;
        const leaves = workspace.getLeavesOfType(VIEW_TYPE_GENESIS);

        if (leaves.length > 0) {
            leaf = leaves[0];
        } else {
            leaf = workspace.getRightLeaf(false);
            await leaf.setViewState({ type: VIEW_TYPE_GENESIS, active: true });
        }
        workspace.revealLeaf(leaf);
    }

    onunload() {
        if (this.healthCheckInterval) {
            window.clearInterval(this.healthCheckInterval);
        }
    }

    startHealthCheck() {
        this.healthCheckInterval = window.setInterval(async () => {
            await this.checkEngineHealth();
        }, 5000);
        this.checkEngineHealth();
    }

    async checkEngineHealth() {
        try {
            const response = await fetch('http://localhost:3000/v1/status');
            if (response.status === 200) {
                this.updateStatusBar('ready');
            } else if (response.status === 503) {
                this.updateStatusBar('maintenance');
            } else {
                this.updateStatusBar('error');
            }
        } catch (e) {
            this.updateStatusBar('offline');
        }
    }

    updateStatusBar(state: 'ready' | 'maintenance' | 'syncing' | 'error' | 'offline' | 'loading') {
        let text = '';
        let color = '';
        switch (state) {
            case 'ready': text = '🧠 Ready'; color = '#4caf50'; break;
            case 'maintenance': text = '🏗️ Building...'; color = '#ff9800'; break;
            case 'offline': text = '🚫 Offline'; color = '#f44336'; break;
            case 'loading': text = '⌛ Connecting...'; color = '#9e9e9e'; break;
            default: text = '⚠️ Check DB'; color = '#f44336';
        }
        this.statusBarIcon.setText(text);
        this.statusBarIcon.style.color = color;
    }

    async syncNoteToDb(file: TFile) {
        const cache = this.app.metadataCache.getFileCache(file);
        if (!cache) return;

        try {
            await fetch('http://localhost:3000/v1/node/add', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    id: file.basename,
                    labels: ['Note'],
                    props: cache.frontmatter || {},
                    lang: cache.frontmatter?.lang || 'en'
                })
            });
            // Update Sidebar if it's the active file
            const activeFile = this.app.workspace.getActiveFile();
            if (activeFile && activeFile.path === file.path) {
                this.refreshSidebar();
            }
        } catch (e) {
            console.error(`Shadow Sync Failed: ${e}`);
        }
    }

    refreshSidebar() {
        const leaves = this.app.workspace.getLeavesOfType(VIEW_TYPE_GENESIS);
        leaves.forEach(l => (l.view as GenesisSidebarView).updateView());
    }
}

class GenesisSidebarView extends ItemView {
    constructor(leaf: WorkspaceLeaf) {
        super(leaf);
    }

    getViewType() { return VIEW_TYPE_GENESIS; }
    getDisplayText() { return "Genesis Knowledge"; }

    async onOpen() {
        this.updateView();
    }

    async updateView() {
        const container = this.containerEl.children[1];
        container.empty();
        const activeFile = this.app.workspace.getActiveFile();
        if (!activeFile) {
            container.createEl("h4", { text: "No active note" });
            return;
        }

        container.createEl("h3", { text: `🧠 ${activeFile.basename}` });
        
        try {
            // Fetch Transitive Context (Step 4 Inference)
            const query = `TRAVERSE FROM "${activeFile.basename}" DEPTH 1 REL ANY`;
            const response = await fetch('http://localhost:3000/v1/query/hql', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(query)
            });

            if (response.ok) {
                const data = await response.json();
                container.createEl("h4", { text: "Intelligent Neighbors" });
                const list = container.createEl("ul");
                data.forEach((res: any) => {
                    const li = list.createEl("li");
                    li.createEl("b", { text: res.node.id });
                    li.createEl("span", { text: ` (Impact: ${res.node.impact?.toFixed(2)})` });
                });
            }
        } catch (e) {
            container.createEl("p", { text: "Could not reach GenesisDB." });
        }
    }
}

class HqlQueryModal extends Modal {
    onOpen() {
        const { contentEl } = this;
        contentEl.createEl('h2', { text: 'Genesis HQL Console' });
        const inputEl = contentEl.createEl('textarea', { attr: { placeholder: 'TRAVERSE FROM ...' } });
        inputEl.style.width = '100%'; inputEl.style.height = '100px';
        const runBtn = contentEl.createEl('button', { text: 'Run Query' });
        const resultsEl = contentEl.createDiv();

        runBtn.onclick = async () => {
            const response = await fetch('http://localhost:3000/v1/query/hql', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(inputEl.value)
            });
            if (response.ok) {
                const data = await response.json();
                resultsEl.empty();
                data.forEach((res: any) => {
                    resultsEl.createDiv({ text: `📄 ${res.node.id} (${res.node.impact?.toFixed(2)})` });
                });
            }
        };
    }
    onClose() { this.contentEl.empty(); }
}
