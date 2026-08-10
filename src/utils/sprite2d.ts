import 'pixi.js/unsafe-eval'
import { readFile, readTextFile } from '@tauri-apps/plugin-fs'
import { error as logError, info as logInfo } from '@tauri-apps/plugin-log'
import {
  AnimatedSprite,
  Application,
  Container,
  Rectangle,
  Texture,
  Ticker,
} from 'pixi.js'

import type { ModelSize } from '@/composables/useModel'
import type { InstalledModel, PetAction } from '@/stores/model'

import { join } from './path'

interface SpriteClip {
  sheet: string
  frames: number[]
  fps: number
  loop: boolean
}

interface SpriteConfig {
  frameSize: ModelSize
  anchor?: { x: number, y: number }
  sheets: Record<string, string>
  clips: Record<string, SpriteClip>
  interactions?: Record<'tap', string>
}

interface Transition {
  from: AnimatedSprite
  to: AnimatedSprite
  elapsed: number
}

const TRANSITION_MS = 120
const LOAD_TIMEOUT_MS = 15_000

export async function withLoadTimeout<T>(
  operation: Promise<T>,
  stage: string,
  timeoutMs = LOAD_TIMEOUT_MS,
) {
  let timeout: ReturnType<typeof setTimeout> | undefined

  try {
    return await Promise.race([
      operation,
      new Promise<never>((_resolve, reject) => {
        timeout = setTimeout(
          () => reject(new Error(`Timed out during ${stage}`)),
          timeoutMs,
        )
      }),
    ])
  } finally {
    if (timeout) clearTimeout(timeout)
  }
}

function describeError(error: unknown) {
  return error instanceof Error ? error.message : String(error)
}

function writeRuntimeLog(level: 'error' | 'info', message: string) {
  const write = level === 'error' ? logError : logInfo
  void write(`[sprite2d] ${message}`).catch(() => {})
}

export function resolveActionClip(activeClip: string, action: PetAction) {
  return action.mode === 'toggle' && activeClip === action.clip
    ? 'idle'
    : action.clip
}

class Sprite2d {
  private app: Application | null = null
  private layoutRoot: Container | null = null
  private motionRoot: Container | null = null
  private sprite: AnimatedSprite | null = null
  private config: SpriteConfig | null = null
  private actions = new Map<string, PetAction>()
  private clipTextures = new Map<string, Texture[]>()
  private sheetTextures: Texture[] = []
  private sheetBitmaps: ImageBitmap[] = []
  private transition: Transition | null = null
  private activeClip = ''
  private elapsed = 0
  private pointerTarget = 0
  private pointerCurrent = 0

  private readonly updateMotion = (ticker: Ticker) => {
    if (!this.motionRoot || !this.config) return

    this.elapsed += ticker.deltaMS
    this.pointerCurrent += (this.pointerTarget - this.pointerCurrent)
      * (1 - 0.82 ** (ticker.deltaMS / (1000 / 60)))

    const breath = this.activeClip === 'sleep'
      ? Math.sin(this.elapsed / 950)
      : 0

    this.motionRoot.rotation = this.pointerCurrent * 0.018 + breath * 0.003
    this.motionRoot.scale.set(1 - breath * 0.004, 1 + breath * 0.012)
    this.motionRoot.y = -Math.max(0, breath) * this.config.frameSize.height * 0.006

    if (!this.transition) return

    this.transition.elapsed += ticker.deltaMS
    const progress = Math.min(1, this.transition.elapsed / TRANSITION_MS)
    this.transition.from.alpha = 1 - progress
    this.transition.to.alpha = progress

    if (progress === 1) {
      this.transition.from.removeFromParent()
      this.transition.from.destroy({ texture: false })
      this.transition = null
    }
  }

  private async initApp() {
    if (this.app) return

    const view = document.getElementById('petCanvas')
    if (!(view instanceof HTMLCanvasElement)) {
      throw new TypeError('Sprite2D canvas is not mounted')
    }

    const app = new Application()

    try {
      await app.init({
        view,
        resizeTo: window,
        backgroundAlpha: 0,
        autoDensity: true,
        resolution: devicePixelRatio,
        sharedTicker: true,
        preference: ['webgl', 'canvas'],
      })
      this.app = app
    } catch (error) {
      try {
        app.destroy()
      } catch {
        // Pixi may not have created a renderer yet.
      }
      throw error
    }
  }

