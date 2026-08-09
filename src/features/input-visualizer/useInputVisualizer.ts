import type { PhysicalPosition } from '@tauri-apps/api/dpi'
import type { LiteralUnion } from 'type-fest'

import { invoke } from '@tauri-apps/api/core'
import { PhysicalPosition as TauriPhysicalPosition } from '@tauri-apps/api/dpi'
import { sep } from '@tauri-apps/api/path'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { readDir } from '@tauri-apps/plugin-fs'
import { isNil } from 'es-toolkit'
import { findKey, nth } from 'es-toolkit/compat'
import { Ticker } from 'pixi.js'
import { computed, onMounted, onUnmounted, reactive, ref, watch } from 'vue'

import { useTauriListen } from '@/composables/useTauriListen'
import { INVOKE_KEY, LISTEN_KEY, WINDOW_LABEL } from '@/constants'
import { useAppStore } from '@/stores/app'
import { useModelStore } from '@/stores/model'
import { usePetStore } from '@/stores/pet'
import { inBetween } from '@/utils/is'
import live2d from '@/utils/live2d'
import { getCursorMonitor } from '@/utils/monitor'
import { join } from '@/utils/path'
import { isMac, isWindows } from '@/utils/platform'
import { clearObject } from '@/utils/shared'

import {
  DEFAULT_INPUT_VISUALIZER_PROFILE,
  mapAxisParameter,
  mapPointerParameter,
} from './profile'

interface MouseButtonEvent {
  kind: 'MousePress' | 'MouseRelease'
  value: string
}

interface CursorPoint {
  x: number
  y: number
}

interface MouseMoveEvent {
  kind: 'MouseMove'
  value: CursorPoint
}

interface KeyboardEvent {
  kind: 'KeyboardPress' | 'KeyboardRelease'
  value: string
}

type DeviceEvent = MouseButtonEvent | MouseMoveEvent | KeyboardEvent
type GamepadEventName = LiteralUnion<
  'LeftStickX' | 'LeftStickY' | 'RightStickX' | 'RightStickY' | 'LeftThumb' | 'RightThumb',
  string
>

interface GamepadEvent {
  kind: 'ButtonChanged' | 'AxisChanged'
  name: GamepadEventName
  value: number
}

interface StickState {
  x: number
  y: number
  pressed: boolean
}

const DAMPING_DECAY = 0.75
const appWindow = getCurrentWebviewWindow()

