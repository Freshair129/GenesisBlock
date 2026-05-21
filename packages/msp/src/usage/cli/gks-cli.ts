import { readFile } from 'node:fs/promises'
import { join } from 'node:path'
import { Command } from 'commander'

/**
 * Master GKS CLI Implementation.
 * Canonical home for GKS interaction.
 */
const program = new Command()
  .name('gks')
  .description('Genesis Knowledge System CLI — for humans and AI agents')
  .version('0.2.0')

// ── List ─────────────────────────────────────────────────────────────────────
program
  .command('list')
  .description('List all atoms in the index')
  .option('-t, --type <type>', 'filter by atom type')
  .option('--json', 'output as JSON')
  .action(async (opts) => {
    try {
      const indexPath = join(process.cwd(), 'gks/00_index/atomic_index.jsonl')
      const content = await readFile(indexPath, 'utf8')
      const lines = content.trim().split('\n')
      const atoms = lines.map(line => JSON.parse(line))

      const filtered = opts.type 
        ? atoms.filter(a => a.type.toLowerCase() === opts.type.toLowerCase())
        : atoms

      if (opts.json) {
        process.stdout.write(JSON.stringify(filtered, null, 2) + '\n')
      } else {
        filtered.forEach(a => {
          process.stdout.write(`[${a.type.padEnd(10)}] ${a.id}\n`)
        })
        process.stdout.write(`\nTotal: ${filtered.length} atoms\n`)
      }
    } catch (err) {
      process.stderr.write(`Error: ${(err as Error).message}\n`)
      process.exit(1)
    }
  })

// ── Get ─────────────────────────────────────────────────────────────────────
program
  .command('get <id>')
  .description('Get atom content and metadata by ID')
  .option('--json', 'output as JSON')
  .action(async (id, opts) => {
    try {
      const indexPath = join(process.cwd(), 'gks/00_index/atomic_index.jsonl')
      const content = await readFile(indexPath, 'utf8')
      const lines = content.trim().split('\n')
      const atoms = lines.map(line => JSON.parse(line))
      
      const atom = atoms.find(a => a.id === id)
      if (!atom) {
        process.stderr.write(`Error: Atom ${id} not found in index.\n`)
        process.exit(1)
      }

      const fullPath = join(process.cwd(), atom.filepath)
      const rawContent = await readFile(fullPath, 'utf8')

      if (opts.json) {
        process.stdout.write(JSON.stringify({ ...atom, content: rawContent }, null, 2) + '\n')
      } else {
        process.stdout.write(rawContent + '\n')
      }
    } catch (err) {
      process.stderr.write(`Error: ${(err as Error).message}\n`)
      process.exit(1)
    }
  })

program.parse()
