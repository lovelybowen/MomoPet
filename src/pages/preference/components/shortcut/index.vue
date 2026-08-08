<script setup lang="ts">
import { storeToRefs } from 'pinia'

import ProListItem from '@/components/pro-list-item/index.vue'
import ProList from '@/components/pro-list/index.vue'
import Shortcut from '@/components/shortcut/index.vue'
import { useKeyPress } from '@/composables/useKeyPress'
import { WINDOW_LABEL } from '@/constants'
import { toggleWindowVisible } from '@/plugins/window'
import { usePetStore } from '@/stores/pet'
import { useShortcutStore } from '@/stores/shortcut.ts'

const shortcutStore = useShortcutStore()
const { visiblePet, visiblePreference, mirrorMode, passThrough, alwaysOnTop } = storeToRefs(shortcutStore)
const petStore = usePetStore()

useKeyPress(visiblePet, () => {
  petStore.window.visible = !petStore.window.visible
})

useKeyPress(visiblePreference, () => {
  toggleWindowVisible(WINDOW_LABEL.PREFERENCE)
})

useKeyPress(mirrorMode, () => {
  petStore.model.mirror = !petStore.model.mirror
})

useKeyPress(passThrough, () => {
  petStore.window.passThrough = !petStore.window.passThrough
})

useKeyPress(alwaysOnTop, () => {
  petStore.window.alwaysOnTop = !petStore.window.alwaysOnTop
})
</script>

<template>
  <ProList :title="$t('pages.preference.shortcut.title')">
    <ProListItem
      :description="$t('pages.preference.shortcut.hints.togglePet')"
      :title="$t('pages.preference.shortcut.labels.togglePet')"
    >
      <Shortcut v-model="shortcutStore.visiblePet" />
    </ProListItem>

    <ProListItem
      :description="$t('pages.preference.shortcut.hints.togglePreferences')"
      :title="$t('pages.preference.shortcut.labels.togglePreferences')"
    >
      <Shortcut v-model="shortcutStore.visiblePreference" />
    </ProListItem>

    <ProListItem
      :description="$t('pages.preference.shortcut.hints.mirrorMode')"
      :title="$t('pages.preference.shortcut.labels.mirrorMode')"
    >
      <Shortcut v-model="shortcutStore.mirrorMode" />
    </ProListItem>

    <ProListItem
      :description="$t('pages.preference.shortcut.hints.passThrough')"
      :title="$t('pages.preference.shortcut.labels.passThrough')"
    >
      <Shortcut v-model="shortcutStore.passThrough" />
    </ProListItem>

    <ProListItem
      :description="$t('pages.preference.shortcut.hints.alwaysOnTop')"
      :title="$t('pages.preference.shortcut.labels.alwaysOnTop')"
    >
      <Shortcut v-model="shortcutStore.alwaysOnTop" />
    </ProListItem>
  </ProList>
</template>
