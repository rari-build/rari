/// <reference path="../types.d.ts" />

import { defineDenoLazyProps, lazyExtScript } from 'ext:init_utilities/utilities.ts'

interface FsModule {
  writeFileSync: typeof Deno.writeFileSync
  writeFile: typeof Deno.writeFile
  writeTextFileSync: typeof Deno.writeTextFileSync
  writeTextFile: typeof Deno.writeTextFile
  readTextFile: typeof Deno.readTextFile
  readTextFileSync: typeof Deno.readTextFileSync
  readFile: typeof Deno.readFile
  readFileSync: typeof Deno.readFileSync
  chmodSync: typeof Deno.chmodSync
  chmod: typeof Deno.chmod
  chown: typeof Deno.chown
  chownSync: typeof Deno.chownSync
  copyFileSync: typeof Deno.copyFileSync
  cwd: typeof Deno.cwd
  makeTempDirSync: typeof Deno.makeTempDirSync
  makeTempDir: typeof Deno.makeTempDir
  makeTempFileSync: typeof Deno.makeTempFileSync
  makeTempFile: typeof Deno.makeTempFile
  mkdirSync: typeof Deno.mkdirSync
  mkdir: typeof Deno.mkdir
  chdir: typeof Deno.chdir
  copyFile: typeof Deno.copyFile
  readDirSync: typeof Deno.readDirSync
  readDir: typeof Deno.readDir
  readLinkSync: typeof Deno.readLinkSync
  readLink: typeof Deno.readLink
  realPathSync: typeof Deno.realPathSync
  realPath: typeof Deno.realPath
  removeSync: typeof Deno.removeSync
  remove: typeof Deno.remove
  renameSync: typeof Deno.renameSync
  rename: typeof Deno.rename
  statSync: typeof Deno.statSync
  lstatSync: typeof Deno.lstatSync
  stat: typeof Deno.stat
  lstat: typeof Deno.lstat
  truncateSync: typeof Deno.truncateSync
  truncate: typeof Deno.truncate
  FsFile: typeof Deno.FsFile
  open: typeof Deno.open
  openSync: typeof Deno.openSync
  create: typeof Deno.create
  createSync: typeof Deno.createSync
  symlink: typeof Deno.symlink
  symlinkSync: typeof Deno.symlinkSync
  link: typeof Deno.link
  linkSync: typeof Deno.linkSync
  utime: typeof Deno.utime
  utimeSync: typeof Deno.utimeSync
  umask: typeof Deno.umask
}

const lazyFs = lazyExtScript<FsModule>('ext:deno_fs/30_fs.js')

defineDenoLazyProps(lazyFs, [
  'writeFileSync',
  'writeFile',
  'writeTextFileSync',
  'writeTextFile',
  'readTextFile',
  'readTextFileSync',
  'readFile',
  'readFileSync',
  'chmodSync',
  'chmod',
  'chown',
  'chownSync',
  'copyFileSync',
  'cwd',
  'makeTempDirSync',
  'makeTempDir',
  'makeTempFileSync',
  'makeTempFile',
  'mkdirSync',
  'mkdir',
  'chdir',
  'copyFile',
  'readDirSync',
  'readDir',
  'readLinkSync',
  'readLink',
  'realPathSync',
  'realPath',
  'removeSync',
  'remove',
  'renameSync',
  'rename',
  'statSync',
  'lstatSync',
  'stat',
  'lstat',
  'truncateSync',
  'truncate',
  'FsFile',
  'open',
  'openSync',
  'create',
  'createSync',
  'symlink',
  'symlinkSync',
  'link',
  'linkSync',
  'utime',
  'utimeSync',
  'umask',
])
