#!/usr/bin/env node
// Cuts a release: bumps the version in every file that carries it, commits just
// those files, tags, and pushes — which triggers the GitHub Actions build.
//
//   npm run release -- v0.2.0   (the leading "v" is optional)
//
// Any other uncommitted changes are intentionally left alone — only the version
// files are staged and committed.

import { readFileSync, writeFileSync } from 'node:fs'
import { execSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

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

// --- Parse + validate the requested version -------------------------------

const raw = process.argv[2]
if (!raw) fail('Usage: npm run release -- v0.2.0')

const version = raw.replace(/^v/, '') // bare semver for file contents
const tag = `v${version}` // tag always has the leading v
if (!/^\d+\.\d+\.\d+$/.test(version)) {
  fail(`"${raw}" is not a valid semver version (expected e.g. v0.2.0)`)
}

// --- Read the current version so replacements are precise ------------------

const pkgPath = join(root, 'package.json')
const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'))
const current = pkg.version
if (current === version) fail(`Version is already ${version} — nothing to bump.`)

// Refuse to clobber an existing tag.
const existingTags = capture('git tag --list').split('\n')
if (existingTags.includes(tag)) fail(`Tag ${tag} already exists.`)

console.log(`\nReleasing ${current} → ${version} (tag ${tag})\n`)

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
