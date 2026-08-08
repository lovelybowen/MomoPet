import { existsSync, statSync } from 'node:fs'
import { resolve } from 'node:path'
import { argv, env, exit } from 'node:process'

const corePath = resolve('public/vendor/live2d/live2dcubismcore.min.js')
const releaseCheck = argv.includes('--release')

if (!existsSync(corePath) || statSync(corePath).size < 100_000) {
  console.error([
    'Live2D Cubism Core is not installed.',
    'Copy live2dcubismcore.min.js from an official Cubism SDK into:',
    `  ${corePath}`,
  ].join('\n'))
  exit(1)
}

if (releaseCheck && env.MOMOPET_LIVE2D_LICENSE_ACKNOWLEDGED !== '1') {
  console.error('Set MOMOPET_LIVE2D_LICENSE_ACKNOWLEDGED=1 after completing the Live2D licensing review.')
  exit(1)
}

console.log(`Cubism Core installation found: ${corePath}`)
