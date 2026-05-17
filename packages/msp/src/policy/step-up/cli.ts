#!/usr/bin/env node
import { parseArgs } from 'node:util'
import * as readline from 'node:readline/promises'
import { stdin as input, stdout as output } from 'node:process'

import { PinProvider } from './pin-provider.js'

const HELP = `msp-auth — Step-up authentication management

Usage:
  msp-auth set-pin

Flags:
  --help             this message
`

async function main(): Promise<number> {
  const { values, positionals } = parseArgs({
    options: {
      help: { type: 'boolean' },
    },
    allowPositionals: true,
  })

  if (values.help || positionals.length === 0) {
    process.stdout.write(HELP)
    return 0
  }

  const command = positionals[0]

  switch (command) {
    case 'set-pin':
      return await doSetPin()
    default:
      process.stderr.write(`error: unknown command ${command}\n${HELP}`)
      return 1
  }
}

async function doSetPin(): Promise<number> {
  const rl = readline.createInterface({ input, output })
  
  try {
    // Note: Standard readline doesn't support hidden input easily without external deps or complex TTY logic.
    // In this environment, we'll do a simple prompt.
    // For a real CLI, we'd use a library that supports silent/hidden input.
    const pin = await rl.question('Enter new PIN: ')
    const confirm = await rl.question('Confirm PIN: ')

    if (pin !== confirm) {
      process.stderr.write('error: PINs do not match\n')
      return 1
    }

    if (pin.length < 4) {
      process.stderr.write('error: PIN must be at least 4 characters\n')
      return 1
    }

    const provider = new PinProvider()
    await provider.setPin(pin)
    
    process.stdout.write('✓ PIN set successfully (hashed with scrypt)\n')
    return 0
  } catch (err) {
    process.stderr.write(`error: ${(err as Error).message}\n`)
    return 1
  } finally {
    rl.close()
  }
}

main().then((code) => process.exit(code))
