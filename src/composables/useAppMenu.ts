import { CheckMenuItem, MenuItem, PredefinedMenuItem, Submenu } from '@tauri-apps/api/menu'
import { exit, relaunch } from '@tauri-apps/plugin-process'
import { range } from 'es-toolkit'
import { useI18n } from 'vue-i18n'

import { WINDOW_LABEL } from '@/constants'
import { showWindow } from '@/plugins/window'
import { usePetStore } from '@/stores/pet'
import { isMac } from '@/utils/platform'

export function useAppMenu() {
  const petStore = usePetStore()
  const { t } = useI18n()

  const getScaleMenuItems = async () => {
    const options = range(50, 151, 25)

    const items = options.map((item) => {
      return CheckMenuItem.new({
        text: `${item}%`,
        checked: petStore.window.scale === item,
        action: () => {
          petStore.window.scale = item
        },
      })
    })

    if (!options.includes(petStore.window.scale)) {
      items.unshift(CheckMenuItem.new({
        text: `${petStore.window.scale}%`,
        checked: true,
        enabled: false,
      }))
    }

    return Promise.all(items)
  }

  const getOpacityMenuItems = async () => {
    const options = range(25, 101, 25)

    const items = options.map((item) => {
      return CheckMenuItem.new({
        text: `${item}%`,
        checked: petStore.window.opacity === item,
        action: () => {
          petStore.window.opacity = item
        },
      })
    })

    if (!options.includes(petStore.window.opacity)) {
      items.unshift(CheckMenuItem.new({
        text: `${petStore.window.opacity}%`,
        checked: true,
        enabled: false,
      }))
    }

    return Promise.all(items)
  }

  const getBaseMenu = async () => {
    return await Promise.all([
      MenuItem.new({
        text: t('composables.useAppMenu.labels.preference'),
        accelerator: isMac ? 'Cmd+,' : '',
        action: () => showWindow(WINDOW_LABEL.PREFERENCE),
      }),
      MenuItem.new({
        text: petStore.window.visible
          ? t('composables.useAppMenu.labels.hidePet')
          : t('composables.useAppMenu.labels.showPet'),
        action: () => {
          petStore.window.visible = !petStore.window.visible
        },
      }),
      PredefinedMenuItem.new({ item: 'Separator' }),
      CheckMenuItem.new({
        text: t('composables.useAppMenu.labels.passThrough'),
        checked: petStore.window.passThrough,
        action: () => {
          petStore.window.passThrough = !petStore.window.passThrough
        },
      }),
      Submenu.new({
        text: t('composables.useAppMenu.labels.windowSize'),
        items: await getScaleMenuItems(),
      }),
      Submenu.new({
        text: t('composables.useAppMenu.labels.opacity'),
        items: await getOpacityMenuItems(),
      }),
    ])
  }

  const getExitMenu = async () => {
    return await Promise.all([
      MenuItem.new({
        text: t('composables.useAppMenu.labels.restartApp'),
        action: relaunch,
      }),
      MenuItem.new({
        text: t('composables.useAppMenu.labels.quitApp'),
        accelerator: isMac ? 'Cmd+Q' : '',
        action: () => exit(0),
      }),
    ])
  }

  return {
    getBaseMenu,
    getExitMenu,
  }
}
