# Home Assistant 实时状态过滤说明

## 目的

让同一个 Home Assistant 实例下的多台 Windows 客户端，只接收自己绑定的空调和控制项状态变化。

例如：

- 房间1有 3 台电脑，绑定 `ac1` 和对应的控制项
- 房间2有 3 台电脑，绑定 `ac2` 和对应的控制项

当房间1的某台电脑打开 `ac1` 时，房间1其他电脑要同步显示“空调已开启”，但房间2不能受到影响。

## 核心思路

### 1. 控制仍然走 REST

开空调、关空调、调温度、开关控制项，仍然走现有的 Home Assistant REST 接口。

这部分不变，保持和现在的实现一致。

### 2. 状态同步改成 HA WebSocket 推送

客户端启动后，额外建立一个到 Home Assistant 的 WebSocket 连接。

Home Assistant 一旦检测到实体状态变化，就主动推送 `state_changed` 事件给客户端。

### 3. 每个客户端只处理自己绑定的实体

每个客户端配置自己的实体绑定，例如：

- `ac_entity_id = climate.ac1`
- `entity_id.ambient_light = light.room1_ambient_light`
- `pc_entity_id = input_boolean.room1_pc_online`（可省略）

收到 HA 推送后，客户端先检查 `entity_id`：

- 如果是自己绑定的 `ac1` 或控制项实体，就更新本地状态
- 如果是别的房间，比如 `ac2`，就直接忽略

这样房间1和房间2就会自然隔离。

## 工作流程示例

### 启动

1. 客户端启动
2. 先用 REST 拉一次当前状态，初始化界面
3. 再建立 HA WebSocket 连接
4. 订阅 `state_changed`

### 房间1开空调

1. 房间1某台电脑点击“开空调”
2. 客户端通过 REST 调用 `climate.turn_on`
3. HA 更新 `climate.ac1` 状态
4. HA 向所有连接推送状态变化
5. 房间1客户端看到 `entity_id = climate.ac1`，更新界面
6. 房间2客户端看到的是 `climate.ac1`，但自己的绑定是 `climate.ac2`，所以忽略

### 房间1开控制项

1. 房间1某台电脑点击“开控制项”
2. 客户端通过 REST 调用对应域的 `turn_on`
3. HA 推送控制项状态变化
4. 只有绑定这个实体的客户端会更新

## 为什么这样做

- 不需要客户端自己猜“是不是同一房间”
- 过滤依据清晰，直接看 `entity_id`
- 保留现有 REST 控制，不用重写按钮逻辑
- 状态更实时，比轮询更自然

## 实现建议

- 启动时只启动一个常驻 WebSocket 监听任务
- 监听任务断线后自动重连
- 收到消息时先解析 `entity_id`
- 只对绑定的 `ac` / `ambient_light` / `main_light` / `door_sign_light` / `pc` 做刷新
- 刷新后继续沿用现有的 `state-refresh` 事件通知前端

## 适用范围

这套方案适合：

- 同一个 HA 里有多个房间
- 每个房间有自己的空调和控制项实体
- 多台电脑同时运行同一个客户端
- 希望同房间同步、跨房间隔离
