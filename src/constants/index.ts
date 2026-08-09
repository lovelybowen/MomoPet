export const LISTEN_KEY = {
  SHOW_WINDOW: 'show-window',
  HIDE_WINDOW: 'hide-window',
  DEVICE_CHANGED: 'device-changed',
  GAMEPAD_CHANGED: 'gamepad-changed',
  START_ACTION: 'start-action',
} as const

export const INVOKE_KEY = {
  IMPORT_PET: 'import_pet_package',
  LIST_PETS: 'list_installed_pets',
  REMOVE_PET: 'remove_installed_pet',
  START_DEVICE_LISTENING: 'start_device_listening',
  STOP_DEVICE_LISTENING: 'stop_device_listening',
  START_GAMEPAD_LISTENING: 'start_gamepad_listening',
  STOP_GAMEPAD_LISTENING: 'stop_gamepad_listening',
} as const

export const LANGUAGE = {
  ZH_CN: 'zh-CN',
  ZH_TW: 'zh-TW',
  EN_US: 'en-US',
  VI_VN: 'vi-VN',
  PT_BR: 'pt-BR',
} as const

export const WINDOW_LABEL = {
  MAIN: 'main',
  PREFERENCE: 'preference',
} as const
