<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { open } from '@tauri-apps/plugin-dialog'
import { message } from 'antdv-next'
import { onMounted, onUnmounted, ref, useTemplateRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type { InstalledModel } from '@/stores/model'

import { INVOKE_KEY } from '@/constants'
import { useModelStore } from '@/stores/model'

const dropRef = useTemplateRef('drop')
const dragenter = ref(false)
const selectedPaths = ref<string[]>([])
const modelStore = useModelStore()
const { t } = useI18n()
let unlistenDragDrop: (() => void) | undefined

onMounted(async () => {
  unlistenDragDrop = await getCurrentWebviewWindow().onDragDropEvent(({ payload }) => {
    if (payload.type === 'over') {
      const { x, y } = payload.position
      const bounds = dropRef.value?.getBoundingClientRect()

      dragenter.value = Boolean(
        bounds
        && x >= bounds.left
        && x <= bounds.right
        && y >= bounds.top
        && y <= bounds.bottom,
      )
    } else if (payload.type === 'drop' && dragenter.value) {
      dragenter.value = false
      selectedPaths.value = payload.paths
    } else {
      dragenter.value = false
    }
  })
})

onUnmounted(() => unlistenDragDrop?.())

async function handleUpload() {
  const selected = await open({
    multiple: true,
    filters: [{ name: 'MomoPet', extensions: ['momopet'] }],
  })

  if (!selected) return
  selectedPaths.value = selected
}

watch(selectedPaths, async (paths) => {
  for (const sourcePath of paths) {
    try {
      const installed = await invoke<InstalledModel>(INVOKE_KEY.IMPORT_PET, { sourcePath })
      const existingIndex = modelStore.models.findIndex(model => model.id === installed.id)

      if (existingIndex >= 0) {
        modelStore.models[existingIndex] = installed
      } else {
        modelStore.models.push(installed)
      }

      modelStore.currentModel = installed
      message.success(t('pages.preference.model.hints.importSuccess'))
    } catch (error) {
      message.error(String(error))
    }
  }

  selectedPaths.value = []
})
</script>

<template>
  <button
    ref="drop"
    class="h-44 w-full flex flex-col items-center justify-center gap-3 border-[#91a5a1] border-dashed bg-[#f7faf9] text-[#315b59] transition border rounded-md hover:border-[#1f7a78] dark:bg-[#27302e] hover:bg-[#eef6f4] dark:text-[#8fd0cb]"
    :class="{ 'border-[#1f7a78]! bg-[#e4f2ef]!': dragenter }"
    type="button"
    @click="handleUpload"
  >
    <span class="i-lucide:package-up size-8" />
    <span>{{ $t('pages.preference.model.hints.clickOrDragToImport') }}</span>
  </button>
</template>
