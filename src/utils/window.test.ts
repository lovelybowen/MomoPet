import { describe, expect, it } from 'vitest'

import { clampWindowPosition, isPositionOnMonitor } from './window'

const primaryMonitor = {
  position: { x: 0, y: 0 },
  size: { width: 1920, height: 1080 },
}

describe('window placement', () => {
  it('clamps every window edge to the selected monitor', () => {
    const windowSize = { width: 360, height: 280 }

    expect(clampWindowPosition({ x: -20, y: -30 }, windowSize, primaryMonitor))
      .toEqual({ x: 0, y: 0 })
    expect(clampWindowPosition({ x: 1800, y: 1000 }, windowSize, primaryMonitor))
      .toEqual({ x: 1560, y: 800 })
  })

  it('recognizes positions on monitors with negative coordinates', () => {
    const secondaryMonitor = {
      position: { x: -2560, y: -300 },
      size: { width: 2560, height: 1440 },
    }

    expect(isPositionOnMonitor({ x: -1200, y: 400 }, secondaryMonitor)).toBe(true)
    expect(isPositionOnMonitor({ x: 120, y: 400 }, secondaryMonitor)).toBe(false)
  })
})
