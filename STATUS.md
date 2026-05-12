# iio-niri-toggle — 项目状态

## 架构

```
┌─ iio-niri-toggle（单一 Rust 二进制）──────────────────────────┐
│                                                               │
│  poll(inotify_fd, ipc_fd, 200ms)                              │
│    ├─ inotify（state.json 变更）→ need_apply = true           │
│    ├─ IPC（lock/unlock/status）→ write_state + need_apply     │
│    ├─ D-Bus（PropertiesChanged）→ 更新缓存方向                │
│    ├─ socket 变化（glob）→ need_apply = true                  │
│    ├─ 30s 健康检查（仅自动旋转）→ 更新方向缓存               │
│    └─ apply 块（纯应用，无副作用）                            │
│                                                               │
│  D-Bus 连接: ClaimAccelerometer, 订阅信号                     │
│  状态文件: /var/lib/iio-niri-toggle/state.json                │
│  IPC: Unix socket /var/run/iio-niri-toggle.sock               │
│  CLI: iio-niri-toggle {daemon|send|lock|unlock|status}        │
└───────────────────────────────────────────────────────────────┘
```

## 功能规格

### 两种模式

#### 自动旋转模式

| 维度 | 规则 |
|------|------|
| 状态标记 | `auto_rotate = true`, `locked_transform = null` |
| 变换来源 | 实时传感器方向（通过 D-Bus `AccelerometerOrientation`） |
| 传感器依赖 | 强依赖——每次变换均由传感器信号驱动 |
| state.json 依赖 | 无——变换完全由传感器推导 |

| 事件 | 行为 | 是否写 state.json |
|------|------|:---:|
| D-Bus `PropertiesChanged` | 更新缓存方向 → apply | 否 |
| 新 niri socket（session 切换） | apply 当前缓存方向 | 否 |
| IPC `unlock` 命令 | 写 `auto_rotate=true, locked_transform=null` → apply 缓存方向 | 是（一次性） |
| 30s 健康检查（信号丢失兜底） | `requery_orientation` → 有变化则 apply | 否 |

#### 锁定模式

| 维度 | 规则 |
|------|------|
| 状态标记 | `auto_rotate = false`, `locked_transform` ≠ 空 |
| 变换来源 | state.json 中的 `locked_transform`（持久化固定值） |
| 传感器依赖 | **无依赖**——传感器信号在锁定模式下被忽略 |
| state.json 依赖 | 依赖——`locked_transform` 必须跨 session 持久化 |

| 事件 | 行为 | 是否写 state.json |
|------|------|:---:|
| IPC `lock` 命令 | 捕获当前 niri 变换 → 写 `locked_transform` → apply | 是（一次性） |
| 新 niri socket（session 切换） | 读 `locked_transform` → apply | 否 |
| 传感器变化 | **忽略** | 否 |
| 健康检查 | **跳过**（锁定模式无需关注传感器） | 否 |

### state.json 写入规则

**仅在下列时机写入，其余任何时候不写：**

| 触发 | 写入内容 |
|------|----------|
| 首次启动（无 state.json） | `{"auto_rotate": true, "locked_transform": null, "monitor": "eDP-1"}` |
| IPC `lock` | `{"auto_rotate": false, "locked_transform": "<当前变换>", "monitor": "eDP-1"}` |
| IPC `unlock` | `{"auto_rotate": true, "locked_transform": null, "monitor": "eDP-1"}` |

**禁止：** apply 块不得写 state.json。

### 关键设计约束

1. **apply 块纯应用**：不写 state.json、不调 D-Bus、不触发其他事件
2. **锁定模式忽略传感器**：D-Bus 信号照收（更新缓存方向），但不设 `need_apply`
3. **socket 重试**：apply 失败 + socket 存在 → 保留 `need_apply` 下轮重试；apply 失败 + 无 socket → 放弃
4. **IPC 同步处理**：lock/unlock 在 IPC 处理器内完成 `write_state`；apply 随后在下轮 poll 迭代处理

### 已知问题

- `/run/user/` 的 inotify 仅监视直接子条目。socket 在已有用户目录内创建（同 UID greetd）时，依赖 200ms poll 超时作为兜底检测，延迟 ≤200ms。

### 关键路径

| 变量 | 路径 |
|------|------|
| state.json | `/var/lib/iio-niri-toggle/state.json` |
| IPC socket | `/var/run/iio-niri-toggle.sock` |
| 二进制 | `/usr/local/bin/iio-niri-toggle` |
| 服务 | `/etc/systemd/system/iio-niri-toggle.service` |

### state.json 格式

```json
{
  "auto_rotate": true,
  "locked_transform": null,
  "monitor": "eDP-1"
}
```

- `auto_rotate`: true = 自动旋转模式，false = 锁定模式
- `locked_transform`: `"normal"` | `"90"` | `"180"` | `"270"` | `null`
