// Works out the next version from Conventional Commits since the last tag, prepends the
// release notes to CHANGELOG.md and bumps the version everywhere it is written.
//
//   bun scripts/release.ts            write files (CI)
//   bun scripts/release.ts --dry-run  only print what would happen
//
// In GitHub Actions it also sets outputs: skip, version, tag (and writes release-notes.md).

import { execFileSync } from 'node:child_process'
import { appendFileSync, existsSync, readFileSync, writeFileSync } from 'node:fs'

const dry = process.argv.includes('--dry-run')
const git = (...args: string[]) => execFileSync('git', args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim()

export interface Change {
  sha: string
  type: string
  scope: string
  subject: string
  breaking: boolean
}

export function parseCommits(log: string): Change[] {
  return log
    .split('\x1e')
    .map((r) => r.trim())
    .filter(Boolean)
    .flatMap((r) => {
      const [sha, subject, body = ''] = r.split('\x1f')
      const m = subject.match(/^(\w+)(?:\(([^)]*)\))?(!)?:\s*(.+)$/)
      if (!m) return []
      return [{ sha, type: m[1].toLowerCase(), scope: m[2] ?? '', subject: m[4], breaking: !!m[3] || /BREAKING[ -]CHANGE/.test(body) }]
    })
    .filter((c) => !(c.type === 'chore' && c.scope === 'release'))
}

/** Next version, or null when nothing worth releasing changed. Before 1.0 a breaking change bumps the minor. */
export function nextVersion(current: string, changes: Change[]): string | null {
  const [major, minor, patch] = current.split('.').map(Number)
  if (changes.some((c) => c.breaking)) return major >= 1 ? `${major + 1}.0.0` : `${major}.${minor + 1}.0`
  if (changes.some((c) => c.type === 'feat')) return `${major}.${minor + 1}.0`
  if (changes.some((c) => ['fix', 'perf'].includes(c.type))) return `${major}.${minor}.${patch + 1}`
  return null
}

const SECTIONS: [string, (c: Change) => boolean][] = [
  ['Breaking changes', (c) => c.breaking],
  ['Features', (c) => c.type === 'feat' && !c.breaking],
  ['Fixes', (c) => c.type === 'fix' && !c.breaking],
  ['Performance', (c) => c.type === 'perf' && !c.breaking],
  ['Other', (c) => !['feat', 'fix', 'perf'].includes(c.type) && !c.breaking],
]

export function notes(version: string, date: string, changes: Change[], repoUrl: string): string {
  const out = [`## v${version} (${date})`, '']
  for (const [title, pick] of SECTIONS) {
    const items = changes.filter(pick)
    if (!items.length) continue
    out.push(`### ${title}`, '')
    for (const c of items) out.push(`- ${c.scope ? `**${c.scope}:** ` : ''}${c.subject} ([${c.sha.slice(0, 7)}](${repoUrl}/commit/${c.sha}))`)
    out.push('')
  }
  return out.join('\n')
}

/** The crate version: the first `version = "x.y.z"` line, which sits under [package]. */
export function crateVersion(cargoToml: string): string {
  const m = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)
  if (!m) throw new Error('no version found in Cargo.toml')
  return m[1]
}

function main() {
  let lastTag = ''
  try {
    lastTag = git('describe', '--tags', '--abbrev=0', '--match', 'v*')
  } catch {
    /* first release */
  }
  const range = lastTag ? [`${lastTag}..HEAD`] : []
  const changes = parseCommits(git('log', ...range, '--format=%H%x1f%s%x1f%b%x1e'))
  const cargo = readFileSync('Cargo.toml', 'utf8')
  // The first release ships the version already in Cargo.toml.
  const version = lastTag ? nextVersion(lastTag.replace(/^v/, ''), changes) : crateVersion(cargo)
  const out = (k: string, v: string) => process.env.GITHUB_OUTPUT && appendFileSync(process.env.GITHUB_OUTPUT, `${k}=${v}\n`)

  if (!version) {
    console.log(`Nothing to release since ${lastTag}: no feat, fix, perf or breaking commits.`)
    out('skip', 'true')
    return
  }
  const repoUrl = (process.env.GITHUB_SERVER_URL && process.env.GITHUB_REPOSITORY ? `${process.env.GITHUB_SERVER_URL}/${process.env.GITHUB_REPOSITORY}` : git('remote', 'get-url', 'origin'))
    .replace(/\.git$/, '')
    .replace(/^git@github\.com:/, 'https://github.com/')
  const text = notes(version, new Date().toISOString().slice(0, 10), changes, repoUrl)
  console.log(`${lastTag || '(no tag yet)'} -> v${version}\n\n${text}`)
  if (dry) return

  const head = '# Changelog\n\n'
  const old = existsSync('CHANGELOG.md') ? readFileSync('CHANGELOG.md', 'utf8').replace(head, '') : ''
  writeFileSync('CHANGELOG.md', head + text + '\n' + old)
  writeFileSync('release-notes.md', text)

  // Only the first `version = ` line belongs to the crate; dependencies come later.
  writeFileSync('Cargo.toml', cargo.replace(/^version\s*=\s*"[^"]+"/m, `version = "${version}"`))
  // Cargo.lock carries the crate's own version too.
  const lock = readFileSync('Cargo.lock', 'utf8').replace(/(name = "cutline"\r?\nversion = )"[^"]+"/, `$1"${version}"`)
  writeFileSync('Cargo.lock', lock)

  out('skip', 'false')
  out('version', version)
  out('tag', `v${version}`)
}

if (import.meta.main) main()
