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

function createInstalledModel(overrides: Partial<InstalledModel> = {}): InstalledModel {
  return {
    id: 'com.example.pet',
    version: '1.0.0',
    name: 'Example Pet',
    authors: [{ name: 'Example Author' }],
    path: '/models/com.example.pet',
    entryPath: '/models/com.example.pet/model.model3.json',
    resourcePath: '/models/com.example.pet/resources',
    mode: 'standard',
    actions: [
      {
        id: 'idle',
        name: 'Idle',
        type: 'motion',
        motionGroup: 'Idle',
        motionIndex: 0,
      },
    ],
    isBuiltin: false,
    isLegacy: false,
    ...overrides,
  }
}

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
      createInstalledModel({ id: 'com.example.first', path: '/new/first' }),
      createInstalledModel({
        id: 'com.example.second',
        path: '/new/second',
        entryPath: '/new/second/model.model3.json',
        resourcePath: '/new/second/resources',
        mode: 'keyboard',
      }),
    ]
    const store = useModelStore()
    store.currentModel = createInstalledModel({
      id: installed[1].id,
      path: '/stale/path',
      entryPath: '/stale/path/model.model3.json',
      resourcePath: '/stale/path/resources',
    })
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