export function useInputVisualizer() {
  const appStore = useAppStore()
  const modelStore = useModelStore()
  const petStore = usePetStore()
  const profile = computed(() => {
    if (modelStore.currentModel?.input) return modelStore.currentModel.input.parameters
    if (modelStore.currentModel?.isLegacy) return DEFAULT_INPUT_VISUALIZER_PROFILE
  })
  const inputEnabled = computed(() => {
    return petStore.model.inputVisualizer && Boolean(profile.value)
  })
  const hideOnHoverEnabled = computed(() => {
    return petStore.window.hideOnHover && Boolean(modelStore.currentModel)
  })
  const deviceListeningEnabled = computed(() => {
    return inputEnabled.value || hideOnHoverEnabled.value
  })
  const cursorTrackingEnabled = computed(() => {
    return hideOnHoverEnabled.value
      || (inputEnabled.value && !petStore.model.ignoreMouse)
  })
  const releaseTimers = new Map<string, ReturnType<typeof setTimeout>>()
  const latestCursorPoint = ref<CursorPoint>()
  const smoothedCursorPoint = ref<CursorPoint>()
  const scaleFactor = ref(1)
  const sticks = reactive<Record<'left' | 'right', StickState>>({
    left: { x: 0, y: 0, pressed: false },
    right: { x: 0, y: 0, pressed: false },
  })
  const stickActive = computed(() => ({
    left: sticks.left.x !== 0 || sticks.left.y !== 0 || sticks.left.pressed,
    right: sticks.right.x !== 0 || sticks.right.y !== 0 || sticks.right.pressed,
  }))

  const handlePress = (key: string) => {
    const path = modelStore.supportKeys[key]

    if (!path) return

    const directory = nth(path.split(sep()), -2)
    const previousKey = findKey(modelStore.pressedKeys, value => value.includes(directory ?? ''))

    if (previousKey) handleRelease(previousKey)
    modelStore.pressedKeys[key] = path
  }

  const handleRelease = (key: string) => {
    delete modelStore.pressedKeys[key]
  }

  const getSupportedKey = (key: string) => {
    let nextKey = key
    const unsupportedKey = !modelStore.supportKeys[nextKey]

    if (key.startsWith('F') && unsupportedKey) {
      nextKey = key.replace(/F(\d+)/, 'Fn')
    }

    for (const modifier of ['Meta', 'Shift', 'Alt', 'Control']) {
      if (key.startsWith(modifier) && unsupportedKey) {
        nextKey = key.replace(new RegExp(`^(${modifier}).*`), '$1')
      }
    }

    return nextKey
  }

  const handleAutoRelease = (key: string, delay = 100) => {
    handlePress(key)

    const existingTimer = releaseTimers.get(key)
    if (existingTimer) clearTimeout(existingTimer)

    releaseTimers.set(key, setTimeout(() => {
      handleRelease(key)
      releaseTimers.delete(key)
    }, delay))
  }

  const updatePointerParameters = async (cursorPoint: PhysicalPosition) => {
    const monitor = await getCursorMonitor(cursorPoint)

    if (!monitor) return

    const xRatio = (cursorPoint.x - monitor.position.x) / monitor.size.width
    const yRatio = (cursorPoint.y - monitor.position.y) / monitor.size.height

    for (const parameterId of profile.value?.pointer ?? []) {
      const range = live2d.getParameterValueRange(parameterId)

      if (!range || isNil(range.min) || isNil(range.max)) continue

      const value = mapPointerParameter(
        parameterId,
        { min: range.min, max: range.max },
        xRatio,
        yRatio,
        petStore.model.mouseMirror,
      )

      live2d.setParameterValue(parameterId, value)
    }
  }

  let hideOnHoverTimer: ReturnType<typeof setTimeout> | undefined
  let wasInWindow = false

  const resetHideOnHover = () => {
    if (hideOnHoverTimer) clearTimeout(hideOnHoverTimer)
    hideOnHoverTimer = undefined
    wasInWindow = false
    document.body.style.removeProperty('opacity')
    void appWindow.setIgnoreCursorEvents(petStore.window.passThrough)
  }

  const onHideOnHover = (x: number, y: number) => {
    const { x: winX, y: winY, width, height } = appStore.windowState[WINDOW_LABEL.MAIN] ?? {}

    if (isNil(winX) || isNil(winY) || isNil(width) || isNil(height)) return

    const isInWindow = inBetween(x, winX, winX + width)
      && inBetween(y, winY, winY + height)

    if (isInWindow === wasInWindow) return
    if (hideOnHoverTimer) clearTimeout(hideOnHoverTimer)

    if (isInWindow) {
      hideOnHoverTimer = setTimeout(() => {
        document.body.style.setProperty('opacity', '0')
        void appWindow.setIgnoreCursorEvents(true)
      }, petStore.window.hideOnHoverDelay * 1000)
    } else {
      document.body.style.removeProperty('opacity')
      void appWindow.setIgnoreCursorEvents(petStore.window.passThrough)
    }

    wasInWindow = isInWindow
  }

  const handleCursorMove = async (cursorPoint: CursorPoint) => {
    const x = cursorPoint.x * scaleFactor.value
    const y = cursorPoint.y * scaleFactor.value

    if (inputEnabled.value) {
      await updatePointerParameters(new TauriPhysicalPosition(x, y))
    }

    if (hideOnHoverEnabled.value) onHideOnHover(x, y)
  }

  const tickerCallback = (ticker: Ticker) => {
    const destination = latestCursorPoint.value

    if (!destination) return

    const current = smoothedCursorPoint.value ?? destination
    const alpha = 1 - DAMPING_DECAY ** (ticker.deltaMS / (1000 / 60))
    const interpolated = {
      x: current.x + (destination.x - current.x) * alpha,
      y: current.y + (destination.y - current.y) * alpha,
    }

    if (Math.hypot(destination.x - interpolated.x, destination.y - interpolated.y) < 0.5) {
      smoothedCursorPoint.value = { ...destination }
      latestCursorPoint.value = undefined
    } else {
      smoothedCursorPoint.value = interpolated
    }

    void handleCursorMove(smoothedCursorPoint.value)
  }

  watch(
    [deviceListeningEnabled, inputEnabled, () => modelStore.currentModel?.mode],
    async ([isListeningEnabled, isInputEnabled, mode]) => {
      if (isListeningEnabled) {
        void invoke(INVOKE_KEY.START_DEVICE_LISTENING)
      } else {
        await invoke(INVOKE_KEY.STOP_DEVICE_LISTENING)
      }

      if (isInputEnabled && mode === 'gamepad') {
        void invoke(INVOKE_KEY.START_GAMEPAD_LISTENING)
      } else {
        await invoke(INVOKE_KEY.STOP_GAMEPAD_LISTENING)
      }

      if (!isInputEnabled) clearObject([modelStore.supportKeys, modelStore.pressedKeys])
    },
    { immediate: true },
  )

  watch(
    [() => modelStore.currentModel, inputEnabled],
    async ([model, isEnabled]) => {
      clearObject([modelStore.supportKeys, modelStore.pressedKeys])

      if (!model || !isEnabled) return

      for (const groupName of ['left-keys', 'right-keys']) {
        const groupDirectory = join(model.resourcePath, groupName)
        const files = await readDir(groupDirectory).catch(() => [])

        for (const file of files.filter(file => file.name.toLowerCase().endsWith('.png'))) {
          modelStore.supportKeys[file.name.split('.')[0]] = join(groupDirectory, file.name)
        }
      }
    },
    { immediate: true },
  )

  watch([modelStore.pressedKeys, stickActive], ([keys, active]) => {
    const currentProfile = profile.value

    if (!inputEnabled.value || !currentProfile) return

    const directories = Object.values(keys).map(path => nth(path.split(sep()), -2) ?? '')
    const leftPressed = directories.some(directory => directory.startsWith('left'))
    const rightPressed = directories.some(directory => directory.startsWith('right'))

    if (currentProfile.hands) {
      live2d.setParameterValue(currentProfile.hands.left, active.left || leftPressed)
      live2d.setParameterValue(currentProfile.hands.right, active.right || rightPressed)
    }

    if (currentProfile.gamepad?.stickHands) {
      live2d.setParameterValue(currentProfile.gamepad.stickHands.left, active.left)
      live2d.setParameterValue(currentProfile.gamepad.stickHands.right, active.right)
    }
  }, { deep: true })

  watch(cursorTrackingEnabled, (isEnabled) => {
    Ticker.shared.remove(tickerCallback)

    if (isEnabled) Ticker.shared.add(tickerCallback)
  }, { immediate: true })

  watch(hideOnHoverEnabled, (isEnabled) => {
    if (!isEnabled) resetHideOnHover()
  }, { immediate: true })

  useTauriListen<DeviceEvent>(LISTEN_KEY.DEVICE_CHANGED, ({ payload }) => {
    if (!deviceListeningEnabled.value) return

    const { kind, value } = payload

    if (kind === 'MouseMove') {
      if (cursorTrackingEnabled.value) latestCursorPoint.value = value
      return
    }

    if (!inputEnabled.value) return

    if (kind === 'KeyboardPress' || kind === 'KeyboardRelease') {
      const key = getSupportedKey(value)

      if (key === 'CapsLock') return handleAutoRelease(key)

      if (kind === 'KeyboardPress') {
        return isWindows
          ? handleAutoRelease(key, petStore.model.autoReleaseDelay * 1000)
          : handlePress(key)
      }

      return handleRelease(key)
    }

    const parameterId = profile.value?.mouseButtons?.[value]
    if (parameterId) live2d.setParameterValue(parameterId, kind === 'MousePress')
  })

  useTauriListen<GamepadEvent>(LISTEN_KEY.GAMEPAD_CHANGED, ({ payload }) => {
    if (!inputEnabled.value) return

    const { name, value } = payload
    const axisParameter = profile.value?.gamepad?.axes?.[name]

    if (axisParameter) {
      const side = name.startsWith('Left') ? sticks.left : sticks.right
      if (name.endsWith('X')) side.x = value
      if (name.endsWith('Y')) side.y = value

      const range = live2d.getParameterValueRange(axisParameter)
      if (range && !isNil(range.min) && !isNil(range.max)) {
        live2d.setParameterValue(
          axisParameter,
          mapAxisParameter({ min: range.min, max: range.max }, value),
        )
      }
      return
    }

    const thumbParameter = profile.value?.gamepad?.thumbButtons?.[name]
    if (thumbParameter) {
      const side = name.startsWith('Left') ? sticks.left : sticks.right
      side.pressed = value !== 0
      live2d.setParameterValue(thumbParameter, side.pressed)
      return
    }

    value > 0 ? handlePress(name) : handleRelease(name)
  })

  onMounted(async () => {
    scaleFactor.value = isMac ? await appWindow.scaleFactor() : 1

    appWindow.onScaleChanged(({ payload }) => {
      if (isMac) scaleFactor.value = payload.scaleFactor
    })
  })

  onUnmounted(() => {
    Ticker.shared.remove(tickerCallback)
    void invoke(INVOKE_KEY.STOP_DEVICE_LISTENING)
    void invoke(INVOKE_KEY.STOP_GAMEPAD_LISTENING)
    releaseTimers.forEach(timer => clearTimeout(timer))
    resetHideOnHover()
  })
}
