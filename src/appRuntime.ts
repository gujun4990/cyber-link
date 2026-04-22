import type { RuntimeMode } from './runtimeMode';
import type { LightingKind } from './lightingCards';

export interface ACState {
  isOn: boolean;
  temp: number;
}

export interface DeviceState {
  room: string;
  pcId: string;
  ac: ACState;
  switchOn: boolean;
  ambientLightOn: boolean;
  mainLightOn: boolean;
  doorSignLightOn: boolean;
  acAvailable: boolean;
  switchAvailable: boolean;
  ambientLightAvailable: boolean;
  mainLightAvailable: boolean;
  doorSignLightAvailable: boolean;
  lightCount: number;
  connected: boolean;
  initError?: string;
}

export type ActionName =
  | 'ac_toggle'
  | 'ac_set_temp'
  | 'switch_toggle'
  | 'startup_online'
  | 'shutdown_signal';

export interface AppRuntime {
  mode: RuntimeMode;
  initializeApp: () => Promise<DeviceState>;
  refreshHaState: () => Promise<DeviceState>;
  handleHaAction: (
    action: ActionName,
    target?: LightingKind,
    value?: number,
  ) => Promise<DeviceState>;
  subscribeStateRefresh: (handler: (state: DeviceState) => void) => Promise<() => void>;
  isAutostartMode: () => Promise<boolean>;
  setWindowSize: (width: number, height: number) => Promise<void>;
  showWindow: () => Promise<void>;
  hideWindow: () => Promise<void>;
  minimizeWindow: () => Promise<void>;
  startDragging: () => Promise<void>;
  appendLogMessage: (message: string) => Promise<void>;
}

function cloneState(state: DeviceState): DeviceState {
  return JSON.parse(JSON.stringify(state)) as DeviceState;
}

function withAmbientLightAliases(state: DeviceState): DeviceState {
  return {
    ...state,
    ambientLightOn: state.switchOn,
    ambientLightAvailable: state.switchAvailable,
  };
}

function createMockLiveState(): DeviceState {
  return {
    room: '核心-01',
    pcId: '终端-05',
    ac: { isOn: true, temp: 24 },
    switchOn: false,
    ambientLightOn: false,
    mainLightOn: true,
    doorSignLightOn: true,
    acAvailable: true,
    switchAvailable: true,
    ambientLightAvailable: true,
    mainLightAvailable: true,
    doorSignLightAvailable: true,
    lightCount: 3,
    connected: true,
  };
}

function createMockOfflineState(): DeviceState {
  return {
    room: '核心-01',
    pcId: '终端-05',
    ac: { isOn: false, temp: 24 },
    switchOn: false,
    ambientLightOn: false,
    mainLightOn: false,
    doorSignLightOn: false,
    acAvailable: false,
    switchAvailable: false,
    ambientLightAvailable: false,
    mainLightAvailable: false,
    doorSignLightAvailable: false,
    lightCount: 3,
    connected: false,
  };
}

function applyMockAction(
  state: DeviceState,
  action: ActionName,
  target?: LightingKind,
  value?: number,
): DeviceState {
  const next = cloneState(state);

  switch (action) {
    case 'ac_toggle':
      next.ac.isOn = !next.ac.isOn;
      return next;
    case 'ac_set_temp':
      next.ac.isOn = true;
      next.ac.temp = Math.min(30, Math.max(16, value ?? next.ac.temp));
      return next;
    case 'switch_toggle':
      switch (target) {
        case 'ambientLight':
          next.switchOn = !next.switchOn;
          break;
        case 'mainLight':
          next.mainLightOn = !next.mainLightOn;
          break;
        case 'doorSignLight':
          next.doorSignLightOn = !next.doorSignLightOn;
          break;
        default:
          throw new Error('missing light target');
      }
      return next;
    case 'startup_online':
      next.connected = true;
      next.ac.isOn = true;
      next.switchOn = true;
      next.mainLightOn = true;
      next.doorSignLightOn = true;
      return next;
    case 'shutdown_signal':
      next.ac.isOn = false;
      next.switchOn = false;
      next.mainLightOn = false;
      next.doorSignLightOn = false;
      return next;
  }
}

