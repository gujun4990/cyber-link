/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';
import { appWindow } from '@tauri-apps/api/window';
import { 
  Monitor, 
  Lightbulb, 
  Thermometer, 
  Power, 
  ChevronUp, 
  ChevronDown, 
  Settings, 
  X, 
  Minus, 
  RefreshCw,
  LogOut,
  Maximize2,
  Cpu,
  Fan,
  Snowflake,
  Flame,
  Zap,
  Activity
} from 'lucide-react';
import { motion } from 'motion/react';
import { ACTIONS, clampTemp } from './haActions';
import { applyStateRefresh } from './appState.js';
import { withTimeout } from './initTimeout.js';

// --- 类型定义 ---
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
    room: "核心-01",
    pcId: "终端-05",
    ac: { isOn: true, temp: 16 },
    lightOn: true,
    acAvailable: false,
    lightAvailable: false,
    connected: true,
  });

  const [initFailed, setInitFailed] = useState(false);
  const [actionFailed, setActionFailed] = useState(false);
  const [refreshFailed, setRefreshFailed] = useState(false);
  const [refreshError, setRefreshError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  const [currentTime, setCurrentTime] = useState(new Date());
  const syncingRef = useRef(false);
  const statusText = actionFailed
      ? '指令发送失败'
      : refreshFailed
        ? refreshError
          ? `刷新失败: ${refreshError}`
          : '刷新失败'
        : device.connected
        ? 'CYBER_NODE: ONLINE'
        : device.initError
          ? `初始化失败: ${device.initError}`
          : '服务器连接失败';

  const syncDevice = async (action: string, value?: number) => {
    if (syncingRef.current) return;
    syncingRef.current = true;
    try {
      const payload = value === undefined ? { action } : { action, value };
      await invoke<DeviceState>('handle_ha_action', payload);
      setActionFailed(false);
    } catch (error) {
      console.error('Failed to sync device action', error);
      setActionFailed(true);
    } finally {
      syncingRef.current = false;
    }
  };

  // 实时时间更新
  useEffect(() => {
    const timer = setInterval(() => setCurrentTime(new Date()), 1000);
    return () => clearInterval(timer);
  }, []);

  useEffect(() => {
    let unlisten: null | (() => void) = null;
    void (async () => {
      unlisten = await listen<DeviceState>('state-refresh', (event) => {
        if (event.payload) {
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
        }
      });

      try {
        await withTimeout(
          invoke<DeviceState>('initialize_app'),
          8000,
          'initialize_app timed out',
        );
      } catch (error) {
        console.error('Failed to initialize Tauri bridge', error);
        // Surface the initialization error so Windows users do not see a blank-looking shell.
        const msg = error instanceof Error ? error.message : String(error);
        setInitFailed(true);
        setDevice(prev => ({ ...prev, connected: false, initError: msg }));
      }
    })();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  const toggleAC = () => void syncDevice(ACTIONS.acToggle);
  const adjustTemp = async (delta: number) => {
    if (!device.ac.isOn) return;
    await syncDevice(ACTIONS.acSetTemp, clampTemp(device.ac.temp, delta));
  };
  const toggleLight = () => void syncDevice(ACTIONS.lightToggle);
  const hideWindow = async () => {
    try {
      await appWindow.hide();
    } catch (error) {
      console.error('Failed to hide window', error);
    }
  };
  const refreshHaState = async () => {
    if (refreshing) return;
    setRefreshing(true);
    try {
      await withTimeout(
        invoke<DeviceState>('refresh_ha_state'),
        8000,
        'refresh_ha_state timed out',
      );
    } catch (error) {
        console.error('Failed to refresh HA state', error);
        const msg = error instanceof Error ? error.message : String(error);
        setRefreshFailed(true);
        setRefreshError(msg);
    } finally {
      setRefreshing(false);
      }
  };

  return (
    <div className="min-h-screen flex items-center justify-center p-4 overflow-hidden">
      
      {/* 扫描线图层 - 极低强度以确保亮度感 */}
      <div className="fixed inset-0 pointer-events-none z-50 bg-[linear-gradient(rgba(18,16,16,0)_50%,rgba(0,0,0,0.02)_50%),linear-gradient(90deg,rgba(255,0,0,0.005),rgba(0,255,0,0.002),rgba(0,0,255,0.005))] bg-[length:100%_4px,3px_100%]" />

      <motion.div
        initial={{ opacity: 0, scale: 0.95, rotateX: 5 }}
        animate={{ opacity: 1, scale: 1, rotateX: 0 }}
        className="relative w-full max-w-[700px] aspect-[16/10] bg-[#0c2461]/90 backdrop-blur-3xl border-2 border-cyan-400/30 rounded-2xl overflow-visible flex flex-col shadow-[0_0_150px_rgba(6,182,212,0.4),inset_0_0_100px_rgba(0,0,0,0.5)]"
      >
            {/* 碳纤维/拉丝金属底层纹理 */}
            <div className="absolute inset-0 bg-carbon mix-blend-overlay opacity-40 rounded-2xl pointer-events-none" />
            <div className="absolute inset-0 bg-brushed-metal rounded-2xl pointer-events-none" />
            
            {/* 内部遮罩 - 全息悬浮感 */}
            <div className="absolute inset-0 bg-gradient-to-tr from-[#050c2d]/80 via-transparent to-cyan-500/10 rounded-2xl pointer-events-none" />
            
            {/* 极细发光线条 (Piping) */}
            <div className="absolute top-0 inset-x-0 h-px bg-gradient-to-r from-transparent via-cyan-400/80 to-transparent shadow-[0_0_10px_cyan] pointer-events-none" />
            <div className="absolute bottom-0 inset-x-0 h-px bg-gradient-to-r from-transparent via-cyan-400/80 to-transparent shadow-[0_0_10px_cyan] pointer-events-none" />
            
            {/* 科技边角装饰 */}
            <TechCorners />

            {/* 极简顶栏 - 提升层级与亮度 */}
            <div className="relative z-[70] flex items-center justify-between px-6 py-5 border-b border-cyan-200/40 bg-cyan-400/10 backdrop-blur-md">
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 flex items-center justify-center border-2 border-cyan-200 rounded shadow-[0_0_25px_rgba(6,182,212,0.6)] text-white bg-cyan-400/20">
                  <Monitor size={16} />
                </div>
                <div className="flex flex-col">
                  <span className="text-[11px] font-black tracking-[0.2em] text-white antialiased drop-shadow-[0_0_10px_cyan]">{statusText}</span>
                </div>
              </div>
              <div className="flex gap-4 items-center">
                <motion.button 
                  whileHover={{ rotate: 180, scale: 1.1, color: '#22d3ee' }}
                  whileTap={{ scale: 0.9 }}
                  disabled={refreshing}
                  onClick={() => {
                    console.log('Refresh Data');
                    void refreshHaState();
                  }} 
                  className={`w-8 h-8 flex items-center justify-center text-white/60 hover:text-cyan-400 transition-colors cursor-pointer ${refreshing ? 'opacity-50 cursor-not-allowed' : ''}`}
                  title="刷新系统数据"
                >
                  <RefreshCw size={18} strokeWidth={2.5} className={refreshing ? 'animate-spin' : ''} />
                </motion.button>
                <button 
                  onClick={() => { void hideWindow(); }} 
                  className="w-8 h-8 flex items-center justify-center hover:bg-cyan-400/30 text-white rounded-full transition-all active:scale-75 cursor-pointer"
                >
                  <Minus size={20} strokeWidth={3} />
                </button>
                <button 
                  onClick={(e) => {
                    e.stopPropagation();
                    void hideWindow();
                  }} 
                  className="w-10 h-10 flex items-center justify-center bg-rose-500/10 hover:bg-rose-500/40 hover:text-white text-rose-300 rounded-xl transition-all active:scale-75 cursor-pointer border border-rose-500/30 shadow-[0_0_15px_rgba(244,63,94,0.2)]"
                >
                  <X size={22} strokeWidth={3} />
                </button>
              </div>
            </div>

            {/* 主操作区 - 居中对称优化 */}
            <div className="relative flex-1 flex items-center justify-around px-12 py-6 overflow-visible">
              
              {/* 中央大控制盘 */}
              <div className="relative flex items-center justify-center w-[360px] h-[360px]">
                {/* 旋转装饰环 */}
                <motion.div 
                  className="absolute w-[320px] h-[320px] border border-dashed border-cyan-500/10 rounded-full"
                  animate={{ rotate: 360 }}
                  transition={{ duration: 20, repeat: Infinity, ease: "linear" }}
                />
                <motion.div 
                  className="absolute w-[280px] h-[280px] border-2 border-cyan-400/20 rounded-full border-t-transparent border-b-transparent"
                  animate={{ rotate: -360 }}
                  transition={{ duration: 15, repeat: Infinity, ease: "linear" }}
                />
                <div className="absolute w-[240px] h-[240px] border-4 border-cyan-500/5 rounded-full shadow-[inset_0_0_40px_rgba(6,182,212,0.05)]" />

                {/* 内容层 - 确保数字居中 */}
                <div className="relative z-10 flex flex-col items-center justify-center h-full w-full">
                  {/* 顶部状态与标题 - 使用绝对定位以不对主体中心造成偏移 */}
                  <div className="absolute top-4 flex flex-col items-center gap-1">
                    <span className="text-[11px] font-black tracking-[0.5em] text-white/50 uppercase font-sans drop-shadow-[0_0_8px_rgba(255,255,255,0.2)]">空调控制系统</span>
                    
                    {/* 模式状态指示灯 - 水晶质感 & 微电路纹理 */}
                    <div className="flex gap-3 mt-1.5 scale-90">
                      <div className={`group relative flex items-center gap-2 px-3 py-1.5 rounded-lg border transition-all duration-700 overflow-hidden ${device.ac.isOn && device.ac.temp < 20 ? 'border-cyan-200 bg-cyan-400/40 text-white shadow-[0_0_30px_rgba(6,182,212,0.7),inset_0_0_15px_rgba(255,255,255,0.4)]' : 'border-white/5 bg-white/2 text-white/10'}`}>
                        <div className="absolute inset-0 circuit-pattern opacity-30 pointer-events-none" />
                        <Snowflake size={12} className={device.ac.isOn && device.ac.temp < 20 ? 'animate-pulse text-cyan-100' : ''} />
                        <span className="text-[9px] font-black tracking-widest uppercase z-10 antialiased">制冷模式</span>
                        <motion.div 
                          animate={{ x: [-200, 200] }}
                          transition={{ duration: 2.5, repeat: Infinity, ease: "linear" }}
                          className="absolute inset-y-0 w-8 bg-white/20 skew-x-12 blur-[1px]"
                        />
                      </div>
                      
                      <div className={`group relative flex items-center gap-2 px-3 py-1.5 rounded-lg border transition-all duration-700 overflow-hidden ${device.ac.isOn && device.ac.temp > 26 ? 'border-orange-400 bg-orange-500/30 text-white shadow-[0_0_20px_rgba(249,115,22,0.4)]' : 'border-white/5 bg-white/2 text-white/5'}`}>
                        <Flame size={12} className={device.ac.isOn && device.ac.temp > 26 ? 'animate-pulse text-orange-400' : ''} />
                        <span className="text-[9px] font-black tracking-widest uppercase antialiased opacity-60">制热模式</span>
                        <div className="absolute inset-0 rounded-lg border border-orange-500/20 pointer-events-none group-hover:border-orange-500/40 transition-colors" />
                        {/* 扫光效果同步至制热模式 */}
                        <motion.div 
                          animate={{ x: [-200, 200] }}
                          transition={{ duration: 2.5, repeat: Infinity, ease: "linear" }}
                          className="absolute inset-y-0 w-8 bg-white/10 skew-x-12 blur-[1px]"
                        />
                      </div>
                    </div>
                  </div>

                  {/* 中央数字输入区 - 确保完全垂直居中 - 移除了 mt-8 以保证绝对中心偏移消失 */}
                  <div className="flex items-center gap-2">
                    <motion.button 
                      whileHover={device.ac.isOn ? { scale: 1.1, textShadow: "0 0 15px cyan" } : {}}
                      whileTap={device.ac.isOn ? { scale: 0.9 } : {}}
                      onClick={() => adjustTemp(-1)}
                      disabled={!device.ac.isOn}
                      className={`p-2 transition-all rounded-full border border-transparent ${
                        device.ac.isOn 
                          ? 'text-cyan-100/80 hover:border-cyan-300/40 hover:bg-cyan-400/10 cursor-pointer' 
                          : 'text-cyan-950/20 cursor-not-allowed'
                      }`}
                    >
                      <ChevronDown size={36} />
                    </motion.button>
                    
                    <div className="flex flex-col items-center min-w-[140px] relative">
                      {/* 多层光晕背景 */}
                      <div className={`absolute inset-0 blur-[50px] rounded-full transition-all duration-1000 ${device.ac.isOn ? 'bg-cyan-400/20 scale-110' : 'bg-transparent'}`} />
                      <div className={`absolute inset-0 blur-[25px] rounded-full transition-all duration-1000 ${device.ac.isOn ? 'bg-purple-500/5 scale-125' : 'bg-transparent'}`} />
                      
                      <motion.span 
                        key={device.ac.temp}
                        initial={{ opacity: 0, scale: 0.8, filter: 'blur(5px)' }}
                        animate={{ opacity: 1, scale: 1, filter: 'blur(0px)' }}
                        className={`text-[9rem] font-black tabular-nums transition-all duration-700 relative z-20 leading-[1.1] ${device.ac.isOn ? 'text-iridescent' : 'text-cyan-950/20'}`}
                        style={{ 
                          filter: device.ac.isOn 
                            ? 'drop-shadow(0 0 20px rgba(6,182,212,0.5))' 
                            : 'none' 
                        }}
                      >
                        {device.ac.temp}
                      </motion.span>
                    </div>

                    <motion.button 
                      whileHover={device.ac.isOn ? { scale: 1.1, textShadow: "0 0 15px cyan" } : {}}
                      whileTap={device.ac.isOn ? { scale: 0.9 } : {}}
                      onClick={() => adjustTemp(1)}
                      disabled={!device.ac.isOn}
                      className={`p-2 transition-all rounded-full border border-transparent ${
                        device.ac.isOn 
                      ? 'text-cyan-100/80 hover:border-cyan-300/40 hover:bg-cyan-400/10 cursor-pointer' 
                      : 'text-cyan-950/20 cursor-not-allowed'
                      }`}
                    >
                      <ChevronUp size={36} />
                    </motion.button>
                  </div>
                </div>
              </div>

              {/* 侧面控制组 - 优化版排版 - 缩短间距防止遮挡 */}
              <div className="w-64 flex flex-col py-2">
                <div className="flex flex-col flex-1 justify-center gap-8">
                  {/* 分组标题 - 简化正如截图要求 */}
                  <div className="flex items-center gap-3 px-1 mb-2">
                    <div className="w-2.5 h-2.5 bg-white rotate-45 shadow-[0_0_15px_white]" />
                    <span className="text-[13px] font-black tracking-[0.4em] text-white uppercase drop-shadow-[0_0_12px_cyan]">操作开关</span>
                  </div>
                  
                  <div className="space-y-5">
                    <TechToggle 
                      active={device.ac.isOn} 
                      onClick={toggleAC} 
                      disabled={!device.acAvailable}
                      label="空调核心系统" 
                      subLabel={device.ac.isOn ? "核心运行中" : "已离线"}
                      icon={<Fan className={device.ac.isOn ? 'animate-spin' : ''} size={24}/>} 
                    />
                    
                    <TechToggle 
                      active={device.lightOn} 
                      onClick={toggleLight} 
                      disabled={!device.lightAvailable}
                      label="环境氛围照明" 
                      subLabel={device.lightOn ? "强光已开启" : "低能耗状态"}
                      icon={<Lightbulb size={24}/>} 
                    />
                  </div>
                </div>
              </div>

            </div>

            {/* 底部信号栏 */}
            <div className="relative px-8 py-5 bg-[#050c2d]/90 text-[11px] flex justify-between items-center tracking-[0.3em] font-black border-t border-cyan-400/20 text-white/50 antialiased shadow-[0_-10px_30px_rgba(0,0,0,0.5)] overflow-hidden">
              {/* 微型滚动虚构数据流 */}
              <div className="absolute top-0 left-0 w-full h-[2px] overflow-hidden opacity-20">
                <motion.div 
                  animate={{ x: [0, -1000] }}
                  transition={{ duration: 20, repeat: Infinity, ease: "linear" }}
                  className="whitespace-nowrap font-mono text-[8px] flex gap-8"
                >
                  {Array.from({ length: 20 }).map((_, i) => (
                    <span key={i}>0x{Math.random().toString(16).slice(2, 10).toUpperCase()} - TRACE_PKT: {Math.floor(Math.random() * 9999)} - STREAMING_IO_READY</span>
                  ))}
                </motion.div>
              </div>

              <div className="flex items-center gap-6 relative z-10">
                    <span className="drop-shadow-[0_0_8px_cyan] text-white">
                    {initFailed ? (
                      '离线模式'
                    ) : actionFailed ? (
                      '指令发送失败'
                    ) : device.connected ? (
                      <>当前时间: <span className="font-mono">{currentTime.toLocaleTimeString('zh-CN', { hour12: false })}</span></>
                    ) : (
                      '离线模式'
                    )}
                  </span>
                <span className="text-cyan-900/40">|</span>
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 bg-cyan-400 rounded-full animate-ping shadow-[0_0_12px_cyan]" />
                  <span className="drop-shadow-[0_0_8px_cyan] text-white uppercase opacity-80">实时链路已同步</span>
                </div>
              </div>
              <span className="text-cyan-900/60 tracking-widest font-mono">CYBER_CTL_V09</span>
            </div>
      </motion.div>
    </div>
  );
}

function TechCorners() {
  return (
    <>
      <div className="absolute top-0 left-0 w-12 h-12 border-t-2 border-l-2 border-cyan-500/40 rounded-tl-2xl z-10" />
      <div className="absolute top-0 right-0 w-12 h-12 border-t-2 border-r-2 border-cyan-500/40 rounded-tr-2xl z-10" />
      <div className="absolute bottom-0 left-0 w-12 h-12 border-b-2 border-l-2 border-cyan-500/40 rounded-bl-2xl z-10" />
      <div className="absolute bottom-0 right-0 w-12 h-12 border-b-2 border-r-2 border-cyan-500/40 rounded-br-2xl z-10" />
    </>
  );
}

function SmallTechStat({ label, value }: { label: string, value: string }) {
  return (
    <div className="p-3 border border-cyan-400/10 bg-cyan-400/5 rounded-xl flex flex-col items-center justify-center backdrop-blur-sm transition-all hover:bg-cyan-400/10 hover:border-cyan-400/30">
      <span className="text-[9px] font-bold text-cyan-300/60 mb-1 tracking-tight">{label}</span>
      <span className="text-sm font-black text-cyan-300 tabular-nums">{value}</span>
    </div>
  );
}

function TechToggle({ active, onClick, disabled, label, subLabel, icon }: { active: boolean, onClick: () => void, disabled?: boolean, label: string, subLabel: string, icon: React.ReactNode }) {
  return (
    <motion.button
      whileHover={{ scale: 1.02, x: 4 }}
      whileTap={{ scale: 0.96 }}
      onClick={onClick}
      disabled={disabled}
      className={`relative w-full p-4 rounded-xl border transition-all duration-700 flex items-center gap-5 overflow-hidden group cursor-pointer shadow-neumorphic ${
        active 
          ? 'bg-cyan-500/15 border-cyan-400/40 text-white shadow-neumorphic-pressed' 
          : 'bg-[#0a153a]/60 border-white/5 text-white/20'
      }`}
    >
      {/* 内部纹理 */}
      <div className="absolute inset-0 bg-carbon mix-blend-overlay opacity-10 pointer-events-none" />
      
      <div className={`p-3 rounded-lg border transition-all duration-700 relative overflow-hidden ${active ? 'border-cyan-300 bg-cyan-400/30 shadow-[0_0_20px_rgba(6,182,212,0.6)]' : 'border-white/5 bg-white/2 opacity-30'}`}>
        {/* 风扇旋转动画模糊感 */}
        <div className={active && label.includes('空调') ? 'animate-spin opacity-80 blur-[1px]' : ''}>
          {icon}
        </div>
      </div>
      
      <div className="flex-1 text-left relative z-10">
        <div className={`text-[14px] font-black tracking-[0.1em] font-sans antialiased transition-colors duration-500 ${active ? 'text-white' : 'text-white/20'}`}>{label}</div>
        <div className={`text-[10px] font-bold mt-0.5 font-sans opacity-60 transition-colors duration-500 ${active ? 'text-cyan-200' : 'text-white/10'}`}>{subLabel}</div>
      </div>
      
      {/* 状态点指示器 */}
      <div className={`w-3 h-3 rounded-full transition-all duration-700 ${active ? 'bg-cyan-300 shadow-[0_0_15px_cyan,0_0_5px_white]' : 'bg-black/40 border border-white/10'}`} />
      
      {/* 高级扫略光效 */}
      {active && (
        <motion.div 
          animate={{ x: [-300, 500] }}
          transition={{ duration: 5, repeat: Infinity, ease: "linear" }}
          className="absolute top-0 right-0 bottom-0 w-32 bg-gradient-to-r from-transparent via-white/10 to-transparent skew-x-[40deg] pointer-events-none"
        />
      )}
    </motion.button>
  );
}
