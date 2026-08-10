const DIGIT_KEYS = '1234567890'.split('') as readonly string[]
const LETTER_KEYS = 'QWERTYUIOPASDFGHJKLZXCVBNM'.split('') as readonly string[]

export function getActionShortcutId(petId: string, actionId: string) {
  return `${petId}:action:${actionId}`
}

export function getDefaultActionShortcut(index: number, primary: 'Command' | 'Control') {
  const modifierGroups = [
    [primary],
    [primary, 'Shift'],
    [primary, 'Alt'],
    [primary, 'Shift', 'Alt'],
  ]
  const tiers = [
    ...modifierGroups.map(modifiers => ({ modifiers, keys: DIGIT_KEYS })),
    ...modifierGroups.map(modifiers => ({ modifiers, keys: LETTER_KEYS })),
  ]
  let nextIndex = index

  for (const tier of tiers) {
    if (nextIndex < tier.keys.length) {
      return [...tier.modifiers, tier.keys[nextIndex]].join('+')
    }

    nextIndex -= tier.keys.length
  }

  return ''
}

export function ensureActionShortcuts(
  shortcuts: Record<string, string>,
  petId: string,
  actionIds: readonly string[],
  primary: 'Command' | 'Control',
) {
  const shortcutIds = actionIds.map(actionId => getActionShortcutId(petId, actionId))
  const usedShortcuts = new Set(shortcutIds.map(id => shortcuts[id]).filter(Boolean))
  let candidateIndex = 0

  for (const shortcutId of shortcutIds) {
    if (shortcuts[shortcutId]) continue

    let shortcut = ''
    while (!shortcut) {
      const candidate = getDefaultActionShortcut(candidateIndex, primary)
      candidateIndex += 1

      if (!candidate) return
      if (!usedShortcuts.has(candidate)) shortcut = candidate
    }

    shortcuts[shortcutId] = shortcut
    usedShortcuts.add(shortcut)
  }
}
