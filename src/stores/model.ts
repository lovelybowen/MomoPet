import { invoke } from '@tauri-apps/api/core'
import { find } from 'es-toolkit/compat'
import { defineStore } from 'pinia'
import { reactive, ref } from 'vue'

import { INVOKE_KEY } from '@/constants'

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

export interface AnimationAction extends PetActionBase {
  type: 'animation'
  clip: string
  mode: 'once' | 'toggle'
}

export type PetAction = AnimationAction

export interface InstalledModel {
  id: string
  version: string
  name: string
  description?: string
  authors: PetAuthor[]
  license?: PetLicense
  runtimeType: 'sprite2d'
  path: string
  entryPath: string
  coverPath?: string
  backgroundPath?: string
  actions: PetAction[]
  isBuiltin: boolean
}

export const useModelStore = defineStore('model', () => {
  const modelReady = ref(true)
  const models = ref<InstalledModel[]>([])
  const currentModel = ref<InstalledModel>()
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
    shortcuts,
    init,
  }
}, {
  tauri: {
    filterKeys: [],
  },
})
