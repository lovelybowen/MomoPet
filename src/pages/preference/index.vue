<script setup lang="ts">
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { Flex, Spin } from 'antdv-next'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { useTray } from '@/composables/useTray'
import { useAppStore } from '@/stores/app'
import { useGeneralStore } from '@/stores/general'
import { useModelStore } from '@/stores/model'
import { isMac } from '@/utils/platform'

import About from './components/about/index.vue'
import General from './components/general/index.vue'
import Model from './components/model/index.vue'
import Pet from './components/pet/index.vue'
import Shortcut from './components/shortcut/index.vue'

useTray()
const appStore = useAppStore()
const current = ref(0)
const { t } = useI18n()
const generalStore = useGeneralStore()
const modelStore = useModelStore()
const appWindow = getCurrentWebviewWindow()

watch(() => generalStore.appearance.language, () => {
  appWindow.setTitle(t('pages.preference.title'))
}, { immediate: true })

const menus = computed(() => [
  {
    key: 'pet',
    label: t('pages.preference.pet.title'),
    icon: 'i-lucide:paw-print',
    component: Pet,
  },
  {
    key: 'general',
    label: t('pages.preference.general.title'),
    icon: 'i-solar:settings-minimalistic-bold',
    component: General,
  },
  {
    key: 'model',
    label: t('pages.preference.model.title'),
    icon: 'i-solar:magic-stick-3-bold',
    component: Model,
  },
  {
    key: 'shortcut',
    label: t('pages.preference.shortcut.title'),
    icon: 'i-solar:keyboard-bold',
    component: Shortcut,
  },
  {
    key: 'about',
    label: t('pages.preference.about.title'),
    icon: 'i-solar:info-circle-bold',
    component: About,
  },
])
</script>

<template>
  <Spin
    class="max-h-unset!"
    :description="t('pages.main.hints.switching')"
    fullscreen
    size="large"
    :spinning="!modelStore.modelReady"
  />

  <Flex class="h-screen">
    <div
      class="h-full w-32 flex flex-col items-center gap-4 overflow-auto border-r border-[#dde4e2] bg-[#f7f9f8] dark:border-[#34413f] dark:bg-[#202725]"
      :class="[isMac ? 'pt-8' : 'pt-4']"
      data-tauri-drag-region
    >
      <div class="flex flex-col items-center gap-2">
        <div class="overflow-hidden border-[#d6ddda] border rounded-lg">
          <img
            class="size-15"
            data-tauri-drag-region
            src="/momopet-app-icon.png"
          >
        </div>

        <span class="font-bold">{{ appStore.name }}</span>
      </div>

      <div class="flex flex-col gap-2">
        <div
          v-for="(item, index) in menus"
          :key="item.key"
          class="size-20 flex flex-col cursor-pointer items-center justify-center gap-2 transition color-text-tertiary rounded-md hover:bg-[#e9efed] dark:color-text-secondary dark:hover:bg-[#303a38]"
          :class="{ 'bg-[#dcebea]! color-[#146b70]! font-bold dark:bg-[#314a47]! dark:color-[#75c9c4]!': current === index }"
          @click="current = index"
        >
          <div
            class="size-8"
            :class="item.icon"
          />

          <span>{{ item.label }}</span>
        </div>
      </div>
    </div>

    <div
      v-for="(item, index) in menus"
      v-show="current === index"
      :key="item.key"
      class="flex-1 overflow-auto bg-[--ant-color-fill-quaternary] p-4 dark:bg-container"
      data-tauri-drag-region
    >
      <component :is="item.component" />
    </div>
  </Flex>
</template>
