<script setup lang="ts">
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { Card, message, Popconfirm } from 'antdv-next'
import { nextTick, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import type { InstalledModel } from '@/stores/model'

import { INVOKE_KEY } from '@/constants'
import { useModelStore } from '@/stores/model'
import { usePetStore } from '@/stores/pet'
import { join } from '@/utils/path'

import BehaviorModal from './components/behavior-modal/index.vue'
import Upload from './components/upload/index.vue'

const petStore = usePetStore()
const modelStore = useModelStore()
const { t } = useI18n()
const openBehaviorModal = ref(false)

function handleToggle(nextModel: InstalledModel) {
  if (modelStore.currentModel?.id === nextModel.id) return
  modelStore.currentModel = nextModel
}

function handleCoverError(event: Event) {
  const image = event.currentTarget as HTMLImageElement

  if (!image.src.endsWith('/momopet-app-icon.png')) {
    image.src = '/momopet-app-icon.png'
  }
}

async function handleDelete(model: InstalledModel) {
  const previousModel = modelStore.currentModel
  const nextModels = modelStore.models.filter(item => item.id !== model.id)

  if (previousModel?.id === model.id) {
    modelStore.currentModel = nextModels[0]
    await nextTick()
  }

  try {
    await invoke(INVOKE_KEY.REMOVE_MODEL, { modelId: model.id })
    modelStore.models = nextModels
    message.success(t('pages.preference.model.hints.deleteSuccess'))
  } catch (error) {
    modelStore.currentModel = previousModel
    message.error(String(error))
  }
}
</script>

<template>
  <section class="model-library">
    <Upload />

    <div
      v-if="modelStore.models.length"
      class="grid grid-cols-2 mt-4 gap-4 2xl:grid-cols-4 xl:grid-cols-3"
    >
      <Card
        v-for="model in modelStore.models"
        :key="model.id"
        :class="{ 'selected-model': model.id === modelStore.currentModel?.id }"
        :classes="{
          actions: `[&>li]:(flex justify-center) [&>li>span]:(inline-flex! justify-center text-4!)`,
        }"
        hoverable
        size="small"
        @click="handleToggle(model)"
      >
        <template #cover>
          <img
            alt=""
            class="aspect-ratio-[4/3] object-cover"
            :src="convertFileSrc(join(model.path, 'resources', 'cover.png'))"
            @error="handleCoverError"
          >
        </template>

        <template #actions>
          <i
            class="i-lucide:circle-check"
            :class="{ 'text-[#14837c]': model.id === modelStore.currentModel?.id }"
            :title="$t('pages.preference.model.tooltips.selectModel')"
          />

          <i
            v-if="petStore.model.behavior && modelStore.currentModel?.id === model.id"
            class="i-lucide:smile"
            :title="$t('pages.preference.model.tooltips.behavior')"
            @click.stop="openBehaviorModal = true"
          />

          <Popconfirm
            v-if="!model.isBuiltin"
            :description="$t('pages.preference.model.hints.deleteModel')"
            placement="topRight"
            :title="$t('pages.preference.model.labels.deleteModel')"
            @confirm="handleDelete(model)"
          >
            <i
              class="i-lucide:trash-2"
              :title="$t('pages.preference.model.labels.deleteModel')"
              @click.stop
            />
          </Popconfirm>
        </template>
      </Card>
    </div>

    <div
      v-else
      class="mt-8 flex flex-col items-center gap-2 text-center text-[#61706d]"
    >
      <span class="i-lucide:package-open size-8" />
      <span class="font-600">{{ $t('pages.preference.model.empty.title') }}</span>
      <span class="text-3.25">{{ $t('pages.preference.model.empty.description') }}</span>
    </div>
  </section>

  <BehaviorModal
    v-if="petStore.model.behavior"
    v-model="openBehaviorModal"
  />
</template>

<style scoped>
.model-library :deep(.selected-model) {
  border-color: #14837c;
  box-shadow: 0 0 0 1px #14837c;
}
</style>
