import { readdirSync, readFileSync } from 'node:fs'
import { dirname, extname, join, relative, resolve } from 'node:path'
import { exit } from 'node:process'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const ignoredDirectories = new Set(['.git', 'dist', 'node_modules', 'target'])
const textExtensions = new Set([
  '.css',
  '.desktop',
  '.html',
  '.json',
  '.md',
  '.mjs',
  '.rs',
  '.scss',
  '.toml',
  '.ts',
  '.vue',
  '.yaml',
  '.yml',
])
const legacyTerms = [
  ['Bongo', 'Cat'].join(''),
  ['ayang', 'web'].join(''),
  ['Upgrade', 'Link'].join(''),
  ['Git', 'ee'].join(''),
]
const allowedLegacyFiles = new Map([
  ['LICENSE', new Set([legacyTerms[1].toLowerCase()])],
  ['THIRD_PARTY_NOTICES.md', new Set(legacyTerms.map(term => term.toLowerCase()))],
])
const violations = []

function walk(directory) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if (entry.isDirectory() && ignoredDirectories.has(entry.name)) continue

    const path = join(directory, entry.name)

    if (entry.isDirectory()) {
      walk(path)
      continue
    }

    const projectPath = relative(root, path).replaceAll('\\', '/')
    if (projectPath === 'scripts/check-scaffold.mjs') continue
    if (entry.name !== 'LICENSE' && !textExtensions.has(extname(entry.name))) continue

    const content = readFileSync(path, 'utf8')
    const allowedTerms = allowedLegacyFiles.get(projectPath) ?? new Set()

    for (const term of legacyTerms) {
      if (!content.toLowerCase().includes(term.toLowerCase())) continue
      if (allowedTerms.has(term.toLowerCase())) continue

      violations.push(`${projectPath}: contains legacy term ${term}`)
    }
  }
}

walk(root)

const tauriConfig = JSON.parse(readFileSync(join(root, 'src-tauri/tauri.conf.json'), 'utf8'))
const expectedScope = [
  '$RESOURCE/assets/models/**/*',
  '$APPDATA/custom-models/**/*',
]

if (tauriConfig.identifier !== 'com.4096bytes.momopet.live2d') {
  violations.push('src-tauri/tauri.conf.json: unexpected application identifier')
}

if (tauriConfig.app?.security?.csp == null) {
  violations.push('src-tauri/tauri.conf.json: CSP must be enabled')
}

if (tauriConfig.app?.security?.dangerousDisableAssetCspModification === true) {
  violations.push('src-tauri/tauri.conf.json: dangerous asset CSP modification is disabled')
}

if (JSON.stringify(tauriConfig.app?.security?.assetProtocol?.scope) !== JSON.stringify(expectedScope)) {
  violations.push('src-tauri/tauri.conf.json: asset protocol scope is broader than the model roots')
}

if (tauriConfig.bundle?.createUpdaterArtifacts !== false) {
  violations.push('src-tauri/tauri.conf.json: updater artifacts must remain disabled')
}

for (const capability of ['main.json', 'preference.json']) {
  const path = join(root, 'src-tauri/capabilities', capability)
  const content = readFileSync(path, 'utf8')
  const broadPermissions = [
    ['fs:read', '-all'].join(''),
    ['fs:write', '-all'].join(''),
  ]

  for (const permission of broadPermissions) {
    if (content.includes(permission)) {
      violations.push(`src-tauri/capabilities/${capability}: contains ${permission}`)
    }
  }
}

if (violations.length > 0) {
  console.error(['MomoPet scaffold audit failed:', ...violations.map(item => `- ${item}`)].join('\n'))
  exit(1)
}

console.log('MomoPet scaffold audit passed.')
