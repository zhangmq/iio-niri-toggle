# iio-niri-toggle

基于 [iio-sensor-proxy](https://gitlab.freedesktop.org/hadess/iio-sensor-proxy/) 为 [niri](https://github.com/niri-wm/niri) Wayland 合成器提供屏幕自动旋转功能。

## 背景

专用于在 x86 二合一平板上使用 niri + [DankMaterialShell](https://danklinux.com/)，补充缺失的屏幕自动旋转能力。

最初参考 [iio-niri](https://github.com/Zhaith-Izaliel/iio-niri) 开发，后因会话切换处理、greetd 兼容性、跨 session 状态持久化等实际需求，自行实现为单一二进制守护进程。

启动时通过 sysfs 自动检测内置屏幕（eDP/DSI/LVDS）。未检测到内屏时退出报错。

## 环境依赖

- [niri](https://github.com/niri-wm/niri) Wayland 合成器
- [iio-sensor-proxy](https://gitlab.freedesktop.org/hadess/iio-sensor-proxy/) — 硬件传感器守护进程
- [DankMaterialShell](https://danklinux.com/) + Quickshell — 面板插件（可选）
- systemd — 服务管理

## 开发依赖

- Rust ≥ 1.91（edition 2021）
- libdbus 开发头文件（`pkg-config` 支持）

## 安装

### 守护进程

```bash
cd iio-niri-toggle && cargo build --release
sudo bash files/install.sh
```

### DMS 插件（可选）

```bash
ln -s "$PWD" ~/.config/DankMaterialShell/plugins/iio-niri-toggle
```

然后在 DMS 面板配置中添加 `iio-niri-toggle` 插件。

插件出现在两个位置：

- **DankBar（面板）** — 点击切换自动旋转开关
- **控制中心** — 打开控制中心，点击编辑按钮进入编辑模式，在可用组件列表中找到"屏幕旋转"磁贴并添加。点击磁贴切换。

## 使用

| 命令 | 说明 |
|------|------|
| `iio-niri-toggle daemon` | 启动守护进程（由 systemd 管理） |
| `iio-niri-toggle lock` | 锁定当前屏幕方向 |
| `iio-niri-toggle unlock` | 恢复自动旋转 |
| `iio-niri-toggle status` | 查看当前状态 |
| `iio-niri-toggle toggle` | 切换锁定/自动旋转 |

## 架构

单一 Rust 二进制，基于 poll 的事件循环（200ms 超时），单线程整合以下模块：

- **D-Bus** — 连接 iio-sensor-proxy，订阅 `AccelerometerOrientation` 变化信号
- **inotify** — 监听状态文件变更和 `/run/user/` 中 niri socket 生命周期
- **IPC** — Unix 域 socket，处理 lock/unlock/status 命令
- **niri CLI** — 通过 `niri msg output <monitor> transform <tr>` 应用变换
- **健康检查** — 每 30 秒重新查询传感器方向，兜底丢失的信号

### 状态机

两种模式：
- **自动旋转** — 变换由实时传感器方向驱动（D-Bus 信号）
- **锁定** — 变换固定为持久化的值，忽略传感器变化

状态持久化到 `/var/lib/iio-niri-toggle/state.json`。apply 块为只读操作（不写 state.json）。

## 协作

本文档为个人自用工具，暂无协作开发计划。欢迎 fork 后自行修改适配。

## 许可

MIT
