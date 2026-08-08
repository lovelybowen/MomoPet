export interface Point {
  x: number
  y: number
}

export interface Size {
  width: number
  height: number
}

export interface MonitorBounds {
  position: Point
  size: Size
}

export function clampWindowPosition(
  position: Point,
  windowSize: Size,
  monitor: MonitorBounds,
): Point {
  const minX = monitor.position.x
  const maxX = monitor.position.x + monitor.size.width - windowSize.width
  const minY = monitor.position.y
  const maxY = monitor.position.y + monitor.size.height - windowSize.height

  return {
    x: Math.max(minX, Math.min(position.x, maxX)),
    y: Math.max(minY, Math.min(position.y, maxY)),
  }
}

export function isPositionOnMonitor(position: Point, monitor: MonitorBounds) {
  const { x, y } = position
  const { position: monitorPosition, size } = monitor

  return x >= monitorPosition.x
    && x <= monitorPosition.x + size.width
    && y >= monitorPosition.y
    && y <= monitorPosition.y + size.height
}
