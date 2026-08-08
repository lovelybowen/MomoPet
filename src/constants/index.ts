export const LISTEN_KEY = {
  SHOW_WINDOW: 'show-window',
  HIDE_WINDOW: 'hide-window',
  DEVICE_CHANGED: 'device-changed',
  GAMEPAD_CHANGED: 'gamepad-changed',
  START_MOTION: 'start-motion',
  SET_EXPRESSION: 'set-expression',
} as const

export const INVOKE_KEY = {
  IMPORT_MODEL: 'import_live2d_model',
  LIST_MODELS: 'list_installed_live2d_models',
  REMOVE_MODEL: 'remove_live2d_model',
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
