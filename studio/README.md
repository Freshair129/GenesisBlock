# Genesis Studio

Genesis Studio is the Tauri desktop controller for GenesisBlockDB. S1 provides
read-only local embedded and remote REST transports through one product
contract. The presentation fixture uses a separate mock transport and does not
open a database.

## Development

```powershell
npm ci
npm test
npm run build
npm run test:e2e
npx tauri build --debug --no-bundle
```

The command above builds the local debug app without creating an installer.
Omit `--no-bundle` to build enabled bundle targets when the host has the
required packaging tools. The CI matrix builds Windows MSI, macOS DMG, and
Linux DEB packages. These are unsigned development artifacts; the matrix is
defined in `.github/workflows/studio-packaging.yml`.

For local mode, select an existing GenesisBlockDB data-root folder with the
native folder picker. Studio opens it read-only and requires exclusive
ownership. For remote mode, provide the base URL and the server's bearer token
when authentication is enabled. Use the presentation fixture only to preview
the UI without connecting to either transport.

The S0-S4 scope, transport contract, release gates, and current evidence are in
[`docs/SPEC--GENESIS-STUDIO-DESKTOP.md`](../docs/SPEC--GENESIS-STUDIO-DESKTOP.md).
S1 is beta implemented; packaged parity, native-picker interaction,
cross-platform startup evidence, and independent architecture/security review
remain release gates.
