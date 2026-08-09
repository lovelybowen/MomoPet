import { describe, expect, it } from 'vitest'

import {
  ensureActionShortcuts,
  getActionShortcutId,
  migrateLegacyActionShortcuts,
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

  it('migrates legacy motion and expression shortcut keys', () => {
    const shortcuts: Record<string, string> = {
      'aabb:motion:Tap Body:2': 'Control+7',
      'aabb:expression:0': 'Control+8',
    }

    migrateLegacyActionShortcuts(shortcuts, 'aabb', [
      {
        id: 'legacy-motion-1',
        name: 'Tap',
        type: 'motion',
        motionGroup: 'Tap Body',
        motionIndex: 2,
      },
      {
        id: 'legacy-expression-1',
        name: 'Smile',
        type: 'expression',
        expression: 'smile',
      },
    ])

    expect(shortcuts['aabb:action:legacy-motion-1']).toBe('Control+7')
    expect(shortcuts['aabb:action:legacy-expression-1']).toBe('Control+8')
  })
})
