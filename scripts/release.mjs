#!/usr/bin/env node
// Cuts a release: bumps the version in every file that carries it, commits just
// those files, tags, and pushes — which triggers the GitHub Actions build.
//
//   npm run release -- v0.2.0   (the leading "v" is optional)
//   npm run release             (no version → prompts for major/minor/patch,
//                                defaulting to a patch bump of the current version)
//
// The actual release build (compile, sign, bundle, publish) happens in GitHub
// Actions once the tag is pushed — nothing is built locally for distribution.
// Before tagging, this runs a quick local compile to catch a broken build so we
// never push a tag that fails CI. Skip that with --skip-checks if you're sure.
//
// Any other uncommitted changes are intentionally left alone — only the version
// files are staged and committed.

import { readFileSync, writeFileSync } from 'node:fs'
import { execSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'
import { createInterface } from 'node:readline/promises'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')

function fail(msg) {
  console.error(`\n✗ ${msg}\n`)
  process.exit(1)
}

function run(cmd) {
  console.log(`  $ ${cmd}`)
  execSync(cmd, { cwd: root, stdio: 'inherit' })
}

function capture(cmd) {
  return execSync(cmd, { cwd: root, encoding: 'utf8' }).trim()
}

// --- Read the current version so we can bump or validate against it --------

const pkgPath = join(root, 'package.json')
const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'))
const current = pkg.version
if (!/^\d+\.\d+\.\d+$/.test(current)) {
  fail(`Current package.json version "${current}" is not a valid semver.`)
}

/** Bump the current x.y.z by the given release type. */
function nextVersion(type) {
  const [major, minor, patch] = current.split('.').map(Number)
  if (type === 'major') return `${major + 1}.0.0`
  if (type === 'minor') return `${major}.${minor + 1}.0`
  return `${major}.${minor}.${patch + 1}` // patch
}

/** Ask which part to bump; default to patch. Returns the bumped version. */
async function promptForBump() {
  // Non-interactive (CI, piped) — just default to a patch bump, no prompt.
  if (!process.stdin.isTTY) return nextVersion('patch')

  const rl = createInterface({ input: process.stdin, output: process.stdout })
  try {
    console.log(`\nCurrent version is ${current}. Which part do you want to bump?`)
    console.log(`  1) patch  → ${nextVersion('patch')}  (default)`)
    console.log(`  2) minor  → ${nextVersion('minor')}`)
    console.log(`  3) major  → ${nextVersion('major')}`)
    const answer = (await rl.question('Choose [1-3, patch/minor/major]: ')).trim().toLowerCase()
    if (answer === '2' || answer === 'minor') return nextVersion('minor')
    if (answer === '3' || answer === 'major') return nextVersion('major')
    return nextVersion('patch') // empty, "1", "patch", or anything else
  } finally {
    rl.close()
  }
}

// --- Parse + validate the requested version -------------------------------

const args = process.argv.slice(2)
const skipChecks = args.includes('--skip-checks')
const raw = args.find((a) => !a.startsWith('--'))

// No explicit version → prompt for a major/minor/patch bump (defaults to patch).
const version = raw ? raw.replace(/^v/, '') : await promptForBump()
const tag = `v${version}` // tag always has the leading v
if (!/^\d+\.\d+\.\d+$/.test(version)) {
  fail(`"${raw}" is not a valid semver version (expected e.g. v0.2.0)`)
}

if (current === version) fail(`Version is already ${version} — nothing to bump.`)

// Refuse to clobber an existing tag.
const existingTags = capture('git tag --list').split('\n')
if (existingTags.includes(tag)) fail(`Tag ${tag} already exists.`)

console.log(`\nReleasing ${current} → ${version} (tag ${tag})\n`)

// --- Pre-flight: compile locally so a broken tag never reaches CI ----------

if (skipChecks) {
  console.log('Skipping pre-flight build checks (--skip-checks).\n')
} else {
  console.log('Pre-flight: building frontend + checking Rust (--skip-checks to bypass)…')
  run('npm run build')
  run('cargo check --manifest-path src-tauri/Cargo.toml')
  console.log('')
}

// --- Bump every file that carries the version -----------------------------

/** Replace exactly one match of `re` in the file at `rel`, or fail loudly. */
function bump(rel, re, replacement) {
  const path = join(root, rel)
  const before = readFileSync(path, 'utf8')
  const after = before.replace(re, replacement)
  if (after === before) fail(`Could not find the version to bump in ${rel}`)
  writeFileSync(path, after)
  console.log(`  ✎ ${rel}`)
}

// package.json + tauri.conf.json: first top-level "version": "x.y.z"
const jsonVersion = /"version":\s*"\d+\.\d+\.\d+"/
bump('package.json', jsonVersion, `"version": "${version}"`)
bump('src-tauri/tauri.conf.json', jsonVersion, `"version": "${version}"`)

// Cargo.toml: the [package] version (line-anchored; deps use inline `version =`)
bump('src-tauri/Cargo.toml', /^version = "\d+\.\d+\.\d+"/m, `version = "${version}"`)

// Cargo.lock: only the trackmyfi-app entry, not some other crate at 0.1.0
bump(
  'src-tauri/Cargo.lock',
  /(name = "trackmyfi-app"\nversion = )"\d+\.\d+\.\d+"/,
  `$1"${version}"`,
)

// --- Commit, tag, push ----------------------------------------------------

const branch = capture('git rev-parse --abbrev-ref HEAD')
console.log('')
run(
  'git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock',
)
run(`git commit -m "chore: release ${tag}"`)
run(`git tag ${tag}`)
run(`git push origin ${branch}`)
run(`git push origin ${tag}`)

console.log(`\n✓ Released ${tag}. GitHub Actions is now building the release.`)
console.log('  Watch it at: https://github.com/TrackMyFi/site/actions\n')
