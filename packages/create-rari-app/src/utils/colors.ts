import process from 'node:process'

const isColorSupported
  = !(process.env.NO_COLOR || process.argv.includes('--no-color'))
    && (
      process.env.FORCE_COLOR
      || process.argv.includes('--color')
      || process.platform === 'win32'
      || (process.stdout?.isTTY && process.env.TERM !== 'dumb')
      || process.env.CI
    )

function formatter(open: string, close: string, replace = open) {
  return (input: string | number) => {
    const string = String(input)
    const index = string.indexOf(close, open.length)
    return ~index
      ? open + replaceClose(string, close, replace, index) + close
      : open + string + close
  }
}

function replaceClose(string: string, close: string, replace: string, index: number): string {
  let result = ''
  let cursor = 0
  do {
    result += string.substring(cursor, index) + replace
    cursor = index + close.length
    index = string.indexOf(close, cursor)
  } while (~index)
  return result + string.substring(cursor)
}

const f = isColorSupported ? formatter : () => String

const colors = {
  isColorSupported,
  black: f('\x1B[30m', '\x1B[39m'),
  red: f('\x1B[31m', '\x1B[39m'),
  green: f('\x1B[32m', '\x1B[39m'),
  cyan: f('\x1B[36m', '\x1B[39m'),
  gray: f('\x1B[90m', '\x1B[39m'),
  bgCyan: f('\x1B[46m', '\x1B[49m'),
}

export default colors
