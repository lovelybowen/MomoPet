<script setup lang="ts">
import { Divider, Flex, InputNumber, Slider, SpaceAddon, SpaceCompact, Switch } from 'antdv-next'

import ProListItem from '@/components/pro-list-item/index.vue'
import ProList from '@/components/pro-list/index.vue'
import { usePetStore } from '@/stores/pet'

const petStore = usePetStore()
</script>

<template>
  <ProList :title="$t('pages.preference.pet.labels.modelSettings')">
    <ProListItem
      :description="$t('pages.preference.pet.hints.mirrorMode')"
      :title="$t('pages.preference.pet.labels.mirrorMode')"
    >
      <Switch v-model:checked="petStore.model.mirror" />
    </ProListItem>

    <ProListItem
      :description="$t('pages.preference.pet.hints.behavior')"
      :title="$t('pages.preference.pet.labels.behavior')"
    >
      <Switch v-model:checked="petStore.model.behavior" />
    </ProListItem>

    <ProListItem
      :description="$t('pages.preference.pet.hints.maxFPS')"
      :title="$t('pages.preference.pet.labels.maxFPS')"
    >
      <InputNumber
        v-model:value="petStore.model.maxFPS"
        class="w-20"
        :min="0"
      />
    </ProListItem>
  </ProList>

  <ProList :title="$t('pages.preference.pet.labels.windowSettings')">
    <ProListItem
      :description="$t('pages.preference.pet.hints.passThrough')"
      :title="$t('pages.preference.pet.labels.passThrough')"
    >
      <Switch v-model:checked="petStore.window.passThrough" />
    </ProListItem>

    <ProListItem
      :description="$t('pages.preference.pet.hints.alwaysOnTop')"
      :title="$t('pages.preference.pet.labels.alwaysOnTop')"
    >
      <Switch v-model:checked="petStore.window.alwaysOnTop" />
    </ProListItem>

    <ProListItem
      :description="$t('pages.preference.pet.hints.hideOnHover')"
      :title="$t('pages.preference.pet.labels.hideOnHover')"
    >
      <Flex align="center">
        <Switch v-model:checked="petStore.window.hideOnHover" />

        <Flex
          align="center"
          class="overflow-hidden transition-all"
          :class="[petStore.window.hideOnHover ? 'w-28 opacity-100' : 'w-0 opacity-0']"
        >
          <Divider type="vertical" />

          <SpaceCompact>
            <InputNumber
              v-model:value="petStore.window.hideOnHoverDelay"
              class="w-16"
              :min="0"
            />

            <SpaceAddon>s</SpaceAddon>
          </SpaceCompact>
        </Flex>
      </Flex>
    </ProListItem>

    <ProListItem
      :description="$t('pages.preference.pet.hints.keepInScreen')"
      :title="$t('pages.preference.pet.labels.keepInScreen')"
    >
      <Switch v-model:checked="petStore.window.keepInScreen" />
    </ProListItem>

    <ProListItem
      :description="$t('pages.preference.pet.hints.windowSize')"
      :title="$t('pages.preference.pet.labels.windowSize')"
    >
      <SpaceCompact>
        <InputNumber
          v-model:value="petStore.window.scale"
          class="w-20"
          :max="500"
          :min="1"
        />

        <SpaceAddon>%</SpaceAddon>
      </SpaceCompact>
    </ProListItem>

    <ProListItem :title="$t('pages.preference.pet.labels.windowRadius')">
      <SpaceCompact>
        <InputNumber
          v-model:value="petStore.window.radius"
          class="w-20"
          :min="0"
        />

        <SpaceAddon>%</SpaceAddon>
      </SpaceCompact>
    </ProListItem>

    <ProListItem
      :title="$t('pages.preference.pet.labels.opacity')"
      vertical
    >
      <Slider
        v-model:value="petStore.window.opacity"
        class="m-0!"
        :max="100"
        :min="10"
        :tooltip="{
          formatter(value) {
            return `${value}%`
          },
        }"
      />
    </ProListItem>
  </ProList>
</template>
