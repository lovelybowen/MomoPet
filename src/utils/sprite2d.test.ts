import { describe, expect, it } from 'vitest'

import type { PetAction } from '@/stores/model'

import { resolveActionClip, withLoadTimeout } from './sprite2d'

const happy: PetAction = {
  id: 'happy',
  name: 'Happy',
  type: 'animation',
  clip: 'happy',
  mode: 'once',
}

const sleep: PetAction = {
  id: 'sleep',
  name: 'Sleep',
  type: 'animation',
  clip: 'sleep',
  mode: 'toggle',
}

describe('sprite2d action state', () => {
  it('always starts a one-shot action from its declared clip', () => {
    expect(resolveActionClip('idle', happy)).toBe('happy')
    expect(resolveActionClip('happy', happy)).toBe('happy')
  })

  it('returns an active toggle action to idle on its second trigger', () => {
    expect(resolveActionClip('idle', sleep)).toBe('sleep')
    expect(resolveActionClip('sleep', sleep)).toBe('idle')
  })
})

describe('sprite2d load timeout', () => {
  it('returns the operation result before the deadline', async () => {
    await expect(
      withLoadTimeout(Promise.resolve('ready'), 'test', 100),
    )
      .resolves
      .toBe('ready')
  })

  it('rejects a stalled operation with its stage', async () => {
    const stalled = new Promise<never>(() => {})

    await expect(
      withLoadTimeout(stalled, 'sprite sheet decode', 1),
    )
      .rejects
      .toThrow('Timed out during sprite sheet decode')
  })
})
