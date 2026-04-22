# “最后一台电脑”逻辑说明

## 目的

不要在 Windows 客户端里判断“是不是最后一台电脑”。这个判断应该交给 Home Assistant 统一处理，并且按房间隔离：

- 每台电脑只负责上报自己的在线/离线状态
- 同房间多台电脑由 HA 聚合成一个房间级在线状态
- 房间里任意一台电脑上线时，立即打开这个房间的空调和灯
- 房间里最后一台电脑离线并持续 30 秒时，再关闭这个房间的空调和灯

## 当前数据模型

### `config.json` 仍然是电脑级

每台电脑仍然使用自己的 `config.json`，可选字段保持不变：

- `pc_entity_id`
- `entity_id.ac`
- `entity_id.ambient_light`
- `entity_id.main_light`
- `entity_id.door_sign_light`

这些字段都可以不填；但如果要启用“最后一台电脑离线后延迟 30 秒关闭空调和灯”，每台电脑都必须配置 `pc_entity_id`。

### `pc_entity_id` 的含义

`pc_entity_id` 仍然表示“这台电脑是否在线”。它是电脑级实体，不是房间级实体。

前提：如果要启用“最后一台电脑离线后延迟 30 秒关闭空调和灯”，每台电脑都必须配置 `pc_entity_id`。

一间房里如果有多台电脑，就会有多个 `pc_entity_id`，例如：

- `input_boolean.room1_pc_a_online`
- `input_boolean.room1_pc_b_online`
- `input_boolean.room1_pc_c_online`

## 房间级聚合

HA 为每个房间创建一个聚合传感器，例如：

- `binary_sensor.room1_any_pc_online`

它的语义是：

- 任意一台电脑在线 -> `on`
- 全部电脑离线 -> `off`

这个聚合传感器由同房间的多个 `pc_entity_id` 计算得到。

没有 `pc_entity_id` 时，HA 没法判断房间里是不是最后一台电脑，因此也就无法做 30 秒延迟关闭。

## 控制逻辑

### 开机

如果 `pc_entity_id` 已配置：

1. 电脑 A 启动
2. App 把 `input_boolean.room1_pc_a_online` 置为 `on`
3. `binary_sensor.room1_any_pc_online` 变为 `on`
4. HA 自动化立即打开该房间的空调和已配置的灯

如果 `pc_entity_id` 未配置：

1. 电脑 A 启动
2. App 直接开启该电脑配置的空调和灯

### 关机

如果 `pc_entity_id` 已配置：

1. 电脑 A 关机
2. App 把 `input_boolean.room1_pc_a_online` 置为 `off`
3. 如果房间里还有其他电脑在线，房间级传感器仍然是 `on`
4. 如果这是最后一台电脑，房间级传感器变成 `off`
5. HA 延迟 30 秒
6. 30 秒内如果又有任意电脑上线，取消关机动作
7. 30 秒后仍然离线，关闭该房间的空调和已配置的灯

如果 `pc_entity_id` 未配置：

1. 电脑 A 关机
2. App 直接关闭该电脑配置的空调和灯

## 推荐 HA 实现

### 1. 房间聚合传感器

每个房间建一个 `template binary_sensor`，把同房间电脑的在线状态 OR 起来。

### 2. Blueprint automation

每个房间只建一个 blueprint 实例，输入：

- `room_online_entity`
- `ac_entity`
- `ambient_light_entity`
- `main_light_entity`
- `door_sign_light_entity`
- `delay_seconds`

`door_sign_light_entity`、`ambient_light_entity`、`main_light_entity`、`ac_entity` 都可以按房间实际情况不填。

### 3. 设备可选

如果某个房间没有门牌灯、没有氛围灯、甚至没有主灯，只要 blueprint 实例里不填对应项即可。

## 示例

### 房间 1

电脑：

- `input_boolean.room1_pc_a_online`
- `input_boolean.room1_pc_b_online`
- `input_boolean.room1_pc_c_online`

聚合传感器：

- `binary_sensor.room1_any_pc_online`

设备：

- `climate.room1_ac`
- `switch.room1_ambient_light`
- `light.room1_main_light`
- `switch.room1_door_sign_light`

### 房间 1 blueprint 实例

- `room_online_entity`: `binary_sensor.room1_any_pc_online`
- `ac_entity`: `climate.room1_ac`
- `ambient_light_entity`: `switch.room1_ambient_light`
- `main_light_entity`: `light.room1_main_light`
- `door_sign_light_entity`: `switch.room1_door_sign_light`
- `delay_seconds`: `30`

### 房间 2（没有门牌灯）

电脑：

- `input_boolean.room2_pc_a_online`
- `input_boolean.room2_pc_b_online`

聚合传感器：

- `binary_sensor.room2_any_pc_online`

设备：

- `climate.room2_ac`
- `switch.room2_ambient_light`
- `light.room2_main_light`

这个房间没有门牌灯，所以不需要配置 `door_sign_light_entity`。

### 房间 2 blueprint 实例

- `room_online_entity`: `binary_sensor.room2_any_pc_online`
- `ac_entity`: `climate.room2_ac`
- `ambient_light_entity`: `switch.room2_ambient_light`
- `main_light_entity`: `light.room2_main_light`
- `door_sign_light_entity`: 留空
- `delay_seconds`: `30`

### 这个例子怎么跑

房间 1：

1. `room1_pc_a_online` 变成 `on`
2. `room1_any_pc_online` 变成 `on`
3. blueprint 立即打开 `room1_ac`、`room1_ambient_light`、`room1_main_light`、`room1_door_sign_light`
4. 所有房间 1 电脑都变成 `off` 并持续 30 秒后，blueprint 再关闭这些设备

房间 2：

1. `room2_pc_b_online` 变成 `on`
2. `room2_any_pc_online` 变成 `on`
3. blueprint 立即打开 `room2_ac`、`room2_ambient_light`、`room2_main_light`
4. `door_sign_light_entity` 没有配置，所以门牌灯不会被控制
5. 所有房间 2 电脑都变成 `off` 并持续 30 秒后，blueprint 只关闭已配置的设备

## 结论

这套方案保留了 `config.json` 的电脑级可选配置，同时把“最后一台电脑”的判断上移到 HA 的房间级聚合层。这样既能支持多房间，也能支持每个房间设备不完全相同的情况。
