import { describe, expect, it } from 'vitest'

import { mapAxisParameter, mapPointerParameter } from './profile'

describe('input visualizer parameter mapping', () => {
  it('maps pointer axes and mirrors horizontal parameters', () => {
    const range = { min: -30, max: 30 }

    expect(mapPointerParameter('ParamAngleX', range, 0.25, 0.75, false)).toBe(15)
    expect(mapPointerParameter('ParamAngleX', range, 0.25, 0.75, true)).toBe(-15)
    expect(mapPointerParameter('ParamAngleY', range, 0.25, 0.75, true)).toBe(-15)
  })

  it('maps diagonal rotation and clamps gamepad axes', () => {
    const range = { min: -30, max: 30 }

    expect(mapPointerParameter('ParamAngleZ', range, 0, 1, false)).toBe(30)
    expect(mapAxisParameter(range, 2)).toBe(30)
    expect(mapAxisParameter(range, -2)).toBe(-30)
  })
})
