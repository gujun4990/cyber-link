export const ACTIONS = {
  acToggle: 'ac_toggle',
  acSetTemp: 'ac_set_temp',
  switchToggle: 'switch_toggle',
  startup: 'startup_online',
  shutdown: 'shutdown_signal',
} as const;

export type AppActionName = keyof typeof ACTIONS;

export function clampTemp(current: number, delta: number) {
  return Math.min(30, Math.max(16, current + delta));
}

export function buildActionPayload(kind: 'startup' | 'shutdown') {
  return {
    action: kind === 'startup' ? ACTIONS.startup : ACTIONS.shutdown,
  };
}
