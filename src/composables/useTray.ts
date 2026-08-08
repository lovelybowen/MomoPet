import type { TrayIconOptions } from '@tauri-apps/api/tray'

import { getName, getVersion } from '@tauri-apps/api/app'
import { Menu, MenuItem, PredefinedMenuItem } from '@tauri-apps/api/menu'
import { resolveResource } from '@tauri-apps/api/path'
import { TrayIcon } from '@tauri-apps/api/tray'
import { watchDebounced } from '@vueuse/core'
import { watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { useGeneralStore } from '@/stores/general'
import { usePetStore } from '@/stores/pet'
import { isMac } from '@/utils/platform'

import { useAppMenu } from './useAppMenu'

const TRAY_ID = 'MOMOPET_TRAY'

export function useTray() {
  const petStore = usePetStore()
  const generalStore = useGeneralStore()
  const { getBaseMenu, getExitMenu } = useAppMenu()
  const { locale } = useI18n()

  const getTrayById = () => TrayIcon.getById(TRAY_ID)

  const getTrayMenu = async () => {
    const appVersion = await getVersion()
    const items = await Promise.all([
      ...await getBaseMenu(),
      PredefinedMenuItem.new({ item: 'Separator' }),
      MenuItem.new({ text: `v${appVersion}`, enabled: false }),
      ...await getExitMenu(),
    ])

    return Menu.new({ items })
  }

  const createTray = async () => {
    const existingTray = await getTrayById()

    if (existingTray) return existingTray

    const appName = await getName()
    const appVersion = await getVersion()
    const path = isMac ? 'assets/tray-mac.png' : 'assets/tray.png'
    const options: TrayIconOptions = {
      menu: await getTrayMenu(),
      icon: await resolveResource(path),
      id: TRAY_ID,
      tooltip: `${appName} v${appVersion}`,
      iconAsTemplate: false,
      menuOnLeftClick: true,
    }

    return TrayIcon.new(options)
  }

  const updateTrayMenu = async () => {
    const tray = await getTrayById()

    if (tray) await tray.setMenu(await getTrayMenu())
  }

  watch(
    [() => petStore.window.visible, () => petStore.window.passThrough, locale],
    updateTrayMenu,
  )
  watchDebounced(
    [() => petStore.window.scale, () => petStore.window.opacity],
    updateTrayMenu,
    { debounce: 200 },
  )
  watch(() => generalStore.app.trayVisible, async (visible) => {
    const tray = await getTrayById() ?? await createTray()

    await tray.setVisible(visible)
  }, { immediate: true })
}
