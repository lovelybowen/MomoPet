import { defineStore } from 'pinia'
import { reactive } from 'vue'

export interface PetStore {
  model: {
    mirror: boolean
    mouseMirror: boolean
    motionSound: boolean
    behavior: boolean
    inputVisualizer: boolean
    autoReleaseDelay: number
    maxFPS: number
    ignoreMouse: boolean
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
    mouseMirror: false,
    motionSound: true,
    behavior: true,
    inputVisualizer: true,
    autoReleaseDelay: 3,
    maxFPS: 60,
    ignoreMouse: false,
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
