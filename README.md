# cyber-link

`cyber-link` 是一个基于 Tauri + React 的 Windows 桌面控制端，用于连接 Home Assistant，实现空调、灯光和在线状态联动控制。

## 目录

- 项目功能
- 架构说明
- 状态流
- 核心特性
- Windows 行为
- 配置文件
- 使用步骤
- 开发环境
- 构建
- Cargo 依赖
- 故障排查
- 相关文档

## 项目功能

- 默认隐藏到系统托盘
- 通过 Home Assistant 控制空调和灯光
- 通过 `input_boolean` 上报本机在线/离线状态
- 拦截 Windows 关机消息并发送离线通知
- 通过后端快照和 `state-refresh` 事件同步前端 UI

## 架构说明

- `src/App.tsx`：React 视觉层和事件驱动状态投影
- `src/haActions.ts`：动作常量和温度辅助函数
- `src-tauri/src/main.rs`：Tauri 命令、HA HTTP 请求、托盘、注册表、关机钩子
- `config.json`：运行时 Home Assistant 配置

## 状态流

1. 前端调用 `initialize_app`
2. Rust 读取 `config.json`
3. Rust 确保自启动注册表已写入
4. Rust 发送上线通知和启动动作
5. Rust 写入 `SharedState`
6. Rust 发送 `state-refresh`
7. 前端根据推送快照更新 UI

如果初始化失败，应用仍会启动，但后端会返回离线快照，前端显示离线或连接失败状态。

## 核心特性

- 空调开关
- 空调温度调节
- 灯光开关
- 在线/离线状态通知
- Windows 关机拦截
- 托盘菜单
- 自启动注册表管理
- 后端统一发送请求并同步状态

## Windows 行为

- 启动后默认隐藏
- 常驻系统托盘
- 在 `WM_QUERYENDSESSION` 前发送关机离线通知
- 关机时尝试关闭空调和灯光

## 配置文件

请在应用目录或可执行文件目录创建 `config.json`：

```json
{
  "ha_url": "https://home-assistant.local",
  "token": "YOUR_HOME_ASSISTANT_LONG_LIVED_TOKEN",
  "pc_entity_id": "input_boolean.your_pc_online",
  "entity_id": {
    "ac": "climate.your_ac_entity",
    "light": "light.your_light_entity"
  }
}
```

### 字段说明

- `ha_url`：Home Assistant 基础地址
- `token`：长期访问令牌
- `pc_entity_id`：表示本机在线/离线的 `input_boolean`
- `entity_id.ac`：空调实体
- `entity_id.light`：灯光实体

### Home Assistant 实体示例

- `input_boolean.your_pc_online`
- `climate.office_ac`
- `light.desk_light`

## 使用步骤

1. 在 Home Assistant 中创建对应实体
2. 复制 `config.json` 到应用目录或可执行文件目录
3. 填入 `ha_url`、`token` 和实体 ID
4. 启动应用
5. 确认托盘图标出现，UI 显示在线或离线状态
6. 通过按钮控制空调和灯光

## 截图

> 这里可放置应用主界面截图、托盘菜单截图、离线状态截图。

- 主界面：`[待插入]`
- 托盘菜单：`[待插入]`
- 离线状态：`[待插入]`

## 开发环境

### 依赖

- Node.js + npm
- Rust 工具链
- Windows（用于完整原生行为验证）

### 前端开发

```bash
npm install
npm run dev
```

### Rust 后端测试

```bash
cd src-tauri
cargo test
```

## 构建

```bash
npm run build
cd src-tauri
cargo build --release
```

## Cargo 依赖

后端当前使用：

- `anyhow`
- `reqwest`（启用 `json`、`rustls-tls`）
- `serde`（启用 `derive`）
- `serde_json`
- `tokio`（启用 `rt-multi-thread`、`macros`）
- Windows 目标下的 `tauri`（启用 `system-tray`、`api-all`）
- Windows 目标下的 `windows`（启用 `Win32_Foundation`、`Win32_System_Registry`、`Win32_UI_WindowsAndMessaging`）

## 说明

- React UI 只负责展示，状态以 Rust 后端快照和事件为准
- `state-refresh` 是前后端同步的主通道
- 启动/关机通知采用尽力执行策略，保证尽可能联动设备
- 如果 HA 或自启动校验失败，应用会降级为离线快照，而不是直接阻断 UI

## 故障排查

- **显示离线**：检查 `config.json`、Home Assistant 地址、Token 和网络连通性
- **无法自启动**：检查 `HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Run` 是否写入成功
- **关机通知未发送**：确认应用正在 Windows 上运行且托盘钩子已成功安装

## 相关文档

- [`“最后一台电脑”逻辑说明`](docs/last-pc-ha-logic.md)
