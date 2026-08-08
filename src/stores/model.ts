import type { ExpressionInfo, MotionInfo } from 'easy-live2d'

import { invoke } from '@tauri-apps/api/core'
import { find } from 'es-toolkit/compat'
import { defineStore } from 'pinia'
import { reactive, ref } from 'vue'

import { INVOKE_KEY } from '@/constants'

export type ModelMode = 'standard' | 'keyboard' | 'gamepad'

export interface InstalledModel {
  id: string
  path: string
  mode: ModelMode
  isBuiltin: boolean
}

export const useModelStore = defineStore('model', () => {
  const modelReady = ref(true)
  const models = ref<InstalledModel[]>([])
  const currentModel = ref<InstalledModel>()
  const supportKeys = reactive<Record<string, string>>({})
  const pressedKeys = reactive<Record<string, string>>({})
  const currentMotions = ref<Array<[string, MotionInfo[]]>>([])
  const currentExpressions = ref<ExpressionInfo[]>([])
  const shortcuts = reactive<Record<string, string>>({})

  const init = async () => {
    const selectedId = currentModel.value?.id
    const installed = await invoke<InstalledModel[]>(INVOKE_KEY.LIST_MODELS)

    models.value = installed
    currentModel.value = find(installed, { id: selectedId }) ?? installed[0]
    modelReady.value = true
  }

  return {
    modelReady,
    models,
    currentModel,
    supportKeys,
    pressedKeys,
    currentMotions,
    currentExpressions,
    shortcuts,
    init,
  }
}, {
  tauri: {
    filterKeys: ['supportKeys', 'pressedKeys', 'currentMotions', 'currentExpressions'],
  },
})
