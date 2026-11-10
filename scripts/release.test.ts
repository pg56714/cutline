import { describe, expect, it } from 'bun:test'
import { crateVersion, nextVersion, notes, parseCommits } from './release'

const log = (...rows: [string, string, string?][]) => rows.map(([sha, s, b = '']) => `${sha}\x1f${s}\x1f${b}\x1e`).join('\n')

describe('release', () => {
  it('reads Conventional Commits and skips release commits', () => {
    const c = parseCommits(log(['a1', 'feat(ui): tabs'], ['b2', 'fix: crash'], ['c3', 'chore(release): v0.1.0'], ['d4', 'not conventional'], ['e5', 'refactor!: new config']))
    expect(c.map((x) => [x.type, x.scope, x.breaking])).toEqual([
      ['feat', 'ui', false],
      ['fix', '', false],
      ['refactor', '', true],
    ])
    expect(parseCommits(log(['f6', 'fix: x', 'BREAKING CHANGE: y']))[0].breaking).toBe(true)
  })

  it('bumps by the biggest change', () => {
    const c = (type: string, breaking = false) => ({ sha: 'x', type, scope: '', subject: 's', breaking })
    expect(nextVersion('0.1.0', [c('fix')])).toBe('0.1.1')
    expect(nextVersion('0.1.3', [c('fix'), c('feat')])).toBe('0.2.0')
    expect(nextVersion('0.4.2', [c('fix', true)])).toBe('0.5.0')
    expect(nextVersion('1.4.2', [c('fix', true)])).toBe('2.0.0')
    expect(nextVersion('0.1.0', [c('docs'), c('chore')])).toBeNull()
  })

  it('groups notes by kind', () => {
    const text = notes('0.2.0', '2026-11-09', parseCommits(log(['abcdef123', 'feat(ui): tabs'], ['bcdef1234', 'docs: readme'])), 'https://github.com/o/r')
    expect(text).toContain('## v0.2.0 (2026-11-09)')
    expect(text).toContain('### Features\n\n- **ui:** tabs ([abcdef1](https://github.com/o/r/commit/abcdef123))')
    expect(text).toContain('### Other')
  })

  it('reads the crate version, not a dependency version', () => {
    const toml = '[package]\nname = "cutline"\nversion = "0.3.1"\n\n[dependencies]\nfoo = { version = "9.9.9" }\n'
    expect(crateVersion(toml)).toBe('0.3.1')
  })
})
