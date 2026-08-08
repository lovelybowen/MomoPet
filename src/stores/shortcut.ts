import { defineStore } from 'pinia'
import { ref } from 'vue'

export type HotKey = 'visiblePet' | 'mirrorMode' | 'passThrough' | 'alwaysOnTop'

export const useShortcutStore = defineStore('shortcut', () => {
  const visiblePet = ref('')
  const visiblePreference = ref('')
  const mirrorMode = ref('')
  const passThrough = ref('')
  const alwaysOnTop = ref('')

  return {
    visiblePet,
    visiblePreference,
    mirrorMode,
    passThrough,
    alwaysOnTop,
  }
})
