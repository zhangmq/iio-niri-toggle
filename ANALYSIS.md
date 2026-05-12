# 合并 iio-niri-toggle + iio-niri-listener 为单一 Rust 二进制

## 现状架构

```
bash daemon (~380 行)
  ├─ 状态管理: read_json_field / write_state (Python one-liner)
  ├─ session 管理: wait_for_sock → apply_session → monitor loop
  ├─ IPC 服务器: 内嵌 Python Unix socket 进程 (per-session)
  ├─ CLI 客户端: socat → IPC socket
  └─ fork 管理: 启动/停止 Rust listener 进程

Rust listener (~112 行, 3 deps)
  ├─ D-Bus: 连接 system bus, ClaimAccelerometer
  ├─ 传感器轮询: conn.process(1000ms) → 读方向
  ├─ 状态读取: serde_json 解析 state.json
  └─ 变换应用: find_niri_socket → niri msg

依赖:
  bash + python3 + socat + inotifywait + find + niri msg
  + rust (listener: dbus + serde_json + glob)
```

## 合并后架构

```
单一 Rust 二进制 (~500-600 行, 6-8 deps)

主线程:
  ├─ D-Bus: 连接 system bus, ClaimAccelerometer
  ├─ 传感器轮询: conn.process(1000ms) → 读方向 → niri msg
  ├─ inotify: 监视 state.json + socket 生命周期 + /run/user/
  │   (替代 bash 的 inotifywait + Rust 的 D-Bus 分离)
  └─ session 管理: 等待 socket → apply → 监视变更

IPC 线程:
  ├─ UnixListener: 接受客户端连接
  ├─ 处理 lock/unlock/status: 读写 state.json
  └─ 通过共享状态通知主线程

CLI 子命令:
  ├─ daemon           ← 守护进程模式
  ├─ send lock/unlock ← 客户端模式 (UnixStream)
  └─ lock/unlock/toggle/status (快捷方式)
```

## 每个功能的代价评估

| 功能 | 当前实现 | Rust 实现代价 | 难度 |
|------|----------|---------------|------|
| state.json 读写 | Python one-liner | serde::Deserialize + serialize, ~20行 | 低 |
| 显示器检测 (sysfs) | cat + for 循环 | std::fs::read_dir + 字符串匹配, ~30行 | 低 |
| 当前变换查询 | niri msg \| python parse | Command + serde_json, ~15行 (已有) | 低 |
| inotify 等待 socket | inotifywait 命令 | inotify crate, ~60行 | 中 |
| inotify 监视 state.json | inotifywait | inotify crate, ~20行 | 低 |
| IPC Unix socket | 内嵌 Python 进程 (50行) | std::os::unix::net::UnixListener, ~80行 | 低 |
| CLI 参数解析 | case 分发 | clap 或手动 argv, ~30行 | 低 |
| D-Bus 传感器 | dbus crate (已有) | 保持不变 | 已有 |
| 并发事件循环 | bash 顺序阻塞调用 | 线程 (thread) 或 async (tokio) | **高** |

## 核心难点：事件循环合并

当前两个进程各自有独立事件循环：

```
bash daemon 循环:
  wait_for_sock (inotifywait blocking)
  → apply_session
  → monitor (inotifywait on state.json + socket, blocking)

Rust listener 循环:
  conn.process(1000ms, blocking)
  → read_config + apply
```

合并后单个进程需要同时处理：

```
需要等待的事件:
  1. D-Bus 信号 (传感器方向变化)    ← 当前 listener 的 conn.process
  2. state.json 文件变更            ← 当前 daemon 的 inotifywait
  3. niri socket 删除               ← 当前 daemon 的 inotifywait
  4. /run/user/ 新 socket 创建      ← 当前 daemon 的 inotifywait
  5. IPC 客户端连接                 ← 当前 daemon 的 Python accept()
```

**方案对比：**

### A. 多线程 (推荐)
```
线程 1: D-Bus 轮询 (conn.process) + niri msg apply
线程 2: inotify 事件循环 (state.json + socket + /run/user/)
线程 3: IPC Unix socket accept loop
        ↑ 共享 Arc<Mutex<State>> 协调
```

- 开发量: ~500 行新代码
- 优点: 线程模型直观，std 库原生支持
- 缺点: 需要 Mutex 保护共享状态

### B. tokio async (最优但最重)
```
tokio::select! {
    dbus_signal => handle_orientation(),
    inotify_event => handle_fs_event(),
    ipc_conn => handle_client(),
    interval => poll_sensor(),
}
```

- 开发量: ~450 行
- 优点: 单线程无锁，最优雅的事件复用
- 缺点: 引入 tokio (+ ~40 deps)，dbus 的 blocking API 与 tokio 集成麻烦

### C. 单线程顺序轮询 (最简但有损)
```
loop {
    conn.process(200ms)   ← 短超时
    check_inotify_fd()
    check_ipc_accept()
    check_ipc_data()
}
```

- 开发量: ~400 行
- 优点: 最简单，不引入新依赖
- 缺点: 轮询替代事件驱动，CPU 开销略增，响应延迟增加 (~200ms)

## 依赖变更

| 当前 (Cargo.toml) | 合并后 |
|-------------------|--------|
| dbus | dbus |
| serde_json | serde_json |
| glob | glob |
| - | **inotify** (新增) |
| - | **clap** 或手动解析 (新增) |
| - | **serde** + derive (为状态结构体) |

从 3 个 crate → ~6 个 crate。

## 成本和收益总表

| 维度 | 代价 | 收益 |
|------|------|------|
| **代码量** | ~400-500 行新 Rust 代码 | 删除 ~380 行 bash + ~50 行内嵌 Python |
| **外部依赖** | 新增 inotify, clap 等 crate | **移除 python3, socat, inotifywait, find** |
| **进程数** | 1 进程 | 从 3 进程 → 1 进程 (bash + listener + Python IPC) |
| **内存** | ~10MB RSS (单二进制) | 当前 ~8MB (bash 1M + listener 7M) + Python ~5M (间歇) |
| **二进制大小** | ~1.5-2MB | 当前 ~812KB (listener) + 12KB (bash) |
| **事件模型** | 需设计并发事件循环 | 消除两个进程通过文件 (state.json) 的隐式 IPC |
| **可修改性** | 需 Rust 工具链 | 当前 bash 可直接修改 |
| **可调试性** | 需重编译 | 当前可加 echo 即时调试 |
| **安全性** | Rust 类型安全 | 消除 shell 注入风险 (python -c 变量插值) |
| **安装** | 1 个二进制 | 当前 2 个二进制 |

## 结论

**推荐的迁移路径：**

如果目标是简化维护和消除运行时依赖，值得合并。关键技术决策：

1. **事件模型**: 采用多线程方案（方案 A），D-Bus 主线程 + inotify 线程 + IPC 线程
2. **状态共享**: 使用 `Arc<Mutex<State>>` + `Condvar` 用于线程间通知
3. **IPC 接口**: 保持现有 Unix socket 协议不变，兼容 QML 插件
4. **CLI 接口**: 保持 `iio-niri-toggle daemon` / `iio-niri-toggle send lock` 等子命令不变

**不建议合并的情况：** 如果当前架构稳定无 Bug，合并带来的工程成本（事件循环设计、测试回归）可能超过收益。Python/socat 依赖已在运行系统上存在。可以考虑逐步替换：先内联 IPC socket 到 Rust，最后再合并 daemon 逻辑。
