import React, { memo, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';
import { appWindow, LogicalSize } from '@tauri-apps/api/window';
import {
  ChevronDown,
  ChevronUp,
  Fan,
  Flame,
  Lightbulb,
  Minus,
  Monitor,
  RefreshCw,
  Snowflake,
  X,
} from 'lucide-react';
import { motion } from 'motion/react';
import { ACTIONS, clampTemp } from './haActions';
import { applyStateRefresh } from './appState.js';
import { withTimeout } from './initTimeout.js';
import windowSize from './shared/windowSize.json';

const consoleLogLevels = ['warn', 'error'] as const;
type ConsoleLogLevel = (typeof consoleLogLevels)[number];
type ConsoleLogLevelName = Uppercase<ConsoleLogLevel>;

function describeError(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function formatLogLine(level: ConsoleLogLevelName, message: string) {
  return `${new Date().toISOString()} [${level}] ${message}`;
}

interface ACState {
  isOn: boolean;
  temp: number;
}

interface DeviceState {
  room: string;
  pcId: string;
  ac: ACState;
  switchOn: boolean;
  acAvailable: boolean;
  switchAvailable: boolean;
  connected: boolean;
  initError?: string;
}

const innerRingSpinTransition = { duration: 34, repeat: Infinity, ease: 'linear' };
const shimmerTransition = { duration: 7, repeat: Infinity, ease: 'linear' };
const toggleShimmerTransition = { duration: 8, repeat: Infinity, ease: 'linear' };
const tickerTransition = { duration: 60, repeat: Infinity, ease: 'linear' };

const tickerLines = Array.from(
  { length: 10 },
  (_, i) => `SYSTEM_CORE_LOAD: ${35 + ((i * 9) % 61)}% - TEMP_STABLE`,
);

function useLatestRef<T>(value: T) {
  const ref = useRef(value);
  useEffect(() => {
    ref.current = value;
  }, [value]);
  return ref;
}

const ClockText = memo(function ClockText() {
  const [currentTime, setCurrentTime] = useState(() => new Date());

  useEffect(() => {
    const timer = window.setInterval(() => {
      setCurrentTime(new Date());
    }, 1000);
    return () => window.clearInterval(timer);
  }, []);

  return (
    <span className="font-mono">
      {currentTime.toLocaleTimeString('zh-CN', { hour12: false })}
    </span>
  );
});

const StatusTicker = memo(function StatusTicker() {
  return (
    <div className="absolute top-0 left-0 w-full h-[1px] pointer-events-none opacity-20">
      <div className="w-full h-full bg-gradient-to-r from-transparent via-cyan-300/50 to-transparent" />
    </div>
  );
});

const TempCore = memo(function TempCore({
  temp,
  tempDisplayOn,
  onDecrease,
  onIncrease,
  disabled,
}: {
  temp: number;
  tempDisplayOn: boolean;
  onDecrease: () => void;
  onIncrease: () => void;
  disabled: boolean;
}) {
  return (
    <div className="flex items-center gap-2">
      <motion.button
        whileHover={tempDisplayOn ? { scale: 1.05 } : {}}
        whileTap={tempDisplayOn ? { scale: 0.95 } : {}}
        onClick={onDecrease}
        disabled={disabled}
        className={`p-2 transition-all rounded-full border border-transparent ${
          tempDisplayOn
            ? 'text-cyan-200/90 hover:border-cyan-300/35 hover:bg-cyan-400/8 cursor-pointer'
            : 'text-cyan-950/20 cursor-not-allowed'
        }`}
      >
        <ChevronDown size={36} />
      </motion.button>

      <div className="flex flex-col items-center min-w-[140px] relative">
        <div
          className={`absolute inset-0 rounded-full transition-all duration-700 ${
            tempDisplayOn ? 'bg-cyan-400/16 scale-102 blur-[14px]' : 'bg-transparent'
          }`}
        />
        <div
          className={`absolute inset-0 rounded-full transition-all duration-700 ${
            tempDisplayOn ? 'bg-purple-500/4 scale-105 blur-[8px]' : 'bg-transparent'
          }`}
        />

        <motion.span
          key={temp}
          initial={{ opacity: 0, scale: 0.92 }}
          animate={{ opacity: 1, scale: 1 }}
          className={`text-[9rem] font-black tabular-nums transition-all duration-500 relative z-20 leading-[1.1] ${
            tempDisplayOn ? 'text-iridescent' : 'text-white/10'
          }`}
          style={{
            textShadow: tempDisplayOn
              ? '0 0 12px rgba(6,182,212,0.6)'
              : '0 0 4px rgba(255,255,255,0.05)',
            transform: 'translateZ(0)',
            willChange: 'transform, opacity',
          }}
        >
          {temp}
        </motion.span>
      </div>

      <motion.button
        whileHover={tempDisplayOn ? { scale: 1.05 } : {}}
        whileTap={tempDisplayOn ? { scale: 0.95 } : {}}
        onClick={onIncrease}
        disabled={disabled}
        className={`p-2 transition-all rounded-full border border-transparent ${
          tempDisplayOn
            ? 'text-cyan-200/90 hover:border-cyan-300/35 hover:bg-cyan-400/8 cursor-pointer'
            : 'text-cyan-950/20 cursor-not-allowed'
        }`}
      >
        <ChevronUp size={36} />
      </motion.button>
    </div>
  );
});

const TechToggle = memo(function TechToggle({
  active,
  onClick,
  disabled,
  label,
  subLabel,
  icon,
}: {
  active: boolean;
  onClick: () => void;
  disabled?: boolean;
  label: string;
  subLabel: string;
  icon: React.ReactNode;
}) {
  return (
    <motion.button
      whileHover={disabled ? {} : { scale: 1.012, x: 2 }}
      whileTap={disabled ? {} : { scale: 0.985 }}
      onClick={onClick}
      disabled={disabled}
      className={`relative w-full p-4 rounded-xl border ring-1 ring-white/5 transition-all duration-500 flex items-center gap-5 overflow-hidden group ${
        disabled ? 'opacity-60 cursor-not-allowed' : 'cursor-pointer'
      } ${
        active
          ? 'bg-cyan-400/40 border-cyan-100 text-white shadow-[0_0_12px_rgba(6,182,212,0.35)]'
          : 'bg-[#2a3b7d]/78 border-white/18 text-white/50 shadow-inner ring-white/10'
      }`}
      style={{ transform: 'translateZ(0)' }}
    >
      <div className="absolute inset-0 bg-carbon mix-blend-overlay opacity-5 pointer-events-none" />

      <div
        className={`p-3 rounded-lg border transition-all duration-500 relative overflow-hidden ${
          active
            ? 'border-cyan-100 bg-cyan-400/40 shadow-[0_0_12px_rgba(6,182,212,0.35)]'
            : 'border-white/10 bg-white/5 opacity-50'
        }`}
      >
        <div className={active && label.includes('空调') ? 'animate-spin opacity-90' : ''}>{icon}</div>
      </div>

      <div className="flex-1 text-left relative z-10">
        <div
          className={`text-[14px] font-black tracking-[0.1em] antialiased transition-colors duration-300 ${
            active ? 'text-white' : 'text-white/70'
          }`}
        >
          {label}
        </div>
        <div
          className={`text-[10px] font-bold mt-0.5 transition-colors duration-300 ${
            active ? 'text-cyan-50' : 'text-white/60'
          }`}
        >
          {subLabel}
        </div>
      </div>

      <div
        className={`w-3 h-3 rounded-full transition-all duration-500 ${
          active
            ? 'bg-cyan-100 shadow-[0_0_12px_rgba(6,182,212,0.35)]'
            : 'bg-black/40 border border-white/10'
        }`}
      />
    </motion.button>
  );
});

export default function App() {
  const [device, setDevice] = useState<DeviceState>({
    room: '核心-01',
    pcId: '终端-05',
    ac: { isOn: true, temp: 16 },
    switchOn: false,
    acAvailable: true,
    switchAvailable: true,
    connected: true,
  });

  const [initFailed, setInitFailed] = useState(false);
  const [actionFailed, setActionFailed] = useState(false);
  const [refreshFailed, setRefreshFailed] = useState(false);
  const [refreshError, setRefreshError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [syncingAction, setSyncingAction] = useState(false);
  const [hasLoadedState, setHasLoadedState] = useState(false);

  const syncingRef = useRef(false);
  const consoleFallbackRef = useRef<Pick<Console, 'error' | 'warn'>>({
    error: console.error.bind(console),
    warn: console.warn.bind(console),
  });

  const latestStateRef = useLatestRef({
    device,
    initFailed,
    actionFailed,
    refreshFailed,
    refreshError,
  });

  const logMessage = useCallback(async (message: string) => {
    try {
      await invoke('append_log_message', { message });
    } catch (error) {
      consoleFallbackRef.current.error('Failed to write app log', error);
    }
  }, []);

  const reportError = useCallback(
    async (message: string, error: unknown) => {
      const line = formatLogLine('ERROR', `${message}: ${describeError(error)}`);
      await logMessage(line);
      consoleFallbackRef.current.error(line);
    },
    [logMessage],
  );

  useEffect(() => {
    const patchedConsole = console as typeof console & Record<
      ConsoleLogLevel,
      (...args: unknown[]) => void
    >;
    const originals = new Map<ConsoleLogLevel, (...args: unknown[]) => void>();

    for (const level of consoleLogLevels) {
      const original = console[level].bind(console) as (...args: unknown[]) => void;
      originals.set(level, original);
      patchedConsole[level] = (...args: unknown[]) => {
        original(...args);
        void logMessage(
          formatLogLine(level.toUpperCase() as ConsoleLogLevelName, args.map(String).join(' ')),
        );
      };
    }

    return () => {
      for (const [level, original] of originals) {
        patchedConsole[level] = original;
      }
    };
  }, [logMessage]);

  useEffect(() => {
    let disposed = false;
    let unlisten: null | (() => void) = null;

    void (async () => {
      const autostartMode = await invoke<boolean>('is_autostart_mode');

      if (disposed) {
        return;
      }

      unlisten = await listen<DeviceState>('state-refresh', (event) => {
        if (!event.payload) return;

        const latest = latestStateRef.current;
        const next = applyStateRefresh(
          {
            device: latest.device,
            initFailed: latest.initFailed,
            actionFailed: latest.actionFailed,
            refreshFailed: latest.refreshFailed,
            refreshError: latest.refreshError,
          },
          event.payload,
        );

        setDevice(next.device);
        setInitFailed(next.initFailed);
        setActionFailed(next.actionFailed);
        setRefreshFailed(next.refreshFailed);
        setRefreshError(next.refreshError);
        setHasLoadedState(true);
      });

      if (disposed && unlisten) {
        unlisten();
        unlisten = null;
        return;
      }

      await appWindow.setSize(new LogicalSize(windowSize.width, windowSize.height));
      if (!autostartMode) {
        await appWindow.show();
      }

      try {
        await withTimeout(
          invoke<DeviceState>('initialize_app'),
          8000,
          'initialize_app timed out',
        );
        setHasLoadedState(true);
      } catch (error) {
        void reportError('Failed to initialize Tauri bridge', error);
        const msg = describeError(error);
        setInitFailed(true);
        setDevice((prev) => ({ ...prev, connected: false, initError: msg }));
        setHasLoadedState(false);
      }
    })();

    return () => {
      disposed = true;
      if (unlisten) unlisten();
    };
  }, [latestStateRef, reportError]);

  const syncDevice = useCallback(
    async (action: string, value?: number) => {
      if (syncingRef.current) return;

      syncingRef.current = true;
      setSyncingAction(true);
      try {
        const payload = value === undefined ? { action } : { action, value };
        await invoke<DeviceState>('handle_ha_action', payload);
        setActionFailed(false);
      } catch (error) {
        void reportError('Failed to sync device action', error);
        setActionFailed(true);
      } finally {
        syncingRef.current = false;
        setSyncingAction(false);
      }
    },
    [reportError],
  );

  const toggleAC = useCallback(() => {
    if (!hasLoadedState || syncingAction || !device.acAvailable) return;
    void syncDevice(ACTIONS.acToggle);
  }, [device.acAvailable, hasLoadedState, syncingAction, syncDevice]);

  const adjustTemp = useCallback(
    async (delta: number) => {
      if (!hasLoadedState || syncingAction || !device.ac.isOn || !device.acAvailable) return;
      await syncDevice(ACTIONS.acSetTemp, clampTemp(device.ac.temp, delta));
    },
    [device.ac.isOn, device.ac.temp, device.acAvailable, hasLoadedState, syncingAction, syncDevice],
  );

  const toggleSwitch = useCallback(() => {
    if (!hasLoadedState || syncingAction || !device.switchAvailable) return;
    void syncDevice(ACTIONS.switchToggle);
  }, [device.switchAvailable, hasLoadedState, syncingAction, syncDevice]);

  const refreshHaState = useCallback(async () => {
    if (refreshing) return;

    setRefreshing(true);
    try {
      await withTimeout(invoke<DeviceState>('refresh_ha_state'), 8000, 'refresh_ha_state timed out');
      setHasLoadedState(true);
      setActionFailed(false);
    } catch (error) {
      void reportError('Failed to refresh HA state', error);
      setRefreshFailed(true);
      setRefreshError(describeError(error));
    } finally {
      setRefreshing(false);
    }
  }, [refreshing, reportError]);

  const hideWindow = useCallback(async () => {
    try {
      await appWindow.hide();
    } catch (error) {
      void reportError('Failed to hide window', error);
    }
  }, [reportError]);

  const minimizeWindow = useCallback(async () => {
    try {
      await appWindow.hide();
    } catch (error) {
      void reportError('Failed to hide window', error);
    }
  }, [reportError]);

  const dragTopBar = useCallback(
    async (event: React.MouseEvent<HTMLDivElement>) => {
      if (event.button !== 0 || event.target !== event.currentTarget) return;
      try {
        await appWindow.startDragging();
      } catch (error) {
        void reportError('Failed to drag window', error);
      }
    },
    [reportError],
  );

  // 底栏状态文案集中计算，避免 JSX 里堆太多条件分支。
  const statusLabel = initFailed
    ? '离线模式'
    : !hasLoadedState
      ? '系统初始化中'
    : actionFailed
      ? '操作失败'
      : refreshFailed
        ? '刷新失败'
        : '系统稳定';

  const acDisplayOn = hasLoadedState && device.connected && device.ac.isOn;
  const switchDisplayOn = hasLoadedState && device.connected && device.switchOn;
  const coolingModeActive = hasLoadedState && device.connected && device.ac.isOn && device.ac.temp < 20;
  const heatingModeActive = hasLoadedState && device.connected && device.ac.isOn && device.ac.temp > 26;
  const tempDisplayOn = hasLoadedState && device.connected && device.ac.isOn;

  const tempDisabled =
    !hasLoadedState || syncingAction || !device.connected || !device.acAvailable || !device.ac.isOn;

  const appShellStyle = useMemo(
    () => ({
      width: windowSize.width,
      height: windowSize.height,
      background: `
        radial-gradient(circle at 20% 20%, rgba(70, 110, 255, 0.5), transparent 55%),
        linear-gradient(135deg, rgba(28, 48, 118, 0.97), rgba(15, 25, 70, 1))
      `,
      backdropFilter: 'blur(6px) saturate(120%)',
      transform: 'translateZ(0)',
      willChange: 'transform',
    }),
    [],
  );

  return (
    <motion.div
      layoutId="main-dashboard"
      initial={{ opacity: 0, scale: 0.97, y: 10 }}
      animate={{ opacity: 1, scale: 1, y: 0 }}
      className="fixed inset-0 m-auto border-[1.5px] border-white/18 overflow-hidden flex flex-col shadow-[0_18px_48px_rgba(0,0,0,0.5),0_0_16px_rgba(6,182,212,0.14)] antialiased"
      style={appShellStyle}
    >
      <div className="absolute inset-0 rounded-xl border border-white/8 pointer-events-none z-50" />
      <div className="absolute inset-0 bg-carbon mix-blend-soft-light opacity-5 pointer-events-none" />

      <div
        className="relative z-[70] flex items-center justify-between px-4 py-3 bg-black/20 border-b border-white/10 backdrop-blur-sm select-none"
        onMouseDown={(event) => {
          void dragTopBar(event);
        }}
        onDoubleClickCapture={(event) => {
          event.preventDefault();
          event.stopPropagation();
        }}
      >
        <div className="flex items-center gap-2.5">
          <div className="w-6 h-6 flex items-center justify-center bg-cyan-500/30 border border-cyan-400/40 rounded shadow-[0_0_8px_rgba(6,182,212,0.35)]">
            <Monitor size={14} className="text-cyan-300" />
          </div>
          <span className="text-[10px] font-black tracking-widest text-white/70 uppercase">
            Cyber Link v1.0
          </span>
        </div>

        <div className="flex gap-2 items-center">
          <motion.button
            whileHover={{ scale: 1.05, color: '#22d3ee' }}
            whileTap={{ scale: 0.95 }}
            onClick={() => {
              void refreshHaState();
            }}
            disabled={refreshing}
            className={`w-7 h-7 flex items-center justify-center text-white/60 hover:text-cyan-300 transition-colors ${
              refreshing ? 'opacity-60 cursor-not-allowed' : 'cursor-pointer'
            }`}
            title="刷新"
          >
            <RefreshCw size={14} strokeWidth={2} className={refreshing ? 'animate-spin' : ''} />
          </motion.button>

          <div className="w-px h-4 bg-white/15 mx-1" />

          <button
            onClick={() => {
              void minimizeWindow();
            }}
            className="w-8 h-8 flex items-center justify-center text-white/60 hover:bg-white/10 hover:text-white rounded transition-all"
          >
            <Minus size={16} strokeWidth={2} />
          </button>

          <button
            onClick={() => {
              void hideWindow();
            }}
            className="w-8 h-8 flex items-center justify-center text-white/60 hover:bg-rose-500 hover:text-white rounded transition-all"
          >
            <X size={16} strokeWidth={2} />
          </button>
        </div>
      </div>

      <div className="relative flex-1 flex flex-col overflow-hidden">
        <div className="absolute top-0 inset-x-0 h-px bg-gradient-to-r from-transparent via-cyan-400/55 to-transparent pointer-events-none" />

        <div className="relative flex-1 flex items-center justify-around px-12 py-4 overflow-visible">
          <div className="relative flex items-center justify-center w-[360px] h-[360px]">
            <div className="absolute w-[320px] h-[320px] border border-dashed border-cyan-500/18 rounded-full opacity-80" />

            <motion.div
              className="absolute w-[280px] h-[280px] border border-cyan-400/18 rounded-full border-t-transparent border-b-transparent transform-gpu will-change-transform"
              animate={{ rotate: -360 }}
              transition={innerRingSpinTransition}
              style={{ transform: 'translateZ(0)' }}
            />

            <div className="absolute w-[240px] h-[240px] border-2 border-cyan-500/8 rounded-full shadow-[inset_0_0_14px_rgba(6,182,212,0.05)]" />

            <div className="relative z-10 flex flex-col items-center justify-center h-full w-full">
              <div className="absolute top-4 flex flex-col items-center gap-1">
                <span className="text-[11px] font-black tracking-[0.45em] text-white uppercase">
                  空调控制系统
                </span>

                <div className="flex gap-3 mt-1.5">
                  <div
                    className={`group relative flex items-center gap-2 px-3 py-1.5 rounded-lg border transition-all duration-500 overflow-hidden ${
                      coolingModeActive
                        ? 'border-cyan-100 bg-cyan-400/40 shadow-[0_0_12px_rgba(6,182,212,0.35)]'
                        : 'border-white/6 bg-white/[0.02]'
                    }`}
                  >
                    <div className="absolute inset-0 circuit-pattern opacity-10 pointer-events-none" />
                    <Snowflake size={12} className={coolingModeActive ? 'text-cyan-100' : 'text-white/25'} />
                    <span
                      className={`text-[10px] font-black tracking-[0.18em] z-10 transition-colors duration-300 ${
                        coolingModeActive ? 'text-cyan-50' : 'text-white/22'
                      }`}
                    >
                      制冷模式
                    </span>
                  </div>

                  <div
                    className={`group relative flex items-center gap-2 px-3 py-1.5 rounded-lg border transition-all duration-500 overflow-hidden ${
                      heatingModeActive
                        ? 'border-orange-300 bg-orange-500/30 shadow-[0_0_12px_rgba(249,115,22,0.3)]'
                        : 'border-white/6 bg-white/[0.02]'
                    }`}
                  >
                    <Flame size={12} className={heatingModeActive ? 'text-orange-300' : 'text-white/25'} />
                    <span
                      className={`text-[10px] font-black tracking-[0.18em] z-10 transition-colors duration-300 ${
                        heatingModeActive ? 'text-orange-100' : 'text-white/22'
                      }`}
                    >
                      制热模式
                    </span>
                  </div>
                </div>

              </div>

              <TempCore
                temp={device.ac.temp}
                tempDisplayOn={tempDisplayOn}
                onDecrease={() => {
                  void adjustTemp(-1);
                }}
                onIncrease={() => {
                  void adjustTemp(1);
                }}
                disabled={tempDisabled}
              />
            </div>
          </div>

          <div className="w-64 flex flex-col py-2">
            <div className="flex flex-col flex-1 justify-center gap-8">
              <div className="flex items-center gap-3 px-1 mb-2">
                <div className="w-2.5 h-2.5 bg-white rotate-45 shadow-[0_0_6px_white,0_0_12px_rgba(255,255,255,0.2)]" />
                <span className="text-[13px] font-black tracking-[0.4em] text-white uppercase">
                  操作开关
                </span>
              </div>

              <div className="space-y-5">
                <TechToggle
                  active={acDisplayOn}
                  onClick={toggleAC}
                  disabled={!hasLoadedState || syncingAction || !device.connected || !device.acAvailable}
                  label="空调核心系统"
                  subLabel={acDisplayOn ? '核心运行中' : '已关闭'}
                  icon={
                    <Fan
                      className={acDisplayOn ? 'opacity-80' : ''}
                      size={24}
                      style={acDisplayOn ? { animation: 'spin 5s linear infinite' } : undefined}
                    />
                  }
                />

                <TechToggle
                  active={switchDisplayOn}
                  onClick={toggleSwitch}
                  disabled={!hasLoadedState || syncingAction || !device.connected || !device.switchAvailable}
                  label="环境氛围照明"
                  subLabel={switchDisplayOn ? '强光已开启' : '已关闭'}
                  icon={
                    <Lightbulb
                      size={24}
                      className={switchDisplayOn ? 'text-yellow-100 opacity-100' : 'text-white/70'}
                      style={
                        switchDisplayOn
                          ? {
                              filter: 'drop-shadow(0 0 10px rgba(255,240,180,0.9)) drop-shadow(0 0 18px rgba(255,220,120,0.7))',
                            }
                          : undefined
                      }
                    />
                  }
                />
              </div>
            </div>
          </div>
        </div>

        <div className="relative px-6 py-4 bg-black/20 text-[10px] flex justify-between items-center tracking-[0.2em] font-black border-t border-white/10 text-white/50 antialiased overflow-hidden">
          <StatusTicker />

          <div className="flex items-center gap-5 relative z-10">
            <span className="text-white/80">
              当前时间: <ClockText />
            </span>
            <span className="text-white/30">|</span>
            <div className="flex items-center gap-2">
              <div
                className={`w-1.5 h-1.5 rounded-full ${
                  device.connected
                    ? 'bg-cyan-300 opacity-90 shadow-[0_0_6px_rgba(103,232,249,0.5)]'
                    : 'bg-rose-300'
                }`}
              />
              <span className="text-white/80 uppercase">{statusLabel}</span>
            </div>
          </div>
        </div>
      </div>
    </motion.div>
  );
}
