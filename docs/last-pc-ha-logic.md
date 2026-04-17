# “最后一台电脑”逻辑说明

## 目的

不要在 Windows 客户端判断“是不是最后一台电脑”。
客户端彼此之间很难实时同步状态，容易出现误判。

正确做法是把这个判断交给 Home Assistant 统一处理：

- 每台电脑只负责上报自己的在线/离线状态
- Home Assistant 汇总同一房间所有电脑的状态
- 由 Home Assistant 判断什么时候该打开或关闭空调、灯光

## 核心思路

### 1. 每台电脑一个 `input_boolean`

每台 Windows 客户端在 Home Assistant 中对应一个虚拟开关：

- `input_boolean.pc_a`
- `input_boolean.pc_b`
- `input_boolean.pc_c`

客户端启动时把自己的开关置为 `on`。
客户端关机时把自己的开关置为 `off`。

### 2. 同房间电脑放进一个 `group`

把同一房间的电脑开关放入一个组，例如：

- `group.room_1_pcs`

这个组代表“这个房间所有电脑的在线集合”。

### 3. HA 自动化根据组状态控制设备

- 组内任意一个开关变成 `on` -> 打开该房间的空调和灯
- 整个组变成 `off` -> 关闭该房间的空调和灯

这样就不需要客户端自己判断“是不是最后一台”。

## 工作流程示例

### 开机

1. 电脑 A 启动
2. 客户端把 `input_boolean.pc_a` 置为 `on`
3. `group.room_1_pcs` 变为 `on`
4. HA 自动化触发，打开空调和灯

### 关机

1. 电脑 A 关机
2. 客户端把 `input_boolean.pc_a` 置为 `off`
3. 如果组内还有其他电脑是 `on`，空调和灯保持开启
4. 如果组内所有电脑都变成 `off`，HA 自动化触发，关闭空调和灯

## YAML 示例

> 下面示例假设你已经有：
> - 电脑开关：`input_boolean.pc_a`、`input_boolean.pc_b`、`input_boolean.pc_c`
> - 房间组：`group.room_1_pcs`
> - 空调：`climate.room_1_ac`
> - 灯光：`light.room_1_light`

### 一整套可复制配置

```yaml
input_boolean:
  pc_a:
    name: 房间1电脑A
    icon: mdi:monitor
  pc_b:
    name: 房间1电脑B
    icon: mdi:monitor
  pc_c:
    name: 房间1电脑C
    icon: mdi:monitor

group:
  room_1_pcs:
    name: 房间1电脑组
    entities:
      - input_boolean.pc_a
      - input_boolean.pc_b
      - input_boolean.pc_c

automation:
  - alias: 房间1-电脑开机开启空调和灯
    id: room_1_pcs_on
    mode: single
    trigger:
      - platform: state
        entity_id:
          - input_boolean.pc_a
          - input_boolean.pc_b
          - input_boolean.pc_c
        to: "on"
    action:
      - service: climate.turn_on
        target:
          entity_id: climate.room_1_ac
      - service: light.turn_on
        target:
          entity_id: light.room_1_light

  - alias: 房间1-全部电脑关闭后关闭空调和灯
    id: room_1_pcs_off
    mode: single
    trigger:
      - platform: state
        entity_id: group.room_1_pcs
        to: "off"
    action:
      - service: climate.turn_off
        target:
          entity_id: climate.room_1_ac
      - service: light.turn_off
        target:
          entity_id: light.room_1_light
```

### 多房间模板版

如果你有多个房间，可以按下面的方式复制一份模板，只需要替换房间编号和实体 ID。

```yaml
input_boolean:
  room_1_pc_a:
    name: 房间1电脑A
    icon: mdi:monitor
  room_1_pc_b:
    name: 房间1电脑B
    icon: mdi:monitor

  room_2_pc_a:
    name: 房间2电脑A
    icon: mdi:monitor
  room_2_pc_b:
    name: 房间2电脑B
    icon: mdi:monitor

group:
  room_1_pcs:
    name: 房间1电脑组
    entities:
      - input_boolean.room_1_pc_a
      - input_boolean.room_1_pc_b

  room_2_pcs:
    name: 房间2电脑组
    entities:
      - input_boolean.room_2_pc_a
      - input_boolean.room_2_pc_b

automation:
  - alias: 房间1-电脑开机开启空调和灯
    id: room_1_pcs_on
    mode: single
    trigger:
      - platform: state
        entity_id:
          - input_boolean.room_1_pc_a
          - input_boolean.room_1_pc_b
        to: "on"
    action:
      - service: climate.turn_on
        target:
          entity_id: climate.room_1_ac
      - service: light.turn_on
        target:
          entity_id: light.room_1_light

  - alias: 房间1-全部电脑关闭后关闭空调和灯
    id: room_1_pcs_off
    mode: single
    trigger:
      - platform: state
        entity_id: group.room_1_pcs
        to: "off"
    action:
      - service: climate.turn_off
        target:
          entity_id: climate.room_1_ac
      - service: light.turn_off
        target:
          entity_id: light.room_1_light

  - alias: 房间2-电脑开机开启空调和灯
    id: room_2_pcs_on
    mode: single
    trigger:
      - platform: state
        entity_id:
          - input_boolean.room_2_pc_a
          - input_boolean.room_2_pc_b
        to: "on"
    action:
      - service: climate.turn_on
        target:
          entity_id: climate.room_2_ac
      - service: light.turn_on
        target:
          entity_id: light.room_2_light

  - alias: 房间2-全部电脑关闭后关闭空调和灯
    id: room_2_pcs_off
    mode: single
    trigger:
      - platform: state
        entity_id: group.room_2_pcs
        to: "off"
    action:
      - service: climate.turn_off
        target:
          entity_id: climate.room_2_ac
      - service: light.turn_off
        target:
          entity_id: light.room_2_light
```

