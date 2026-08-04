import { createHash } from 'node:crypto'
import {
  copyFileSync,
  createWriteStream,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  renameSync,
  rmSync,
  statSync,
  unlinkSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { basename, dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { pipeline } from 'node:stream/promises'
import { execFileSync, spawnSync } from 'node:child_process'
import { Readable } from 'node:stream'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const binDir = join(root, 'src-tauri', 'bin')
const manifestPath = join(root, 'sidecar.manifest.json')

const SUPPORTED_TARGETS = [
  'x86_64-pc-windows-msvc',
  'x86_64-unknown-linux-gnu',
  'aarch64-apple-darwin',
  'x86_64-apple-darwin',
]

const KINDS = ['frpc', 'frps']

function fail(message) {
  console.error(`ERROR: ${message}`)
  process.exit(1)
}

function parseArgs(argv) {
  let target = null
  let all = false
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i]
    if (arg === '--all') {
      all = true
      continue
    }
    if (arg === '--target') {
      const value = argv[i + 1]
      if (!value || value.startsWith('--')) {
        fail('--target requires a rustc triple')
      }
      target = value
      i += 1
      continue
    }
    fail(`Unknown argument: ${arg}`)
  }
  if (all && target) {
    fail('Use either --all or --target <triple>, not both')
  }
  return { target, all }
}

function resolveHostTriple() {
  const { platform, arch } = process
  if (platform === 'win32' && arch === 'x64') return 'x86_64-pc-windows-msvc'
  if (platform === 'linux' && arch === 'x64') return 'x86_64-unknown-linux-gnu'
  if (platform === 'darwin' && arch === 'arm64') return 'aarch64-apple-darwin'
  if (platform === 'darwin' && arch === 'x64') return 'x86_64-apple-darwin'
  return null
}

function detectHostTriple() {
  const triple = resolveHostTriple()
  if (!triple) {
    fail(
      `Unsupported host platform ${process.platform}/${process.arch}; pass --target <triple> or --all`,
    )
  }
  return triple
}

function loadManifest() {
  if (!existsSync(manifestPath)) {
    fail(`Missing manifest: ${manifestPath}`)
  }
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'))
  if (!manifest?.frpVersion || typeof manifest.frpVersion !== 'string') {
    fail('sidecar.manifest.json must include string frpVersion')
  }
  if (!manifest.artifacts || typeof manifest.artifacts !== 'object') {
    fail('sidecar.manifest.json must include artifacts map')
  }
  for (const triple of SUPPORTED_TARGETS) {
    const entry = manifest.artifacts[triple]
    if (!entry) fail(`Manifest missing artifacts for ${triple}`)
    for (const kind of KINDS) {
      const artifact = entry[kind]
      if (!artifact?.url || !artifact?.sha256) {
        fail(`Manifest missing ${kind}.url/sha256 for ${triple}`)
      }
      if (!/^[0-9a-f]{64}$/i.test(artifact.sha256)) {
        fail(`Invalid sha256 for ${kind} @ ${triple}`)
      }
    }
  }
  return manifest
}

function isWindowsTriple(triple) {
  return triple.includes('windows')
}

function destFileName(kind, triple) {
  return isWindowsTriple(triple) ? `${kind}-${triple}.exe` : `${kind}-${triple}`
}

function destPath(kind, triple) {
  return join(binDir, destFileName(kind, triple))
}

function sha256File(filePath) {
  const hash = createHash('sha256')
  hash.update(readFileSync(filePath))
  return hash.digest('hex')
}

function localMatches(filePath, expectedSha) {
  if (!existsSync(filePath)) return false
  return sha256File(filePath).toLowerCase() === expectedSha.toLowerCase()
}

function safeRm(path) {
  try {
    if (!existsSync(path)) return
    const st = statSync(path)
    if (st.isDirectory()) rmSync(path, { recursive: true, force: true })
    else unlinkSync(path)
  } catch {
    // best-effort cleanup of temp paths
  }
}

async function downloadToFile(url, outPath) {
  const response = await fetch(url)
  if (!response.ok || !response.body) {
    throw new Error(`Download failed (${response.status}) for ${url}`)
  }
  await pipeline(Readable.fromWeb(response.body), createWriteStream(outPath))
}

function findExtractedBinary(extractDir, binaryName) {
  const stack = [extractDir]
  while (stack.length > 0) {
    const dir = stack.pop()
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = join(dir, entry.name)
      if (entry.isDirectory()) {
        stack.push(full)
        continue
      }
      if (entry.isFile() && entry.name === binaryName) {
        return full
      }
    }
  }
  return null
}

function extractArchive(archivePath, extractDir) {
  mkdirSync(extractDir, { recursive: true })
  // Windows 10+ and Unix both ship tar that can open zip / tar.gz.
  execFileSync('tar', ['-xf', archivePath, '-C', extractDir], {
    stdio: 'pipe',
  })
}

