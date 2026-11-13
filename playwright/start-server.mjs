import { spawn } from 'node:child_process'
import { existsSync, mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { DatabaseSync } from 'node:sqlite'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const rootDir = path.resolve(__dirname, '..')
const dataDir = mkdtempSync(path.join(tmpdir(), 'channelzero-e2e-'))
const staticDir = path.join(rootDir, 'frontend', 'dist')
const releaseBin = path.join(rootDir, 'target', 'release', 'channelzero')

const env = {
  ...process.env,
  PORT: '8080',
  DATA_DIR: dataDir,
  STATIC_DIR: staticDir,
  TVMAZE_ENABLED: 'false',
  TMDB_ENABLED: 'false',
  OMDB_ENABLED: 'false',
  WATCHMODE_ENABLED: 'false',
}

const useReleaseBin = existsSync(releaseBin)
const cmd = useReleaseBin ? releaseBin : 'cargo'
const args = useReleaseBin ? [] : ['run', '--manifest-path', path.join(rootDir, 'Cargo.toml')]

const child = spawn(cmd, args, {
  cwd: rootDir,
  env,
  stdio: 'inherit',
})

function daysAgoIso(days) {
  const d = new Date(Date.now() - days * 86_400_000)
  return d.toISOString().slice(0, 10)
}

async function seedDatabaseWhenReady() {
  const dbPath = path.join(dataDir, 'channelzero.db')
  for (let i = 0; i < 120; i++) {
    try {
      const res = await fetch('http://127.0.0.1:8080/api/health')
      if (res.ok && existsSync(dbPath)) {
        const db = new DatabaseSync(dbPath)
        const now = new Date().toISOString()
        const insert = db.prepare(`
          INSERT OR IGNORE INTO programs (
            id, dedup_key, title, media_type, release_kind, release_date,
            release_year, episode_info, synopsis, poster_url, backdrop_url,
            genres_json, providers_json, primary_url, rating, ratings_json,
            sources_json, imdb_id, tmdb_id, tvmaze_id, updated_at
          ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        `)

        const items = [
          [
            'prog-neon',
            'neon horizon|2026|movie',
            'Neon Horizon',
            'movie',
            'new_title',
            daysAgoIso(2),
            2026,
            null,
            'A cyberpunk courier races across a flooded megacity.',
            null,
            null,
            JSON.stringify(['Sci-Fi', 'Thriller']),
            JSON.stringify([
              {
                id: 'netflix',
                name: 'Netflix',
                url: 'https://netflix.com/title/1',
                is_direct: true,
              },
            ]),
            'https://netflix.com/title/1',
            8.7,
            JSON.stringify([{ source: 'TMDB', score: '8.7/10', normalized_10: 8.7 }]),
            JSON.stringify(['TMDB']),
            'tt1000001',
            1001,
            null,
            now,
          ],
          [
            'prog-laugh',
            'laugh track|2026|series',
            'Laugh Track',
            'series',
            'new_episode',
            daysAgoIso(3),
            2026,
            'S02E04 · The Standup',
            'Behind the scenes of a late-night comedy club.',
            null,
            null,
            JSON.stringify(['Comedy']),
            JSON.stringify([
              { id: 'hulu', name: 'Hulu', url: 'https://hulu.com/series/2', is_direct: false },
            ]),
            'https://hulu.com/series/2',
            7.4,
            JSON.stringify([{ source: 'TVmaze', score: '7.4/10', normalized_10: 7.4 }]),
            JSON.stringify(['TVmaze']),
            'tt1000002',
            null,
            2002,
            now,
          ],
          [
            'prog-orbit',
            'deep orbit|2026|series',
            'Deep Orbit',
            'series',
            'new_title',
            daysAgoIso(1),
            2026,
            'S01E01 · Liftoff',
            'Crew aboard a distant research station uncover a signal.',
            null,
            null,
            JSON.stringify(['Sci-Fi', 'Drama']),
            JSON.stringify([
              { id: 'max', name: 'Max', url: 'https://max.com/show/3', is_direct: true },
            ]),
            'https://max.com/show/3',
            9.1,
            JSON.stringify([{ source: 'TMDB', score: '9.1/10', normalized_10: 9.1 }]),
            JSON.stringify(['TMDB']),
            'tt1000003',
            1003,
            null,
            now,
          ],
          [
            'prog-classic',
            'old classic|2026|movie',
            'Old Classic',
            'movie',
            'new_title',
            daysAgoIso(20),
            2026,
            null,
            'A quiet historical drama set in the highlands.',
            null,
            null,
            JSON.stringify(['Drama']),
            JSON.stringify([
              {
                id: 'prime',
                name: 'Prime Video',
                url: 'https://amazon.com/dp/4',
                is_direct: false,
              },
            ]),
            'https://amazon.com/dp/4',
            6.8,
            JSON.stringify([{ source: 'TMDB', score: '6.8/10', normalized_10: 6.8 }]),
            JSON.stringify(['TMDB']),
            'tt1000004',
            1004,
            null,
            now,
          ],
          [
            'prog-vintage',
            'vintage quest|2022|series',
            'Vintage Quest',
            'series',
            'new_title',
            daysAgoIso(120),
            2022,
            null,
            'An older fantasy adventure outside the 30-day window.',
            null,
            null,
            JSON.stringify(['Adventure']),
            JSON.stringify([
              {
                id: 'prime_video',
                name: 'Prime Video',
                url: 'https://amazon.com/dp/5',
                is_direct: false,
              },
            ]),
            'https://amazon.com/dp/5',
            8.1,
            JSON.stringify([{ source: 'TVmaze', score: '8.1/10', normalized_10: 8.1 }]),
            JSON.stringify(['TVmaze']),
            'tt1000005',
            null,
            1005,
            now,
          ],
        ]

        for (const row of items) {
          insert.run(...row)
        }
        db.close()
        return
      }
    } catch {
      // wait for server startup
    }
    await new Promise((r) => setTimeout(r, 250))
  }
}

seedDatabaseWhenReady()

function cleanup() {
  child.kill('SIGTERM')
  try {
    rmSync(dataDir, { recursive: true, force: true })
  } catch {
    // ignore cleanup errors
  }
}

process.on('SIGINT', () => {
  cleanup()
  process.exit(0)
})
process.on('SIGTERM', () => {
  cleanup()
  process.exit(0)
})

child.on('exit', (code) => {
  cleanup()
  process.exit(code ?? 0)
})
