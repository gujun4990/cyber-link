export function applyStateRefresh(_current, snapshot) {
  return {
    device: snapshot,
    initFailed: false,
    actionFailed: _current.actionFailed,
    refreshFailed: false,
    refreshError: null,
  };
}
