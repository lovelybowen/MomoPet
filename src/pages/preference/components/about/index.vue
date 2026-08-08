<script setup lang="ts">
import { getTauriVersion } from '@tauri-apps/api/app'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { arch, platform, version } from '@tauri-apps/plugin-os'
import { Button, message } from 'antdv-next'
import { useI18n } from 'vue-i18n'

import ProListItem from '@/components/pro-list-item/index.vue'
import ProList from '@/components/pro-list/index.vue'
import { useAppStore } from '@/stores/app'

const appStore = useAppStore()
const { t } = useI18n()

async function copyInfo() {
  const info = {
    appName: appStore.name,
    appVersion: appStore.version,
    tauriVersion: await getTauriVersion(),
    platform: platform(),
    platformArch: arch(),
    platformVersion: version(),
  }

  await writeText(JSON.stringify(info, null, 2))
  message.success(t('pages.preference.about.hints.copySuccess'))
}
</script>

<template>
  <ProList :title="$t('pages.preference.about.labels.aboutApp')">
    <ProListItem
      :description="`v${appStore.version}`"
      :title="appStore.name"
    />

    <ProListItem
      :description="$t('pages.preference.about.hints.appInfo')"
      :title="$t('pages.preference.about.labels.appInfo')"
    >
      <Button @click="copyInfo">
        {{ $t('pages.preference.about.buttons.copy') }}
      </Button>
    </ProListItem>

    <ProListItem
      :description="$t('pages.preference.about.hints.license')"
      :title="$t('pages.preference.about.labels.license')"
    />
  </ProList>
</template>
