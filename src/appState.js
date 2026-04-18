export function applyStateRefresh(_current, snapshot) {
  return {
    device: snapshot,
    initFailed: false,
    actionFailed: false,
    refreshFailed: false,
    refreshError: null,
  };
}
