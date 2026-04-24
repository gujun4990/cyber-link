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

## 环境配置

### 配置bp

创建文件：/opt/homeassistant/blueprints/automation/cyber_link/room_last_pc_control.yaml 内容如下。重启后验证：设置 → 自动化与场景 → 蓝图
```yaml
blueprint:
  name: 房间最后一台电脑离线控制空调和灯
  description: >
    房间级在线状态为 on 时，立即打开空调和已配置的灯；
    房间级在线状态为 off 并持续一段时间后，关闭空调和已配置的灯。
  domain: automation
  input:
    room_online_entity:
      name: 房间在线聚合实体
      description: 例如 binary_sensor.room1_any_pc_online
      selector:
        entity:
          domain: binary_sensor

    ac_entity:
      name: 空调实体
      default: []
      selector:
        entity:
          multiple: false
          domain: climate

    ambient_light_entity:
      name: 氛围灯实体
      default: []
      selector:
        entity:
          multiple: false

    main_light_entity:
      name: 主灯实体
      default: []
      selector:
        entity:
          multiple: false

    door_sign_light_entity:
      name: 门牌灯实体
      default: []
      selector:
        entity:
          multiple: false

    delay_seconds:
      name: 延迟关闭秒数
      default: 30
      selector:
        number:
          min: 0
          max: 600
          step: 1
          mode: box

mode: restart
max_exceeded: silent

trigger:
  - platform: state
    entity_id: !input room_online_entity
    to: "on"
    id: room_online

  - platform: state
    entity_id: !input room_online_entity
    to: "off"
    for:
      seconds: !input delay_seconds
    id: room_offline_delay

variables:
  ac_entity: !input ac_entity
  ambient_light_entity: !input ambient_light_entity
  main_light_entity: !input main_light_entity
  door_sign_light_entity: !input door_sign_light_entity

action:
  - choose:
      - conditions:
          - condition: trigger
            id: room_online
        sequence:
          - if:
              - condition: template
                value_template: "{{ ac_entity != [] and ac_entity != '' }}"
            then:
              - service: climate.turn_on
                target:
                  entity_id: !input ac_entity

          - if:
              - condition: template
                value_template: "{{ ambient_light_entity != [] and ambient_light_entity != '' }}"
            then:
              - service: homeassistant.turn_on
                target:
                  entity_id: !input ambient_light_entity

          - if:
              - condition: template
                value_template: "{{ main_light_entity != [] and main_light_entity != '' }}"
            then:
              - service: homeassistant.turn_on
                target:
                  entity_id: !input main_light_entity

          - if:
              - condition: template
                value_template: "{{ door_sign_light_entity != [] and door_sign_light_entity != '' }}"
            then:
              - service: homeassistant.turn_on
                target:
                  entity_id: !input door_sign_light_entity

      - conditions:
          - condition: trigger
            id: room_offline_delay
        sequence:
          - if:
              - condition: template
                value_template: "{{ ac_entity != [] and ac_entity != '' }}"
            then:
              - service: climate.turn_off
                target:
                  entity_id: !input ac_entity

          - if:
              - condition: template
                value_template: "{{ ambient_light_entity != [] and ambient_light_entity != '' }}"
            then:
              - service: homeassistant.turn_off
                target:
                  entity_id: !input ambient_light_entity

          - if:
              - condition: template
                value_template: "{{ main_light_entity != [] and main_light_entity != '' }}"
            then:
              - service: homeassistant.turn_off
                target:
                  entity_id: !input main_light_entity

          - if:
              - condition: template
                value_template: "{{ door_sign_light_entity != [] and door_sign_light_entity != '' }}"
            then:
              - service: homeassistant.turn_off
                target:
                  entity_id: !input door_sign_light_entity
```

### 配置实体

配置文件： /opt/homeassistant/configuration.yaml 内容如下。重启后验证：设置 → 设备与服务 → 实体
```yaml

# Loads default set of integrations. Do not remove.
default_config:

# Load frontend themes from the themes folder
frontend:
  themes: !include_dir_merge_named themes

automation: !include automations.yaml
script: !include scripts.yaml
scene: !include scenes.yaml

input_boolean:
  room1_pc_a_online:
    name: 房间1-PC-A在线
  room1_pc_b_online:
    name: 房间1-PC-B在线
  room1_pc_c_online:
    name: 房间1-PC-C在线

  room2_pc_a_online:
    name: 房间2-PC-A在线
  room2_pc_b_online:
    name: 房间2-PC-B在线

template:
  - binary_sensor:
      - name: 房间1任意电脑在线
        unique_id: room1_any_pc_online
        state: >
          {{
            is_state('input_boolean.room1_pc_a_online', 'on')
            or is_state('input_boolean.room1_pc_b_online', 'on')
            or is_state('input_boolean.room1_pc_c_online', 'on')
          }}

      - name: 房间2任意电脑在线
        unique_id: room2_any_pc_online
        state: >
          {{
            is_state('input_boolean.room2_pc_a_online', 'on')
            or is_state('input_boolean.room2_pc_b_online', 'on')
          }}
```

### 创建自动化

设置 → 自动化与场景 → 蓝图 -> 最右边“创建自动化”

选择如下：
- 房间在线聚合实体（这里就是我们上面创建“房间1任意电脑在线”和“房间2任意电脑在线”）
- 空调实体：选择对应房间的空调
- 氛围灯实体：选择对应房间的氛围灯
- 主灯实体：选择对应房间的主灯
- 门牌灯实体：选择对应房间的门牌灯
- 延迟关闭秒数：默认30s

具体参看该[图](./last-pc-ha-logic.png)

## 结论

这套方案保留了 `config.json` 的电脑级可选配置，同时把“最后一台电脑”的判断上移到 HA 的房间级聚合层。这样既能支持多房间，也能支持每个房间设备不完全相同的情况。
