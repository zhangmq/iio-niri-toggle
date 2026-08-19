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

### 方式一：Release 安装脚本（推荐）

```bash
curl -LO https://raw.githubusercontent.com/zhangmq/iio-niri-toggle/main/deploy/install-release.sh
bash install-release.sh        # 可加版本号参数，默认安装最新 release
```

自动下载最新 release、校验 SHA256SUMS、解压并安装（安装时会询问 DMS 插件安装位置）。

### 方式二：手动安装（从仓库）

```bash
cargo build --release
sudo bash deploy/install.sh
```

### DMS 插件（可选）

`install.sh` 会一并安装插件，安装时询问位置：
- **用户级**（默认）— `~/.config/DankMaterialShell/plugins/iio-niri-toggle`
- **系统级** — `/etc/xdg/quickshell/dms-plugins/iio-niri-toggle`

手动安装：

```bash
# 用户级
cp -r plugin ~/.config/DankMaterialShell/plugins/iio-niri-toggle
# 系统级
sudo cp -r plugin /etc/xdg/quickshell/dms-plugins/iio-niri-toggle
```

然后在 DMS 面板配置中添加 `iio-niri-toggle` 插件。

插件出现在两个位置：

- **DankBar（面板）** — 点击切换自动旋转开关
- **控制中心** — 打开控制中心，点击编辑按钮进入编辑模式，在可用组件列表中找到"屏幕旋转"磁贴并添加。点击磁贴切换。

插件文案跟随系统语言（简体中文 / English）。

## 卸载

```bash
curl -LO https://raw.githubusercontent.com/zhangmq/iio-niri-toggle/main/deploy/uninstall.sh
bash uninstall.sh        # 加 -y 跳过确认
```

移除 systemd 服务、二进制、状态与 DMS 插件（系统级 + 用户级）。

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
- **systemd 加固** — 因需连接登录用户 `/run/user/<uid>` 下的 niri socket，守护进程以 root 运行；unit 内置沙箱（`ProtectSystem`、`PrivateNetwork`、capability 裁剪等），将权限收敛到最小

### 状态机

两种模式：
- **自动旋转** — 变换由实时传感器方向驱动（D-Bus 信号）
- **锁定** — 变换固定为持久化的值，忽略传感器变化

状态持久化到 `/var/lib/iio-niri-toggle/state.json`。apply 块为只读操作（不写 state.json）。

## 已知限制

- **DMS 控制中心插件懒加载**：如果只在控制中心启用磁贴（DankBar 面板未启用），插件 QML 实例在控制中心首次弹出前不会激活。此时通过 `iio-niri-toggle lock/unlock` CLI 操作仍正常，但 Toast 通知不会显示。这是 DMS/Quickshell 的行为——无面板 pille 的插件需等到控制中心渲染后才会实例化。

## 发布 Release

打版本标签即自动构建并发布：

```bash
git tag v1.0.1 && git push origin v1.0.1
```

GitHub Actions（archlinux 容器）打包 `iio-niri-toggle-<版本>-x86_64-unknown-linux-gnu.tar.gz`（含二进制、systemd unit、安装/卸载脚本、DMS 插件与文档）并附带 `SHA256SUMS` 校验和，自动创建 GitHub Release。

### 从 Release 安装

```bash
# 下载并校验
curl -LO https://github.com/zhangmq/iio-niri-toggle/releases/download/v1.0.1/iio-niri-toggle-1.0.1-x86_64-unknown-linux-gnu.tar.gz
curl -LO https://github.com/zhangmq/iio-niri-toggle/releases/download/v1.0.1/SHA256SUMS
sha256sum -c SHA256SUMS

# 解压并安装
tar xzf iio-niri-toggle-1.0.1-x86_64-unknown-linux-gnu.tar.gz
cd iio-niri-toggle-1.0.1-x86_64-unknown-linux-gnu
sudo bash install.sh
```

运行时依赖：glibc、libdbus-1（Arch 系系统默认自带）。

## 协作

本文档为个人自用工具，暂无协作开发计划。欢迎 fork 后自行修改适配。

## 许可

MIT
