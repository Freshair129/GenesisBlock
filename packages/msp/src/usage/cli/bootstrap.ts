import { cp, mkdir, writeFile } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { Command } from 'commander'

/**
 * Production Bootstrapper.
 * Instantiates a clean GKS/MSP project from the framework source.
 */
export function addBootstrapCommand(program: Command) {
  program
    .command('bootstrap <targetDir>')
    .description('Instantiate a fresh production-ready GKS/MSP project')
    .option('--name <name>', 'Project name (namespace)', 'new-project')
    .action(async (targetDir, opts) => {
      const srcRoot = process.cwd()
      const destRoot = resolve(targetDir)
      const projectName = opts.name

      try {
        process.stdout.write(`🚀 Bootstrapping production project: ${projectName} at ${destRoot}\n`)

        // 1. Create directory structure
        const dirs = [
          'packages/msp/dist',
          'packages/gks/dist',
          'apps/cli/dist',
          'apps/mcp/dist',
          'gks/concept',
          'gks/adr',
          'gks/feat',
          'gks/blueprint',
          'gks/audit',
          'gks/00_index',
          `.brain/msp/projects/${projectName}/memory`,
          `.brain/msp/projects/${projectName}/session`,
          `.brain/msp/projects/${projectName}/vector`,
          `.brain/msp/projects/${projectName}/audit`,
          `.brain/msp/projects/${projectName}/candidates`,
          'msp/LLM_Contract'
        ]

        for (const dir of dirs) {
          await mkdir(join(destRoot, dir), { recursive: true })
        }

        // 2. Copy essential binaries and logic (Compiled only)
        process.stdout.write('📦 Copying core binaries...\n')
        await cp(join(srcRoot, 'packages/msp/dist'), join(destRoot, 'packages/msp/dist'), { recursive: true })
        await cp(join(srcRoot, 'packages/gks/dist'), join(destRoot, 'packages/gks/dist'), { recursive: true })
        await cp(join(srcRoot, 'apps/cli/dist'), join(destRoot, 'apps/cli/dist'), { recursive: true })
        await cp(join(srcRoot, 'apps/mcp/dist'), join(destRoot, 'apps/mcp/dist'), { recursive: true })

        // 3. Copy contracts and schemas
        process.stdout.write('📜 Copying schemas and contracts...\n')
        await cp(join(srcRoot, 'atom_schema.yaml'), join(destRoot, 'atom_schema.yaml'))
        await cp(join(srcRoot, 'atom_registry.yaml'), join(destRoot, 'atom_registry.yaml'))
        await cp(join(srcRoot, 'msp/LLM_Contract'), join(destRoot, 'msp/LLM_Contract'), { recursive: true })

        // 4. Instantiate foundational atoms
        process.stdout.write('🧬 Initializing foundational axioms...\n')
        const foundationAtoms = [
          'gks/framework/FRAMEWORK--MSP-ARCHITECTURE-V2.md',
          'gks/concept/CONCEPT--KNOWLEDGE-LAYERS-V2.md',
          'gks/framework/FRAMEWORK--PHASE-GOVERNANCE.md'
        ]

        for (const atom of foundationAtoms) {
          const destPath = join(destRoot, atom)
          await mkdir(join(destRoot, atom.split('/').slice(0, -1).join('/')), { recursive: true })
          await cp(join(srcRoot, atom), destPath)
        }

        // 5. Create a minimal package.json for the new project
        await writeFile(join(destRoot, 'package.json'), JSON.stringify({
          name: projectName,
          version: '0.1.0',
          private: true,
          type: 'module',
          scripts: {
            "gks": "node ./apps/cli/dist/index.js",
            "msp:validate": "node ./packages/msp/dist/validator/cli.js --all --root=.",
            "msp:index": "node ./packages/msp/dist/usage/cli/re-indexer.js --root=."
          }
        }, null, 2))

        process.stdout.write(`\n✅ Bootstrap complete! Project is ready at ${destRoot}\n`)
        process.stdout.write(`Run: cd ${targetDir} && npm run gks list\n`)

      } catch (err) {
        process.stderr.write(`✗ Bootstrap failed: ${(err as Error).message}\n`)
        process.exit(1)
      }
    })
}
