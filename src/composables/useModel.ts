import { LogicalSize } from '@tauri-apps/api/dpi'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { message } from 'antdv-next'
import { round } from 'es-toolkit'
import { ref } from 'vue'

import { ensureActionShortcuts } from '@/features/pet-actions'
import { useModelStore } from '@/stores/model'
import { usePetStore } from '@/stores/pet'
import { isMac } from '@/utils/platform'
import sprite2d from '@/utils/sprite2d'

const appWindow = getCurrentWebviewWindow()

export interface ModelSize {
  width: number
  height: number
}

export function useModel() {
  const modelStore = useModelStore()
  const petStore = usePetStore()
  const modelSize = ref<ModelSize>()

  async function handleLoad() {
    const currentModel = modelStore.currentModel

    if (!currentModel) return

    try {
      const { width, height } = await sprite2d.load(currentModel)

      modelSize.value = { width, height }

      await handleResize()

      ensureActionShortcuts(
        modelStore.shortcuts,
        currentModel.id,
        currentModel.actions.map(action => action.id),
        isMac ? 'Command' : 'Control',
      )
    } catch (error) {
      message.error(String(error))
    }
  }

  function handleDestroy() {
    sprite2d.destroy()
    modelSize.value = undefined
  }

  async function handleResize() {
    if (!modelSize.value) return

    sprite2d.resizeModel(modelSize.value)

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
