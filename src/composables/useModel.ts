import { LogicalSize } from '@tauri-apps/api/dpi'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { message } from 'antdv-next'
import { round } from 'es-toolkit'
import { ref } from 'vue'

import { useModelStore } from '@/stores/model'
import { usePetStore } from '@/stores/pet'
import live2d from '@/utils/live2d'
import { isMac } from '@/utils/platform'

const appWindow = getCurrentWebviewWindow()
const digitKeys = '1234567890'.split('') as readonly string[]
const letterKeys = 'QWERTYUIOPASDFGHJKLZXCVBNM'.split('') as readonly string[]

export interface ModelSize {
  width: number
  height: number
}

export function useModel() {
  const modelStore = useModelStore()
  const petStore = usePetStore()
  const modelSize = ref<ModelSize>()

  function getBehaviorShortcut(index: number) {
    const primary = isMac ? 'Command' : 'Control'
    const modifierGroups = [
      [primary],
      [primary, 'Shift'],
      [primary, 'Alt'],
      [primary, 'Shift', 'Alt'],
    ]
    const tiers = [
      ...modifierGroups.map(modifiers => ({ modifiers, keys: digitKeys })),
      ...modifierGroups.map(modifiers => ({ modifiers, keys: letterKeys })),
    ]
    let nextIndex = index

    for (const tier of tiers) {
      if (nextIndex < tier.keys.length) {
        return [...tier.modifiers, tier.keys[nextIndex]].join('+')
      }

      nextIndex -= tier.keys.length
    }

    return ''
  }

  function getMotionShortcutId(modelId: string, groupName: string, index: number) {
    return `${modelId}:motion:${groupName}:${index}`
  }

  function getExpressionShortcutId(modelId: string, index: number) {
    return `${modelId}:expression:${index}`
  }

  async function handleLoad() {
    const currentModel = modelStore.currentModel

    if (!currentModel) return

    try {
      const { width, height, motions, expressions } = await live2d.load(currentModel.path)
      const nextMotions = Object.entries(motions)

      modelSize.value = { width, height }
      modelStore.currentMotions = nextMotions
      modelStore.currentExpressions = expressions

      await handleResize()

      const behaviorIds = [
        ...nextMotions.flatMap(([groupName, items]) => items.map((_, index) => {
          return getMotionShortcutId(currentModel.id, groupName, index)
        })),
        ...expressions.map((_, index) => getExpressionShortcutId(currentModel.id, index)),
      ]

      for (const [index, id] of behaviorIds.entries()) {
        if (modelStore.shortcuts[id]) continue

        const shortcut = getBehaviorShortcut(index)

        if (shortcut) modelStore.shortcuts[id] = shortcut
      }
    } catch (error) {
      message.error(String(error))
    }
  }

  function handleDestroy() {
    live2d.destroy()
    modelSize.value = undefined
    modelStore.currentMotions = []
    modelStore.currentExpressions = []
  }

  async function handleResize() {
    if (!modelSize.value) return

    live2d.resizeModel(modelSize.value)

    const { width, height } = modelSize.value

    if (round(innerWidth / innerHeight, 1) !== round(width / height, 1)) {
      await appWindow.setSize(new LogicalSize({
        width: innerWidth,
        height: Math.ceil(innerWidth * (height / width)),
      }))
    }

    const size = await appWindow.size()
    petStore.window.scale = round((size.width / width) * 100)
  }

  return {
    modelSize,
    handleLoad,
    handleDestroy,
    handleResize,
  }
}