  async load(model: InstalledModel) {
    writeRuntimeLog('info', `loading ${model.id} from ${model.entryPath}`)

    try {
      await withLoadTimeout(this.initApp(), 'renderer initialization')
      this.destroy()

      const configText = await withLoadTimeout(
        readTextFile(model.entryPath),
        'sprite configuration read',
      )
      const config = JSON.parse(configText) as SpriteConfig
      const anchor = config.anchor ?? { x: 0.5, y: 1 }
      const sheetTextures = new Map<string, Texture>()

      this.config = config
      this.actions = new Map(model.actions.map(action => [action.id, action]))
      this.layoutRoot = new Container()
      this.motionRoot = new Container()
      this.layoutRoot.addChild(this.motionRoot)
      this.app?.stage.addChild(this.layoutRoot)

      for (const [sheetId, path] of Object.entries(config.sheets)) {
        const sheetPath = join(model.path, path)
        const bytes = await withLoadTimeout(
          readFile(sheetPath),
          `sprite sheet read (${sheetId})`,
        )
        const bitmap = await withLoadTimeout(
          createImageBitmap(new Blob([bytes], { type: 'image/png' })),
          `sprite sheet decode (${sheetId})`,
        )
        const texture = Texture.from(bitmap, true)

        this.sheetBitmaps.push(bitmap)
        this.sheetTextures.push(texture)
        sheetTextures.set(sheetId, texture)
      }

      for (const [clipId, clip] of Object.entries(config.clips)) {
        const sheet = sheetTextures.get(clip.sheet)
        if (!sheet) throw new Error(`Missing sprite sheet: ${clip.sheet}`)

        const columns = Math.floor(sheet.width / config.frameSize.width)
        const textures = clip.frames.map((frame) => {
          const x = (frame % columns) * config.frameSize.width
          const y = Math.floor(frame / columns) * config.frameSize.height

          return new Texture({
            source: sheet.source,
            frame: new Rectangle(x, y, config.frameSize.width, config.frameSize.height),
            defaultAnchor: anchor,
          })
        })

        this.clipTextures.set(clipId, textures)
      }

      this.playClip('idle')
      Ticker.shared.add(this.updateMotion)
      writeRuntimeLog('info', `loaded ${model.id}`)

      return { ...config.frameSize }
    } catch (error) {
      this.destroy()
      const message = `failed to load ${model.id}: ${describeError(error)}`
      writeRuntimeLog('error', message)
      throw new Error(`Sprite2D ${message}`)
    }
  }

  destroy() {
    Ticker.shared.remove(this.updateMotion)
    this.transition = null
    this.sprite = null
    this.layoutRoot?.removeFromParent()
    this.layoutRoot?.destroy({ children: true, texture: false })
    this.layoutRoot = null
    this.motionRoot = null
    this.config = null
    this.actions.clear()
    this.activeClip = ''
    this.elapsed = 0
    this.pointerTarget = 0
    this.pointerCurrent = 0

    for (const textures of this.clipTextures.values()) {
      textures.forEach(texture => texture.destroy(false))
    }
    this.clipTextures.clear()

    this.sheetTextures.forEach(texture => texture.destroy(true))
    this.sheetTextures = []

    this.sheetBitmaps.forEach(bitmap => bitmap.close())
    this.sheetBitmaps = []
  }

  resizeModel(modelSize: ModelSize) {
    if (!this.layoutRoot) return

    const scale = Math.min(innerWidth / modelSize.width, innerHeight / modelSize.height)

    this.layoutRoot.scale.set(scale)
    this.layoutRoot.x = innerWidth / 2
    this.layoutRoot.y = innerHeight
  }

  startAction(action: PetAction) {
    const clip = resolveActionClip(this.activeClip, action)
    this.playClip(clip, clip === 'idle' ? undefined : action.mode)
  }

  tap() {
    if (!this.config) return

    const actionId = this.config.interactions?.tap
    const action = actionId ? this.actions.get(actionId) : undefined
    if (action) {
      this.startAction(action)
    } else if (this.activeClip !== 'idle' && this.sprite?.loop) {
      this.playClip('idle')
    }
  }

  setPointer(xRatio: number) {
    this.pointerTarget = Math.max(-1, Math.min(1, xRatio * 2 - 1))
  }

  setMaxFPS(fps: number) {
    Ticker.shared.maxFPS = fps
  }

  private playClip(clipId: string, mode?: PetAction['mode']) {
    const clip = this.config?.clips[clipId]
    const textures = this.clipTextures.get(clipId)
    if (!clip || !textures || !this.motionRoot) return

    this.transition?.from.destroy({ texture: false })
    this.transition = null

    const next = new AnimatedSprite({
      textures,
      animationSpeed: clip.fps / 60,
      autoPlay: true,
      loop: mode === 'once' ? false : clip.loop,
    })
    next.alpha = this.sprite ? 0 : 1

    if (!next.loop) {
      next.onComplete = () => this.playClip('idle')
    }

    const previous = this.sprite
    if (previous) {
      previous.stop()
      previous.onComplete = undefined
      this.transition = { from: previous, to: next, elapsed: 0 }
    }

    this.motionRoot.addChild(next)
    this.sprite = next
    this.activeClip = clipId
    this.elapsed = 0
  }
}

const sprite2d = new Sprite2d()

export default sprite2d
