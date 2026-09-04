#!/usr/bin/env node

// npm executable entrypoint. Keep the actual MCP implementation in server.js
// so repo-local `npm run mcp:start` and registry-installed `genesisblock-mcp`
// exercise the same code path.
require('./server.js');
