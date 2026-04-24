export function shouldPauseUi(hidden: boolean) {
  return hidden;
}

export function shouldRefreshOnResume(options: {
  wasPaused: boolean;
  isPaused: boolean;
  hasLoadedState: boolean;
}) {
  return options.wasPaused && !options.isPaused && options.hasLoadedState;
}
