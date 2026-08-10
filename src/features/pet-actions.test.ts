import { describe, expect, it } from 'vitest'

import {
  ensureActionShortcuts,
  getActionShortcutId,
} from './pet-actions'

describe('pet action identity', () => {
  it('uses stable pet and action IDs without array positions', () => {
    expect(getActionShortcutId('com.example.momo', 'wave'))
      .toBe('com.example.momo:action:wave')
  })

  it('keeps existing shortcuts when an action is inserted during an upgrade', () => {
    const shortcuts = {
      'com.example.momo:action:idle': 'Control+1',
      'com.example.momo:action:wave': 'Control+2',
    }

    ensureActionShortcuts(
      shortcuts,
      'com.example.momo',
      ['idle', 'smile', 'wave'],
      'Control',
    )

    expect(shortcuts).toEqual({
      'com.example.momo:action:idle': 'Control+1',
      'com.example.momo:action:smile': 'Control+3',
      'com.example.momo:action:wave': 'Control+2',
    })
  })
})
