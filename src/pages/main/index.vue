<script setup lang="ts">
import type { MotionInfo } from 'easy-live2d'

import { convertFileSrc } from '@tauri-apps/api/core'
import { PhysicalSize } from '@tauri-apps/api/dpi'
import { Menu, PredefinedMenuItem } from '@tauri-apps/api/menu'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { exists } from '@tauri-apps/plugin-fs'
import { useDebounceFn, useEventListener } from '@vueuse/core'
import { round } from 'es-toolkit'
import { onUnmounted, ref, watch } from 'vue'

import { useAppMenu } from '@/composables/useAppMenu'
import { useModel } from '@/composables/useModel'
import { useTauriListen } from '@/composables/useTauriListen'
import { LISTEN_KEY, WINDOW_LABEL } from '@/constants'
import { useInputVisualizer } from '@/features/input-visualizer/useInputVisualizer'
import { hideWindow, setAlwaysOnTop, setTaskbarVisibility, showWindow } from '@/plugins/window'
import { useGeneralStore } from '@/stores/general'
import { useModelStore } from '@/stores/model'
import { usePetStore } from '@/stores/pet'
import live2d from '@/utils/live2d'
import { join } from '@/utils/path'
import { isWindows } from '@/utils/platform'

const appWindow = getCurrentWebviewWindow()
const { modelSize, handleLoad, handleDestroy, handleResize } = useModel()
const petStore = usePetStore()
const { getBaseMenu, getExitMenu } = useAppMenu()
const modelStore = useModelStore()
const generalStore = useGeneralStore()
const resizing = ref(false)
const backgroundImagePath = ref<string>()

useInputVisualizer()
onUnmounted(handleDestroy)

const debouncedResize = useDebounceFn(async () => {
  await handleResize()
  resizing.value = false
}, 100)

useEventListener('resize', () => {
  if (!modelStore.currentModel) return

  resizing.value = true
  debouncedResize()
})

watch(() => modelStore.currentModel, async (model) => {
  handleDestroy()
  backgroundImagePath.value = undefined

  if (!model) {
    modelStore.modelReady = true
    return
  }

  modelStore.modelReady = false

  try {
    await handleLoad()

    const backgroundPath = join(model.path, 'resources', 'background.png')
    backgroundImagePath.value = await exists(backgroundPath)
      ? convertFileSrc(backgroundPath)
      : undefined
  } finally {
    modelStore.modelReady = true
  }
}, { immediate: true })

watch([() => petStore.window.scale, modelSize], async ([scale, size]) => {
  if (!size) return

  await appWindow.setSize(new PhysicalSize({
    width: Math.round(size.width * (scale / 100)),
    height: Math.round(size.height * (scale / 100)),
  }))
}, { immediate: true })

watch(() => petStore.window.visible, value => value ? showWindow() : hideWindow())
watch(() => petStore.window.passThrough, value => appWindow.setIgnoreCursorEvents(value), { immediate: true })
watch(() => petStore.window.alwaysOnTop, setAlwaysOnTop, { immediate: true })
watch(() => generalStore.app.taskbarVisible, setTaskbarVisibility, { immediate: true })
watch(() => petStore.model.motionSound, live2d.setMotionSoundEnabled, { immediate: true })
watch(() => petStore.model.maxFPS, live2d.setMaxFPS, { immediate: true })

useTauriListen<MotionInfo>(LISTEN_KEY.START_MOTION, ({ payload }) => {
  live2d.startMotion(payload)
})

useTauriListen<number>(LISTEN_KEY.SET_EXPRESSION, ({ payload }) => {
  live2d.setExpression(payload)
})

function handleMouseDown(event: MouseEvent) {
  if ((event.target as HTMLElement).closest('button')) return
  appWindow.startDragging()
}

async function handleContextmenu(event: MouseEvent) {
  event.preventDefault()

  if (event.shiftKey) return

  const menu = await Menu.new({
    items: [
      ...await getBaseMenu(),
      await PredefinedMenuItem.new({ item: 'Separator' }),
      ...await getExitMenu(),
    ],
  })

  if (isWindows && petStore.window.alwaysOnTop) setAlwaysOnTop(false)
  await menu.popup()
  if (isWindows && petStore.window.alwaysOnTop) setAlwaysOnTop(true)
}

function handleMouseMove(event: MouseEvent) {
  const { buttons, shiftKey, movementX, movementY } = event

  if (buttons !== 2 || !shiftKey) return

  const delta = (movementX + movementY) * 0.5
  petStore.window.scale = round(Math.max(10, Math.min(petStore.window.scale + delta, 500)))
}

function openModelManager() {
  showWindow(WINDOW_LABEL.PREFERENCE)
}
</script>

<template>
  <div
    class="relative size-screen overflow-hidden"
    :class="{ '-scale-x-100': petStore.model.mirror && modelStore.currentModel }"
    :style="{
      opacity: petStore.window.opacity / 100,
      borderRadius: `${petStore.window.radius}%`,
    }"
    @contextmenu="handleContextmenu"
    @mousedown="handleMouseDown"
    @mousemove="handleMouseMove"
  >
    <template v-if="modelStore.currentModel">
      <img
        v-if="backgroundImagePath"
        class="absolute size-full object-cover"
        :src="backgroundImagePath"
      >

      <canvas
        id="live2dCanvas"
        class="absolute size-full"
      />

      <img
        v-for="path in modelStore.pressedKeys"
        :key="path"
        class="absolute size-full object-cover"
        :src="convertFileSrc(path)"
      >

      <div
        v-show="resizing || !modelStore.modelReady"
        class="absolute inset-0 flex items-center justify-center bg-black/85"
      >
        <span class="text-center text-5 text-white">
          {{ resizing ? $t('pages.main.hints.redrawing') : $t('pages.main.hints.switching') }}
        </span>
      </div>
    </template>

    <div
      v-else
      class="empty-model absolute inset-3 flex flex-col items-center justify-center gap-3 px-6 text-center"
    >
      <img
        alt=""
        class="size-16 rounded-lg"
        src="/momopet-app-icon.png"
      >

      <div>
        <div class="text-4.5 text-[#263238] font-700">
          {{ $t('pages.main.empty.title') }}
        </div>
        <div class="mt-1 text-3.25 text-[#526066]">
          {{ $t('pages.main.empty.description') }}
        </div>
      </div>

      <button
        class="h-9 inline-flex items-center gap-2 border-0 bg-[#e85d4a] px-4 text-3.25 text-white transition rounded-md hover:bg-[#cf4938]"
        type="button"
        @click="openModelManager"
      >
        <span class="i-lucide:folder-open size-4" />
        {{ $t('pages.main.empty.action') }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.empty-model {
  border: 1px solid rgb(38 50 56 / 16%);
  background:
    linear-gradient(rgb(255 255 255 / 92%), rgb(250 251 249 / 92%)),
    repeating-linear-gradient(90deg, transparent 0 23px, rgb(31 111 115 / 8%) 24px);
  box-shadow: 0 8px 28px rgb(20 36 38 / 18%);
  backdrop-filter: blur(12px);
}
</style>
