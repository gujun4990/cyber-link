/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useEffect, useRef, useState } from 'react';
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
  lightOn: boolean;
  acAvailable: boolean;
  lightAvailable: boolean;
  connected: boolean;
  initError?: string;
}

export default function App() {
  const [device, setDevice] = useState<DeviceState>({
    room: '核心-01',
    pcId: '终端-05',
    ac: { isOn: true, temp: 16 },
    lightOn: false,
    acAvailable: true,
    lightAvailable: true,
    connected: true,
  });

  const [currentTime, setCurrentTime] = useState(new Date());
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

  const logMessage = async (message: string) => {
    try {
      await invoke('append_log_message', { message });
    } catch (error) {
      consoleFallbackRef.current.error('Failed to write app log', error);
    }
  };

  const reportError = async (message: string, error: unknown) => {
    const line = formatLogLine('ERROR', `${message}: ${describeError(error)}`);
    await logMessage(line);
    consoleFallbackRef.current.error(line);
  };

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
        void logMessage(formatLogLine(level.toUpperCase() as ConsoleLogLevelName, args.map(String).join(' ')));
      };
    }

    return () => {
      for (const [level, original] of originals) {
        patchedConsole[level] = original;
      }
    };
  }, []);

  // 保持底栏时间每秒刷新一次，和桌面状态感一致。
  useEffect(() => {
    const timer = setInterval(() => setCurrentTime(new Date()), 1000);
    return () => clearInterval(timer);
  }, []);

  // 监听后端推送的状态快照，并在启动时初始化 Tauri 桥接。
  useEffect(() => {
    let unlisten: null | (() => void) = null;

    void (async () => {
      const autostartMode = await invoke<boolean>('is_autostart_mode');

      unlisten = await listen<DeviceState>('state-refresh', (event) => {
        if (!event.payload) {
          return;
        }

        const next = applyStateRefresh(
          {
            device,
            initFailed,
            actionFailed,
            refreshFailed,
            refreshError,
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
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  // 所有设备操作都先走后端，避免前端状态和真实设备脱节。
  const syncDevice = async (action: string, value?: number) => {
    if (syncingRef.current) {
      return;
    }

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
  };

  // 设备不可用时直接拦截，保持 UI 和后端能力一致。
  const toggleAC = () => {
    if (!hasLoadedState || syncingAction || !device.acAvailable) {
      return;
    }

    void syncDevice(ACTIONS.acToggle);
  };

  const adjustTemp = async (delta: number) => {
    if (!hasLoadedState || syncingAction || !device.ac.isOn || !device.acAvailable) {
      return;
    }

    await syncDevice(ACTIONS.acSetTemp, clampTemp(device.ac.temp, delta));
  };

  const toggleLight = () => {
    if (!hasLoadedState || syncingAction || !device.lightAvailable) {
      return;
    }

    void syncDevice(ACTIONS.lightToggle);
  };

  // 顶栏右侧的刷新按钮，用于主动拉取最新状态。
  const refreshHaState = async () => {
    if (refreshing) {
      return;
    }

    setRefreshing(true);
    try {
      await withTimeout(
        invoke<DeviceState>('refresh_ha_state'),
        8000,
        'refresh_ha_state timed out',
      );
      setHasLoadedState(true);
      setActionFailed(false);
    } catch (error) {
      void reportError('Failed to refresh HA state', error);
      const msg = describeError(error);
      setRefreshFailed(true);
      setRefreshError(msg);
    } finally {
      setRefreshing(false);
    }
  };

  // 关闭按钮也不真正退出，而是隐藏到托盘，保持后台常驻。
  const hideWindow = async () => {
    try {
      await appWindow.hide();
    } catch (error) {
      void reportError('Failed to hide window', error);
    }
  };

  // 最小化按钮同样隐藏到托盘，和关闭按钮保持一致。
  const minimizeWindow = async () => {
    try {
      await appWindow.hide();
    } catch (error) {
      void reportError('Failed to hide window', error);
    }
  };

  const dragTopBar = async (event: React.MouseEvent<HTMLDivElement>) => {
    if (event.button !== 0 || event.target !== event.currentTarget) {
      return;
    }

    try {
      await appWindow.startDragging();
    } catch (error) {
      void reportError('Failed to drag window', error);
    }
  };

  // 底栏状态文案集中计算，避免 JSX 里堆太多条件分支。
  const statusLabel = initFailed
    ? 'OFFLINE_MODE'
    : !hasLoadedState
      ? '系统初始化中'
    : actionFailed
      ? 'ACTION_FAILED'
      : refreshFailed
        ? 'REFRESH_FAILED'
        : 'Encrypted_Link_Stable';

  const acDisplayOn = hasLoadedState && device.connected && device.ac.isOn;
  const lightDisplayOn = hasLoadedState && device.connected && device.lightOn;
  const coolingModeActive = hasLoadedState && device.connected && device.ac.isOn && device.ac.temp < 20;
  const heatingModeActive = hasLoadedState && device.connected && device.ac.isOn && device.ac.temp > 26;
  const tempDisplayOn = hasLoadedState && device.connected && device.ac.isOn;

  return (
    <>
      <motion.div
        layoutId="main-dashboard"
        initial={{ opacity: 0, scale: 0.9, y: 20 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        className="fixed inset-0 m-auto border-[1.5px] border-white/20 overflow-hidden flex flex-col shadow-[0_30px_100px_rgba(0,0,0,0.7),0_0_40px_rgba(6,182,212,0.25)] antialiased"
        style={{
          width: windowSize.width,
          height: windowSize.height,
          background: `
            linear-gradient(135deg, rgba(18, 32, 72, 0.90), rgba(10, 20, 60, 0.95)),
            rgba(14, 26, 80, 1)
          `,
        }}
      >
        {/* 窗口边框和材质层，负责整体的客户端壳感。 */}
        <div className="absolute inset-0 rounded-xl border border-white/10 pointer-events-none z-50" />
        <div className="absolute inset-0 bg-carbon mix-blend-soft-light opacity-20 pointer-events-none" />

          {/* 顶栏支持拖拽，右侧按钮区必须禁用拖拽。 */}
          <div
            className="relative z-[70] flex items-center justify-between px-4 py-3 bg-black/20 border-b border-white/10 backdrop-blur-xl select-none"
            onMouseDown={(event) => {
              void dragTopBar(event);
            }}
            onDoubleClickCapture={(event) => {
              event.preventDefault();
              event.stopPropagation();
            }}
          >
            <div className="flex items-center gap-2.5">
              <div className="w-6 h-6 flex items-center justify-center bg-cyan-500/35 border border-cyan-400/50 rounded shadow-[0_0_18px_rgba(6,182,212,0.55)]">
                <Monitor size={14} className="text-cyan-300" />
              </div>
              <div className="flex flex-col">
                <span className="text-[10px] font-black tracking-widest text-white antialiased uppercase">
                  Cyber Terminal v0.9
                </span>
              </div>
            </div>

            <div className="flex gap-2 items-center">
              <motion.button
                whileHover={{ scale: 1.1, color: '#22d3ee' }}
                whileTap={{ scale: 0.9 }}
                onClick={() => {
                  void refreshHaState();
                }}
                disabled={refreshing}
                className={`w-7 h-7 flex items-center justify-center text-white/60 hover:text-cyan-300 transition-colors cursor-pointer ${
                  refreshing ? 'opacity-60 cursor-not-allowed' : ''
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
                className="w-8 h-8 flex items-center justify-center text-white/60 hover:bg-white/10 hover:text-white rounded transition-all cursor-pointer"
              >
                <Minus size={16} strokeWidth={2} />
              </button>
              <button
                onClick={() => {
                  void hideWindow();
                }}
                className="w-8 h-8 flex items-center justify-center text-white/60 hover:bg-rose-500 hover:text-white rounded transition-all cursor-pointer"
              >
                <X size={16} strokeWidth={2} />
              </button>
            </div>
          </div>

          {/* 主体区：中间控制盘 + 右侧开关。 */}
          <div className="relative flex-1 flex flex-col overflow-hidden">
            <div className="absolute top-0 inset-x-0 h-px bg-gradient-to-r from-transparent via-cyan-400/70 to-transparent pointer-events-none" />

            <div className="relative flex-1 flex items-center justify-around px-12 py-4 overflow-visible">
                  {/* 中央温控盘：视觉核心。 */}
                  <div className="relative flex items-center justify-center w-[360px] h-[360px]">
                    <motion.div
                      className="absolute w-[320px] h-[320px] border border-dashed border-cyan-500/25 rounded-full"
                      animate={{ rotate: 360 }}
                      transition={{ duration: 20, repeat: Infinity, ease: 'linear' }}
                    />
                    <motion.div
                      className="absolute w-[280px] h-[280px] border-2 border-cyan-400/40 rounded-full border-t-transparent border-b-transparent"
                      animate={{ rotate: -360 }}
                      transition={{ duration: 15, repeat: Infinity, ease: 'linear' }}
                    />
                    <div className="absolute w-[240px] h-[240px] border-4 border-cyan-500/10 rounded-full shadow-[inset_0_0_40px_rgba(6,182,212,0.08)]" />

                    <div className="relative z-10 flex flex-col items-center justify-center h-full w-full">
                      <div className="absolute top-4 flex flex-col items-center gap-1">
                        <span className="text-[11px] font-black tracking-[0.5em] text-white/70 uppercase font-sans drop-shadow-[0_0_8px_rgba(255,255,255,0.3)]">
                          空调控制系统
                        </span>

                        {/* 模式标签：根据温度和空调状态点亮。 */}
                        <div className="flex gap-3 mt-1.5 scale-90">
                          <div
                            className={`group relative flex items-center gap-2 px-3 py-1.5 rounded-lg border transition-all duration-700 overflow-hidden ${
                              coolingModeActive
                                ? 'border-cyan-100 bg-cyan-400/55 text-white shadow-[0_0_45px_rgba(6,182,212,0.9),inset_0_0_20px_rgba(255,255,255,0.5)]'
                                : 'border-white/8 bg-white/2 text-white/15'
                            }`}
                          >
                            <div className="absolute inset-0 circuit-pattern opacity-30 pointer-events-none" />
                            <Snowflake size={12} className={coolingModeActive ? 'animate-pulse text-cyan-100' : ''} />
                            <span className="text-[9px] font-black tracking-widest uppercase z-10 antialiased">
                              制冷模式
                            </span>
                            <motion.div
                              animate={{ x: [-200, 200] }}
                              transition={{ duration: 2.5, repeat: Infinity, ease: 'linear' }}
                              className="absolute inset-y-0 w-8 bg-white/20 skew-x-12 blur-[1px]"
                            />
                          </div>

                          <div
                            className={`group relative flex items-center gap-2 px-3 py-1.5 rounded-lg border transition-all duration-700 overflow-hidden ${
                              heatingModeActive
                                ? 'border-orange-300 bg-orange-500/40 text-white shadow-[0_0_30px_rgba(249,115,22,0.6)]'
                                : 'border-white/5 bg-white/2 text-white/8'
                            }`}
                          >
                            <Flame size={12} className={heatingModeActive ? 'animate-pulse text-orange-300' : ''} />
                            <span className="text-[9px] font-black tracking-widest uppercase antialiased opacity-60">
                              制热模式
                            </span>
                            <div className="absolute inset-0 rounded-lg border border-orange-500/20 pointer-events-none group-hover:border-orange-500/40 transition-colors" />
                            <motion.div
                              animate={{ x: [-200, 200] }}
                              transition={{ duration: 2.5, repeat: Infinity, ease: 'linear' }}
                              className="absolute inset-y-0 w-8 bg-white/10 skew-x-12 blur-[1px]"
                            />
                          </div>
                        </div>
                      </div>

                      {/* 温度调节区：上下按钮包围数字。 */}
                      <div className="flex items-center gap-2">
                        <motion.button
                          whileHover={tempDisplayOn ? { scale: 1.1, textShadow: '0 0 15px cyan' } : {}}
                          whileTap={tempDisplayOn ? { scale: 0.9 } : {}}
                          onClick={() => {
                            void adjustTemp(-1);
                          }}
                          disabled={!hasLoadedState || syncingAction || !device.connected || !device.acAvailable || !device.ac.isOn}
                          className={`p-2 transition-all rounded-full border border-transparent ${
                            tempDisplayOn
                              ? 'text-cyan-200/90 hover:border-cyan-300/50 hover:bg-cyan-400/15 cursor-pointer'
                              : 'text-cyan-950/20 cursor-not-allowed'
                          }`}
                        >
                          <ChevronDown size={36} />
                        </motion.button>

                        <div className="flex flex-col items-center min-w-[140px] relative">
                          <div
                            className={`absolute inset-0 blur-[50px] rounded-full transition-all duration-1000 ${
                              tempDisplayOn ? 'bg-cyan-400/40 scale-110' : 'bg-transparent'
                            }`}
                          />
                          <div
                            className={`absolute inset-0 blur-[25px] rounded-full transition-all duration-1000 ${
                              tempDisplayOn ? 'bg-purple-500/8 scale-125' : 'bg-transparent'
                            }`}
                          />

                          <motion.span
                            key={device.ac.temp}
                            initial={{ opacity: 0, scale: 0.8, filter: 'blur(5px)' }}
                            animate={{ opacity: 1, scale: 1, filter: 'blur(0px)' }}
                            className={`text-[9rem] font-black tabular-nums transition-all duration-700 relative z-20 leading-[1.1] ${
                              tempDisplayOn ? 'text-iridescent' : 'text-cyan-950/20'
                            }`}
                            style={{
                              filter: tempDisplayOn
                                ? 'drop-shadow(0 0 35px rgba(6,182,212,0.85)) drop-shadow(0 0 60px rgba(6,182,212,0.4))'
                                : 'none',
                            }}
                          >
                            {device.ac.temp}
                          </motion.span>
                        </div>

                        <motion.button
                          whileHover={tempDisplayOn ? { scale: 1.1, textShadow: '0 0 15px cyan' } : {}}
                          whileTap={tempDisplayOn ? { scale: 0.9 } : {}}
                          onClick={() => {
                            void adjustTemp(1);
                          }}
                          disabled={!hasLoadedState || syncingAction || !device.connected || !device.acAvailable || !device.ac.isOn}
                          className={`p-2 transition-all rounded-full border border-transparent ${
                            tempDisplayOn
                              ? 'text-cyan-200/90 hover:border-cyan-300/50 hover:bg-cyan-400/15 cursor-pointer'
                              : 'text-cyan-950/20 cursor-not-allowed'
                          }`}
                        >
                          <ChevronUp size={36} />
                        </motion.button>
                      </div>
                    </div>
                  </div>

                  {/* 右侧开关区：对应设备能力和当前状态。 */}
                  <div className="w-64 flex flex-col py-2">
                    <div className="flex flex-col flex-1 justify-center gap-8">
                      <div className="flex items-center gap-3 px-1 mb-2">
                        <div className="w-2.5 h-2.5 bg-white rotate-45 shadow-[0_0_18px_white,0_0_30px_rgba(255,255,255,0.4)]" />
                        <span className="text-[13px] font-black tracking-[0.4em] text-white uppercase drop-shadow-[0_0_15px_cyan]">
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
                          icon={<Fan className={acDisplayOn ? 'animate-spin' : ''} size={24} />}
                        />

                        <TechToggle
                          active={lightDisplayOn}
                          onClick={toggleLight}
                          disabled={!hasLoadedState || syncingAction || !device.connected || !device.lightAvailable}
                          label="环境氛围照明"
                          subLabel={lightDisplayOn ? '强光已开启' : '已关闭'}
                          icon={<Lightbulb size={24} />}
                        />
                      </div>
                    </div>
                  </div>
                </div>

                {/* 底栏状态条：时间、连接状态、版本标识。 */}
                <div className="relative px-6 py-4 bg-black/20 text-[10px] flex justify-between items-center tracking-[0.2em] font-black border-t border-white/10 text-white/50 antialiased overflow-hidden">
                  <div className="absolute top-0 left-0 w-full h-[1px] overflow-hidden opacity-15">
                    <motion.div
                      animate={{ x: [0, -1000] }}
                      transition={{ duration: 30, repeat: Infinity, ease: 'linear' }}
                      className="whitespace-nowrap font-mono text-[7px] flex gap-8"
                    >
                      {Array.from({ length: 20 }).map((_, i) => (
                        <span key={i}>SYSTEM_CORE_LOAD: {Math.floor(Math.random() * 100)}% - TEMP_STABLE - NODE_READY</span>
                      ))}
                    </motion.div>
                  </div>

                  <div className="flex items-center gap-5 relative z-10">
                    <span className="text-white/80">
                      TIME: <span className="font-mono">{currentTime.toLocaleTimeString('zh-CN', { hour12: false })}</span>
                    </span>
                    <span className="text-white/15">|</span>
                    <div className="flex items-center gap-2">
                      <div
                        className={`w-1.5 h-1.5 rounded-full ${
                          device.connected ? 'bg-cyan-300 animate-ping opacity-80' : 'bg-rose-300'
                        }`}
                      />
                      <span className="text-white/60 uppercase">{statusLabel}</span>
                    </div>
                  </div>
                  <span className="text-white/20 tracking-[0.4em] font-mono">B_CTL_V9</span>
                </div>
              </div>
      </motion.div>
    </>
  );
}

function TechToggle({
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
      whileHover={{ scale: 1.02, x: 4 }}
      whileTap={{ scale: 0.96 }}
      onClick={onClick}
      disabled={disabled}
      className={`relative w-full p-4 rounded-xl border transition-all duration-700 flex items-center gap-5 overflow-hidden group ${
        disabled ? 'opacity-60 cursor-not-allowed' : 'cursor-pointer'
      } ${
        active
          ? 'bg-cyan-500/25 border-cyan-400/60 text-white shadow-[0_0_25px_rgba(6,182,212,0.2)]'
          : 'bg-[#0a153a]/60 border-white/8 text-white/25'
      }`}
    >
      <div className="absolute inset-0 bg-carbon mix-blend-overlay opacity-10 pointer-events-none" />

      <div
        className={`p-3 rounded-lg border transition-all duration-700 relative overflow-hidden ${
          active
            ? 'border-cyan-200 bg-cyan-400/50 shadow-[0_0_30px_rgba(6,182,212,0.85)]'
            : 'border-white/5 bg-white/2 opacity-30'
        }`}
      >
        <div className={active && label.includes('空调') ? 'animate-spin opacity-80 blur-[1px]' : ''}>{icon}</div>
      </div>

      <div className="flex-1 text-left relative z-10">
        <div className={`text-[14px] font-black tracking-[0.1em] font-sans antialiased transition-colors duration-500 ${active ? 'text-white' : 'text-white/25'}`}>
          {label}
        </div>
        <div className={`text-[10px] font-bold mt-0.5 font-sans opacity-70 transition-colors duration-500 ${active ? 'text-cyan-100' : 'text-white/12'}`}>
          {subLabel}
        </div>
      </div>

      <div className={`w-3 h-3 rounded-full transition-all duration-700 ${
        active
          ? 'bg-cyan-200 shadow-[0_0_20px_cyan,0_0_8px_white,0_0_40px_rgba(6,182,212,0.6)]'
          : 'bg-black/40 border border-white/10'
      }`} />

      {active && (
        <motion.div
          animate={{ x: [-300, 500] }}
          transition={{ duration: 5, repeat: Infinity, ease: 'linear' }}
          className="absolute top-0 right-0 bottom-0 w-32 bg-gradient-to-r from-transparent via-white/12 to-transparent skew-x-[40deg] pointer-events-none"
        />
      )}
    </motion.button>
  );
}
