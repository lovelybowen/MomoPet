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
    entryPath: '/models/com.example.pet/model/pet.sprite.json',
    runtimeType: 'sprite2d',
    actions: [
      {
        id: 'happy',
        name: 'Happy',
        type: 'animation',
        clip: 'happy',
        mode: 'once',
      },
    ],
    isBuiltin: false,
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
        entryPath: '/new/second/pet.sprite.json',
      }),
    ]
    const store = useModelStore()
    store.currentModel = createInstalledModel({
      id: installed[1].id,
      path: '/stale/path',
      entryPath: '/stale/path/pet.sprite.json',
    })
    invokeMock.mockResolvedValue(installed)

    await store.init()

    expect(store.currentModel).toEqual(installed[1])
  })

  it('uses generic pet settings without deprecated migration fields', () => {
    const state = usePetStore().$state

    expect(state).toHaveProperty('model.maxFPS', 60)
    expect(state).toHaveProperty('window.alwaysOnTop', true)
    expect(state).not.toHaveProperty('migrated')
    expect(state).not.toHaveProperty('mirrorMode')
    expect(state).not.toHaveProperty('penetrable')
  })
})
