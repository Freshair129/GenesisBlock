import * as http from 'http';
import { App } from 'obsidian';
import { PgvectorManager } from './pgvector.js';

export interface ServerOptions {
  port: number;
  dbConnectionString?: string;
}

export class BridgeServer {
  private server: http.Server | null = null;
  private pg: PgvectorManager | null = null;

  constructor(private app: App) {}

  public async start(opts: ServerOptions) {
    if (opts.dbConnectionString) {
      this.pg = new PgvectorManager({ connectionString: opts.dbConnectionString });
      await this.pg.connect();
    }

    this.server = http.createServer(async (req, res) => {
      // CORS headers
      res.setHeader('Access-Control-Allow-Origin', '*');
      res.setHeader('Access-Control-Allow-Methods', 'GET, POST, PATCH, DELETE, OPTIONS');
      res.setHeader('Access-Control-Allow-Headers', 'Content-Type');

      if (req.method === 'OPTIONS') {
        res.writeHead(204);
        res.end();
        return;
      }

      const url = new URL(req.url || '', `http://localhost:${opts.port}`);
      
      try {
        if (url.pathname === '/api/status' && req.method === 'GET') {
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ 
            name: 'msp-bridge', 
            version: '0.1.0',
            db_connected: !!this.pg 
          }));
          return;
        }

        if (url.pathname === '/api/semantic-search' && req.method === 'POST') {
          const body = await this.readBody(req);
          const data = JSON.parse(body);
          
          if (!this.pg) {
            res.writeHead(503, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: 'Database not connected' }));
            return;
          }

          const hits = await this.pg.search(data.vector, data.limit || 10);
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ hits }));
          return;
        }

        if (url.pathname === '/api/insert' && req.method === 'POST') {
          const body = await this.readBody(req);
          const data = JSON.parse(body);
          
          if (!this.pg) {
            res.writeHead(503, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: 'Database not connected' }));
            return;
          }

          await this.pg.insert({
            id: data.id,
            store: data.store || 'default',
            source: data.source || 'obsidian',
            chunk_id: data.chunk_id || '0',
            text: data.text,
            vector: data.vector,
            metadata: data.metadata || {}
          });
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ ok: true }));
          return;
        }

        if (url.pathname === '/api/delete' && req.method === 'POST') {
          const body = await this.readBody(req);
          const data = JSON.parse(body);
          
          if (!this.pg) {
            res.writeHead(503, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: 'Database not connected' }));
            return;
          }

          await this.pg.delete(data.id, data.store || 'default');
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ ok: true }));
          return;
        }

        if (url.pathname === '/api/insert-batch' && req.method === 'POST') {
          const body = await this.readBody(req);
          const items = JSON.parse(body);
          
          if (!this.pg) {
            res.writeHead(503, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: 'Database not connected' }));
            return;
          }

          for (const item of items) {
            await this.pg.insert({
              id: item.id,
              store: item.store || 'default',
              source: item.source || 'obsidian',
              chunk_id: item.chunk_id || '0',
              text: item.text,
              vector: item.vector,
              metadata: item.metadata || {}
            });
          }
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ ok: true }));
          return;
        }

        // TODO: Implement other endpoints (insert-batch, patch-metadata, etc.)
        
        res.writeHead(404, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'Not Found' }));

      } catch (err) {
        console.error(`[bridge-server] error: ${(err as Error).message}`);
        res.writeHead(500, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: (err as Error).message }));
      }
    });

    this.server.listen(opts.port, () => {
      console.log(`msp-bridge server listening on port ${opts.port}`);
    });
  }

  private readBody(req: http.IncomingMessage): Promise<string> {
    return new Promise((resolve, reject) => {
      let body = '';
      req.on('data', chunk => body += chunk.toString());
      req.on('end', () => resolve(body));
      req.on('error', reject);
    });
  }

  public stop() {
    if (this.server) {
      this.server.close();
      console.log('msp-bridge server stopped');
    }
  }
}
