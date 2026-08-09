export interface ParameterRange {
  min: number
  max: number
}

export interface InputVisualizerProfile {
  hands?: {
    left: string
    right: string
  }
  mouseButtons?: Record<string, string>
  pointer?: readonly string[]
  gamepad?: {
    axes?: Record<string, string>
    thumbButtons?: Record<string, string>
    stickHands?: {
      left: string
      right: string
    }
  }
}

export const DEFAULT_INPUT_VISUALIZER_PROFILE: InputVisualizerProfile = {
  hands: {
    left: 'CatParamLeftHandDown',
    right: 'CatParamRightHandDown',
  },
  mouseButtons: {
    Left: 'ParamMouseLeftDown',
    Right: 'ParamMouseRightDown',
  },
  pointer: [
    'ParamMouseX',
    'ParamMouseY',
    'ParamAngleX',
    'ParamAngleY',
    'ParamAngleZ',
    'ParamEyeBallX',
    'ParamEyeBallY',
  ],
  gamepad: {
    axes: {
      LeftStickX: 'CatParamStickLX',
      LeftStickY: 'CatParamStickLY',
      RightStickX: 'CatParamStickRX',
      RightStickY: 'CatParamStickRY',
    },
    thumbButtons: {
      LeftThumb: 'CatParamStickLeftDown',
      RightThumb: 'CatParamStickRightDown',
    },
    stickHands: {
      left: 'CatParamStickShowLeftHand',
      right: 'CatParamStickShowRightHand',
    },
  },
}

export function mapPointerParameter(
  parameterId: string,
  range: ParameterRange,
  xRatio: number,
  yRatio: number,
  mirrorX: boolean,
) {
  const isXAxis = parameterId.endsWith('X')
  const isYAxis = parameterId.endsWith('Y')
  const isZAxis = parameterId.endsWith('Z')
  let value: number

  if (isZAxis) {
    const dragX = 1 - 2 * xRatio
    const dragY = 1 - 2 * yRatio
    value = dragX * dragY * range.min
  } else {
    const ratio = isXAxis ? xRatio : yRatio
    value = range.max - ratio * (range.max - range.min)
  }

  return mirrorX && !isYAxis ? value * -1 : value
}

export function mapAxisParameter(range: ParameterRange, value: number) {
  return Math.max(range.min, Math.min(range.max, value * range.max))
}
