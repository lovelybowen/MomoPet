import { defineStore } from 'pinia'
import { reactive } from 'vue'

export interface PetStore {
  model: {
    mirror: boolean
    behavior: boolean
    maxFPS: number
  }
  window: {
    visible: boolean
    passThrough: boolean
    alwaysOnTop: boolean
    scale: number
    opacity: number
    radius: number
    hideOnHover: boolean
    hideOnHoverDelay: number
    keepInScreen: boolean
  }
}

export const usePetStore = defineStore('pet', () => {
  const model = reactive<PetStore['model']>({
    mirror: false,
    behavior: true,
    maxFPS: 60,
  })

  const window = reactive<PetStore['window']>({
    visible: true,
    passThrough: false,
    alwaysOnTop: true,
    scale: 100,
    opacity: 100,
    radius: 0,
    hideOnHover: false,
    hideOnHoverDelay: 0,
    keepInScreen: true,
  })

  return {
    model,
    window,
  }
})
