export function applyStateRefresh(_current, snapshot) {
  return {
    device: snapshot,
    initFailed: false,
    actionFailed: _current.actionFailed,
    refreshFailed: false,
    refreshError: null,
  };
}

export function shouldIgnoreRevertedActionRefresh(previous, next, action) {
  if (!action) return false;

  switch (action.action) {
    case 'ac_toggle':
      return isSameAcSnapshot(previous.ac, next.ac);
    case 'ac_set_temp':
      return isSameAcSnapshot(previous.ac, next.ac);
    case 'switch_toggle':
      switch (action.target) {
        case 'ambientLight':
          return next.ambientLightOn === previous.ambientLightOn;
        case 'mainLight':
          return next.mainLightOn === previous.mainLightOn;
        case 'doorSignLight':
          return next.doorSignLightOn === previous.doorSignLightOn;
        default:
          return false;
      }
    default:
      return false;
  }
}

function isSameAcSnapshot(previous, next) {
  return (
    next.isOn === previous.isOn &&
    next.temp === previous.temp &&
    next.minTemp === previous.minTemp &&
    next.maxTemp === previous.maxTemp &&
    next.targetTempStep === previous.targetTempStep &&
    next.temperatureUnit === previous.temperatureUnit &&
    next.unitOfMeasurement === previous.unitOfMeasurement
  );
}
