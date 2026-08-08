import { invoke } from '@tauri-apps/api/core'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import type { InstalledModel } from './model'

import { useModelStore } from './model'
import { usePetStore } from './pet'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const invokeMock = vi.mocked(invoke)

describe('desktop pet stores', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    invokeMock.mockReset()
  })

  it('starts cleanly without an installed model', async () => {
    invokeMock.mockResolvedValue([])
    const store = useModelStore()

    await store.init()

    expect(store.models).toEqual([])
    expect(store.currentModel).toBeUndefined()
    expect(store.modelReady).toBe(true)
  })

  it('restores the selected model by stable ID using the controlled repository result', async () => {
    const installed: InstalledModel[] = [
      { id: 'a'.repeat(64), path: '/new/first', mode: 'standard', isBuiltin: false },
      { id: 'b'.repeat(64), path: '/new/second', mode: 'keyboard', isBuiltin: false },
    ]
    const store = useModelStore()
    store.currentModel = {
      id: installed[1].id,
      path: '/stale/path',
      mode: 'standard',
      isBuiltin: false,
    }
    invokeMock.mockResolvedValue(installed)

    await store.init()

    expect(store.currentModel).toEqual(installed[1])
  })

  it('uses generic pet settings without deprecated migration fields', () => {
    const state = usePetStore().$state

    expect(state).toHaveProperty('model.inputVisualizer', true)
    expect(state).toHaveProperty('window.alwaysOnTop', true)
    expect(state).not.toHaveProperty('migrated')
    expect(state).not.toHaveProperty('mirrorMode')
    expect(state).not.toHaveProperty('penetrable')
  })
})
