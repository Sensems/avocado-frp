import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')

function readJson(relPath) {
  return JSON.parse(readFileSync(join(root, relPath), 'utf8'))
}

function readCargoPackageVersion(relPath) {
  const text = readFileSync(join(root, relPath), 'utf8')
  const match = text.match(
    /^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m,
  )
  if (!match) {
    throw new Error(`Could not parse [package].version from ${relPath}`)
  }
  return match[1]
}

const packageVersion = readJson('package.json').version
const tauriVersion = readJson('src-tauri/tauri.conf.json').version
const cargoVersion = readCargoPackageVersion('src-tauri/Cargo.toml')

const rows = [
  ['package.json', packageVersion],
  ['src-tauri/tauri.conf.json', tauriVersion],
  ['src-tauri/Cargo.toml', cargoVersion],
]

console.log('App version SSOT check (tauri.conf.json is source of truth):')
for (const [file, version] of rows) {
  console.log(`  ${file}: ${version}`)
}

const expected = tauriVersion
const mismatches = rows.filter(([, version]) => version !== expected)

if (mismatches.length > 0) {
  console.error('\nVersion mismatch detected:')
  for (const [file, version] of mismatches) {
    console.error(
      `  ${file}: got "${version}", expected "${expected}" (tauri.conf.json)`,
    )
  }
  process.exit(1)
}

console.log(`\nOK — all versions aligned at ${expected}`)