function createMockRuntime(): AppRuntime {
  let state = createMockLiveState();
  const listeners = new Set<(snapshot: DeviceState) => void>();

  const emit = () => {
    const snapshot = withAmbientLightAliases(cloneState(state));
    for (const listener of listeners) {
      listener(snapshot);
    }
    return snapshot;
  };

  return {
    mode: 'mock',
    initializeApp: async () => {
      const offlineSnapshot = withAmbientLightAliases(createMockOfflineState());
      state = createMockLiveState();
      queueMicrotask(() => {
        emit();
      });
      return offlineSnapshot;
    },
    refreshHaState: async () => emit(),
    handleHaAction: async (action, target, value) => {
      state = applyMockAction(state, action, target, value);
      return emit();
    },
    subscribeStateRefresh: async (handler) => {
      listeners.add(handler);
      return () => {
        listeners.delete(handler);
      };
    },
    isAutostartMode: async () => false,
    setWindowSize: async () => {},
    showWindow: async () => {},
    hideWindow: async () => {},
    minimizeWindow: async () => {},
    startDragging: async () => {},
    appendLogMessage: async () => {},
  };
}

function createTauriRuntime(): AppRuntime {
  let tauriApiPromise: Promise<{
    invoke: <T>(command: string, payload?: Record<string, unknown>) => Promise<T>;
    listen: <T>(event: string, handler: (event: { payload: T }) => void) => Promise<() => void>;
    appWindow: {
      setSize: (size: { width: number; height: number }) => Promise<void>;
      show: () => Promise<void>;
      hide: () => Promise<void>;
      minimize: () => Promise<void>;
      startDragging: () => Promise<void>;
    };
    LogicalSize: new (width: number, height: number) => { width: number; height: number };
  }> | null = null;

  const loadTauriApi = async () => {
    if (!tauriApiPromise) {
      tauriApiPromise = Promise.all([
        import('@tauri-apps/api/tauri'),
        import('@tauri-apps/api/event'),
        import('@tauri-apps/api/window'),
      ]).then(([tauri, event, window]) => ({
        invoke: tauri.invoke,
        listen: event.listen,
        appWindow: window.appWindow,
        LogicalSize: window.LogicalSize,
      }));
    }

    return tauriApiPromise;
  };

    return {
      mode: 'tauri',
      initializeApp: async () => {
        const { invoke } = await loadTauriApi();
      return withAmbientLightAliases(await invoke<DeviceState>('initialize_app'));
      },
      refreshHaState: async () => {
        const { invoke } = await loadTauriApi();
      return withAmbientLightAliases(await invoke<DeviceState>('refresh_ha_state'));
      },
      handleHaAction: async (action, target, value) => {
        const { invoke } = await loadTauriApi();
       return withAmbientLightAliases(
         await invoke<DeviceState>(
           'handle_ha_action',
           value === undefined ? { action, target } : { action, target, value },
         ),
       );
      },
      subscribeStateRefresh: async (handler) => {
        const { listen } = await loadTauriApi();
      return listen<DeviceState>('state-refresh', (event) => handler(withAmbientLightAliases(event.payload)));
      },
    isAutostartMode: async () => {
      const { invoke } = await loadTauriApi();
      return invoke<boolean>('is_autostart_mode');
    },
    setWindowSize: async (width, height) => {
      const { appWindow, LogicalSize } = await loadTauriApi();
      await appWindow.setSize(new LogicalSize(width, height));
    },
    showWindow: async () => {
      const { appWindow } = await loadTauriApi();
      await appWindow.show();
    },
    hideWindow: async () => {
      const { appWindow } = await loadTauriApi();
      await appWindow.hide();
    },
    minimizeWindow: async () => {
      const { appWindow } = await loadTauriApi();
      await appWindow.minimize();
    },
    startDragging: async () => {
      const { appWindow } = await loadTauriApi();
      await appWindow.startDragging();
    },
    appendLogMessage: async (message) => {
      const { invoke } = await loadTauriApi();
      await invoke('append_log_message', { message });
    },
  };
}

export function createAppRuntime(options: { mode?: RuntimeMode; hasTauriBridge?: boolean } = {}): AppRuntime {
  const mode = options.mode ?? 'tauri';
  const hasTauriBridge = options.hasTauriBridge ?? (typeof window !== 'undefined' && '__TAURI__' in window);

  if (mode === 'mock') {
    return createMockRuntime();
  }

  if (!hasTauriBridge) {
    return createTauriRuntime();
  }

  return createTauriRuntime();
}