function atomicReplace(srcPath, destPathFinal) {
  const destDir = dirname(destPathFinal)
  mkdirSync(destDir, { recursive: true })
  const staging = join(
    destDir,
    `.${basename(destPathFinal)}.${process.pid}.tmp`,
  )
  safeRm(staging)
  copyFileSync(srcPath, staging)
  try {
    renameSync(staging, destPathFinal)
  } catch {
    // Windows may block rename over an existing executable; fall back to copy+unlink staging.
    copyFileSync(staging, destPathFinal)
    safeRm(staging)
  }
}

function canSmokeVersion(triple) {
  return resolveHostTriple() === triple
}

function smokeVersion(filePath, frpVersion) {
  const result = spawnSync(filePath, ['--version'], {
    encoding: 'utf8',
    timeout: 15_000,
  })
  if (result.error) {
    fail(`Failed to execute ${filePath}: ${result.error.message}`)
  }
  const output = `${result.stdout ?? ''}${result.stderr ?? ''}`
  if (!output.includes(frpVersion)) {
    fail(
      `--version for ${filePath} did not contain ${frpVersion}\nOutput:\n${output}`,
    )
  }
  console.log(`  version smoke OK: ${output.trim().split(/\r?\n/)[0]}`)
}

async function prepareTarget(triple, manifest) {
  if (!SUPPORTED_TARGETS.includes(triple)) {
    fail(
      `Unsupported target ${triple}. Supported: ${SUPPORTED_TARGETS.join(', ')}`,
    )
  }

  const entry = manifest.artifacts[triple]
  console.log(`\nPreparing sidecars for ${triple} (frp ${manifest.frpVersion})`)

  const pending = []
  for (const kind of KINDS) {
    const artifact = entry[kind]
    const dest = destPath(kind, triple)
    if (localMatches(dest, artifact.sha256)) {
      console.log(`  skip ${kind}: SHA256 match (${destFileName(kind, triple)})`)
    } else {
      pending.push(kind)
      if (existsSync(dest)) {
        console.log(
          `  refresh ${kind}: local SHA256 mismatch or missing expected hash`,
        )
      } else {
        console.log(`  download ${kind}: missing ${destFileName(kind, triple)}`)
      }
    }
  }

  if (pending.length === 0) {
    if (canSmokeVersion(triple)) {
      for (const kind of KINDS) {
        smokeVersion(destPath(kind, triple), manifest.frpVersion)
      }
    }
    return { refreshed: [] }
  }

  // Official FRP ships both binaries in one archive; download once per unique URL.
  const urls = [...new Set(pending.map((kind) => entry[kind].url))]
  const workRoot = mkdtempSync(join(tmpdir(), 'prepare-sidecars-'))
  const refreshed = []

  try {
    for (const url of urls) {
      const archiveName = url.split('/').pop() || 'frp-archive'
      const archivePath = join(workRoot, archiveName)
      const extractDir = join(workRoot, `extract-${createHash('sha1').update(url).digest('hex').slice(0, 8)}`)

      console.log(`  downloading ${url}`)
      await downloadToFile(url, archivePath)
      extractArchive(archivePath, extractDir)

      const kindsForUrl = pending.filter((kind) => entry[kind].url === url)
      for (const kind of kindsForUrl) {
        const artifact = entry[kind]
        const binaryName = isWindowsTriple(triple) ? `${kind}.exe` : kind
        const extracted = findExtractedBinary(extractDir, binaryName)
        if (!extracted) {
          fail(`Archive from ${url} did not contain ${binaryName}`)
        }

        const actual = sha256File(extracted).toLowerCase()
        const expected = artifact.sha256.toLowerCase()
        if (actual !== expected) {
          // Fail closed: do not place unverified bytes into src-tauri/bin.
          fail(
            `SHA256 mismatch for extracted ${kind} @ ${triple}\n  expected: ${expected}\n  actual:   ${actual}`,
          )
        }

        const dest = destPath(kind, triple)
        atomicReplace(extracted, dest)

        const placed = sha256File(dest).toLowerCase()
        if (placed !== expected) {
          safeRm(dest)
          fail(
            `Post-write SHA256 mismatch for ${dest}; removed dirty file\n  expected: ${expected}\n  actual:   ${placed}`,
          )
        }

        console.log(`  placed ${destFileName(kind, triple)} (${placed})`)
        refreshed.push(kind)
      }
    }
  } finally {
    safeRm(workRoot)
  }

  if (canSmokeVersion(triple)) {
    for (const kind of KINDS) {
      smokeVersion(destPath(kind, triple), manifest.frpVersion)
    }
  }

  return { refreshed }
}

async function main() {
  const { target, all } = parseArgs(process.argv.slice(2))
  const manifest = loadManifest()
  const triples = all
    ? SUPPORTED_TARGETS
    : [target ?? detectHostTriple()]

  mkdirSync(binDir, { recursive: true })

  console.log(`sidecar.manifest.json frpVersion=${manifest.frpVersion}`)
  console.log(`targets: ${triples.join(', ')}`)

  for (const triple of triples) {
    await prepareTarget(triple, manifest)
  }

  console.log('\nOK — sidecar prepare complete')
}

main().catch((error) => {
  fail(error?.stack || String(error))
})
