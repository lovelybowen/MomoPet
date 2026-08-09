<script setup lang="ts">
import { emit } from '@tauri-apps/api/event'
import { Empty, Modal } from 'antdv-next'
import { isEmpty } from 'es-toolkit/compat'

import { LISTEN_KEY } from '@/constants'
import { getActionShortcutId } from '@/features/pet-actions'
import { useModelStore } from '@/stores/model'

import BehaviorItem from './components/behavior-item/index.vue'

const modelValue = defineModel<boolean>()
const modelStore = useModelStore()

function shortcutId(actionId: string) {
  return getActionShortcutId(modelStore.currentModel?.id ?? '', actionId)
}

function startAction(actionId: string) {
  emit(LISTEN_KEY.START_ACTION, actionId)
}
</script>

<template>
  <Modal
    v-model:open="modelValue"
    :cancel-text="false"
    centered
    :footer="null"
    force-render
    :title="$t('pages.preference.model.behaviorModal.title')"
  >
    <div class="flex flex-col">
      <Empty
        v-if="isEmpty(modelStore.currentModel?.actions)"
        :image="Empty.PRESENTED_IMAGE_SIMPLE"
      />

      <div
        v-else
        class="max-h-[60vh] overflow-y-auto b-1 b-solid b-border rounded-lg"
      >
        <template
          v-for="action in modelStore.currentModel?.actions"
          :key="action.id"
        >
          <BehaviorItem
            v-model="modelStore.shortcuts[shortcutId(action.id)]"
            :label="action.name"
            @click="startAction(action.id)"
          />
        </template>
      </div>
    </div>
  </Modal>
</template>
