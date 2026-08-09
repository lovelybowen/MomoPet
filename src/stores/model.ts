import { invoke } from '@tauri-apps/api/core'
import { find } from 'es-toolkit/compat'
import { defineStore } from 'pinia'
import { reactive, ref } from 'vue'

import type { InputVisualizerProfile } from '@/features/input-visualizer/profile'

import { INVOKE_KEY } from '@/constants'

export type ModelMode = 'standard' | 'keyboard' | 'gamepad'

export interface PetAuthor {
  name: string
  url?: string
}

export interface PetLicense {
  name: string
  file: string
  url?: string
}

interface PetActionBase {
  id: string
  name: string
  description?: string
}

export interface MotionAction extends PetActionBase {
  type: 'motion'
  motionGroup: string
  motionIndex: number
}

export interface ExpressionAction extends PetActionBase {
  type: 'expression'
  expression: string
}

export type PetAction = MotionAction | ExpressionAction

export interface PetInput {
  mode: ModelMode
  parameters: InputVisualizerProfile
}

export interface InstalledModel {
  id: string
  version: string
  name: string
  description?: string
  authors: PetAuthor[]
  license?: PetLicense
  path: string
  entryPath: string
  resourcePath: string
  coverPath?: string
  backgroundPath?: string
  mode: ModelMode
  input?: PetInput
  actions: PetAction[]
  isBuiltin: boolean
  isLegacy: boolean
}

export const useModelStore = defineStore('model', () => {
  const modelReady = ref(true)
  const models = ref<InstalledModel[]>([])
  const currentModel = ref<InstalledModel>()
  const supportKeys = reactive<Record<string, string>>({})
  const pressedKeys = reactive<Record<string, string>>({})
  const shortcuts = reactive<Record<string, string>>({})

  const init = async () => {
    const selectedId = currentModel.value?.id
    const installed = await invoke<InstalledModel[]>(INVOKE_KEY.LIST_PETS)

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
    shortcuts,
    init,
  }
}, {
  tauri: {
    filterKeys: ['supportKeys', 'pressedKeys'],
  },
})