## 这份 YAML 怎么放到 Home Assistant

你可以用下面两种方式之一：

### 方式 1：直接写到 `configuration.yaml`

把 `input_boolean`、`group`、`automation` 三段内容分别放到 Home Assistant 的主配置中。

适合：

- 测试环境
- 配置量不大
- 想先快速跑通

### 方式 2：拆分成独立文件再 `!include`

推荐这种方式，后期维护更清晰。

例如：

```yaml
input_boolean: !include input_boolean.yaml
group: !include groups.yaml
automation: !include automations.yaml
```

然后分别创建：

- `input_boolean.yaml`
- `groups.yaml`
- `automations.yaml`

把对应内容放进去。

### 配置建议

- 每个房间单独一组 `group.xxx`
- 每台电脑一个 `input_boolean`
- 自动化里尽量使用明确的实体名，不要混用模糊命名
- 如果后面要扩展灯光、空调、投影仪等设备，建议每类设备都单独做自动化

## 客户端该怎么上报 on/off

Windows 客户端不需要判断“是不是最后一台”，只需要上报自己的状态：

### 开机时

客户端启动后：

1. 读取自己的 `config.json`
2. 调用 Home Assistant 的 `input_boolean/turn_on`
3. 把自己的 `pc_entity_id` 置为 `on`

这表示：

- 这台电脑在线
- 房间里至少有一台电脑在用

### 关机时

Windows 关机前：

1. 拦截 `WM_QUERYENDSESSION`
2. 调用 Home Assistant 的 `input_boolean/turn_off`
3. 把自己的 `pc_entity_id` 置为 `off`

这表示：

- 这台电脑离线
- 如果组里所有电脑都离线了，HA 自动化就会关闭空调和灯

### 客户端配置中需要什么

每台客户端只需要知道自己的：

- `ha_url`
- `token`
- `pc_entity_id`

它不需要知道：

- 同房间还有几台电脑
- 自己是不是最后一台
- 什么时候该关空调和灯

这些都交给 Home Assistant 侧的 `group` 和自动化处理。

### 1. 组定义

```yaml
group:
  room_1_pcs:
    name: 房间1电脑组
    entities:
      - input_boolean.pc_a
      - input_boolean.pc_b
      - input_boolean.pc_c
```

### 2. 开机自动化

当任意电脑开机，把自己的开关置 `on` 后，触发空调和灯打开：

```yaml
automation:
  - alias: 房间1-电脑开机开启空调和灯
    mode: single
    trigger:
      - platform: state
        entity_id:
          - input_boolean.pc_a
          - input_boolean.pc_b
          - input_boolean.pc_c
        to: "on"
    action:
      - service: climate.turn_on
        target:
          entity_id: climate.room_1_ac
      - service: light.turn_on
        target:
          entity_id: light.room_1_light
```

### 3. 关机自动化

当整个组都变成 `off`，说明所有电脑都关机了，触发空调和灯关闭：

```yaml
automation:
  - alias: 房间1-全部电脑关闭后关闭空调和灯
    mode: single
    trigger:
      - platform: state
        entity_id: group.room_1_pcs
        to: "off"
    action:
      - service: climate.turn_off
        target:
          entity_id: climate.room_1_ac
      - service: light.turn_off
        target:
          entity_id: light.room_1_light
```

## 客户端职责

Windows 客户端只做两件事：

- 启动时把自己的 `input_boolean` 置为 `on`
- 关机时把自己的 `input_boolean` 置为 `off`

不要让客户端判断：

- 组里还有几台电脑在线
- 自己是不是最后一台
- 现在是否该关闭空调和灯

这些都交给 Home Assistant 处理。

## 优点

- 不依赖客户端之间同步
- 状态判断集中在 Home Assistant
- 逻辑更可靠，更适合网吧环境
- 某台电脑掉线不影响整体策略

## 备注

- 如果你要区分不同房间，只要为每个房间各建一个组即可
- 如果房间里电脑数量变化，只需要更新对应组里的实体列表
- 如果后续想更细，可以把“开空调”和“开灯”拆成不同自动化
