# Genesis Studio S0

Genesis Studio is the candidate desktop controller for GenesisBlockDB. S0 is a
read-only product shell backed exclusively by deterministic fixture data. It does
not open a database, call the REST server, or mutate engine state.

```powershell
npm install
npm test
npm run build
npx tauri build --debug --no-bundle
```

The architecture and phase gates are defined in
`docs/SPEC--GENESIS-STUDIO-DESKTOP.md`.
