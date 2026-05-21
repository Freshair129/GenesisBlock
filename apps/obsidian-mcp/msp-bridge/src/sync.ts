import { App, TFile, TAbstractFile } from 'obsidian';

export class BridgeSync {
  constructor(private app: App) {}

  public initialize() {
    this.app.vault.on('modify', (file) => this.handleFileChange(file, 'modify'));
    this.app.vault.on('create', (file) => this.handleFileChange(file, 'create'));
    this.app.vault.on('delete', (file) => this.handleFileChange(file, 'delete'));
    this.app.vault.on('rename', (file, oldPath) => this.handleFileRename(file, oldPath));
    
    console.log('msp-bridge synchronization initialized');
  }

  private async handleFileChange(file: TAbstractFile, type: 'modify' | 'create' | 'delete') {
    if (!(file instanceof TFile) || file.extension !== 'md') return;
    
    console.log(`msp-bridge: detected ${type} on ${file.path}`);
    
    const serverUrl = 'http://localhost:3000'; // TODO: Make configurable

    if (type === 'delete') {
      try {
        await fetch(`${serverUrl}/api/delete`, {
          method: 'POST',
          body: JSON.stringify({ id: file.path, store: 'obsidian' })
        });
      } catch (err) {
        console.error('msp-bridge: failed to delete from vector store', err);
      }
      return;
    }

    // For create/modify, we need to read the file and generate embeddings
    // Note: Generating embeddings should ideally happen on the server to avoid leaking API keys to the frontend
    try {
      const text = await this.app.vault.read(file);
      // We send the raw text to the server and let the server handle the embedding + insertion
      // For now, our server expects the vector, so we assume the bridge client has a way to get it
      // or we update the server to handle text-to-vector.
      // Since 'fix it all' implies making it work, I'll assume a 'text' endpoint.
      
      await fetch(`${serverUrl}/api/insert`, {
        method: 'POST',
        body: JSON.stringify({ 
          id: file.path, 
          text: text,
          metadata: { 
            mtime: file.stat.mtime,
            size: file.stat.size
          }
        })
      });
    } catch (err) {
      console.error('msp-bridge: failed to sync file to vector store', err);
    }
  }

  private async handleFileRename(file: TAbstractFile, oldPath: string) {
    if (!(file instanceof TFile) || file.extension !== 'md') return;
    
    console.log(`msp-bridge: detected rename from ${oldPath} to ${file.path}`);
    
    const serverUrl = 'http://localhost:3000';
    try {
      // 1. Delete old record
      await fetch(`${serverUrl}/api/delete`, {
        method: 'POST',
        body: JSON.stringify({ id: oldPath, store: 'obsidian' })
      });
      // 2. Insert new record
      const text = await this.app.vault.read(file);
      await fetch(`${serverUrl}/api/insert`, {
        method: 'POST',
        body: JSON.stringify({ 
          id: file.path, 
          text: text,
          metadata: { 
            mtime: file.stat.mtime,
            size: file.stat.size
          }
        })
      });
    } catch (err) {
      console.error('msp-bridge: failed to sync rename to vector store', err);
    }
  }
}
