import { AsyncLocalStorage } from 'node:async_hooks'
import { Buffer } from 'node:buffer'
import { createHash } from 'node:crypto'
import { EventEmitter } from 'node:events'
import { readFile } from 'node:fs/promises'
import { hostname, platform } from 'node:os'
import { join } from 'node:path'
import process from 'node:process'
import { Readable } from 'node:stream'
import { setTimeout as delay } from 'node:timers/promises'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { inspect, types } from 'node:util'

async function runProbes() {
  const cwd = process.cwd()
  const packageJson = await readFile(join(cwd, 'package.json'), 'utf8')
  const parsedPackage: unknown = JSON.parse(packageJson)
  const pkgName =
    typeof parsedPackage === 'object' &&
    parsedPackage !== null &&
    'name' in parsedPackage &&
    typeof parsedPackage.name === 'string'
      ? parsedPackage.name
      : undefined

  const als = new AsyncLocalStorage<string>()
  const alsValue = als.run('e2e-store', () => als.getStore())

  const emitter = new EventEmitter()
  let emitted = false
  emitter.once('ping', () => {
    emitted = true
  })
  emitter.emit('ping')

  const readableEnded = await new Promise<boolean>((resolve, reject) => {
    const stream = Readable.from(['rari'])
    let data = ''
    stream.on('data', chunk => {
      data += String(chunk)
    })
    stream.on('end', () => {
      resolve(data === 'rari')
    })
    stream.on('error', reject)
  })

  await delay(1)

  return {
    process: {
      cwd: typeof cwd === 'string' && cwd.length > 0,
      envObject: typeof process.env === 'object',
      platform: typeof process.platform === 'string',
      versionsNode: typeof process.versions.node === 'string',
    },
    path: {
      join: join('a', 'b') === 'a/b' || join('a', 'b') === 'a\\b',
    },
    fs: {
      readPackageName: pkgName === '@test/app',
    },
    buffer: {
      fromUtf8: Buffer.from('rari').toString('utf8') === 'rari',
    },
    crypto: {
      sha256: createHash('sha256').update('rari').digest('hex').length === 64,
    },
    asyncHooks: {
      asyncLocalStorage: alsValue === 'e2e-store',
    },
    url: {
      fileURLToPath:
        fileURLToPath(pathToFileURL(join(cwd, 'package.json'))) === join(cwd, 'package.json'),
    },
    events: {
      emit: emitted,
    },
    os: {
      platform: typeof platform() === 'string',
      hostname: typeof hostname() === 'string' && hostname().length > 0,
    },
    stream: {
      readable: readableEnded,
    },
    timers: {
      promises: true,
    },
    util: {
      inspect: inspect({ ok: true }).includes('ok'),
      typesIsDate: types.isDate(new Date()),
    },
  }
}

export async function GET() {
  try {
    const probes = await runProbes()
    const flat = flatten(probes)
    const failed = Object.entries(flat)
      .filter(([, value]) => !value)
      .map(([name]) => name)

    return Response.json({
      ok: failed.length === 0,
      failed,
      probes,
    })
  } catch (error) {
    return Response.json(
      {
        ok: false,
        failed: ['probe'],
        error: error instanceof Error ? error.message : String(error),
      },
      { status: 500 },
    )
  }
}

function flatten(value: {
  readonly [key: string]: { readonly [key: string]: boolean }
}): Record<string, boolean> {
  const out: Record<string, boolean> = {}
  for (const [group, probes] of Object.entries(value)) {
    for (const [name, ok] of Object.entries(probes)) out[`${group}.${name}`] = ok
  }

  return out
}
