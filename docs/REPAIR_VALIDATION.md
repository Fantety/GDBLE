# 0.6.0 修复与验收记录

基线提交：`864652a2d4d03971de07ebe7152b6e9b6914fd18`。版本保持 `0.6.0`；没有创建 tag 或发布 Release。此前的依赖与文档更新已保留。

本次源码内容 SHA-256：`b46c7421d90b6460874aa788ccbc169fbbaa5dd3be5eaa3e53663d63b6131ccb`。构建与发行提交可以不同，以该摘要和各产物哈希核验一致性。

## 四批改动

1. **回调与断连**：Manager 的设备信号调用持有重入保护；连接代际独立于请求 ID。主动断连回声、重复/迟到事件与真实远端断连分别处理，疑似旧事件通过有超时的连接状态查询核验。
2. **后端与通知**：补齐四类广播更新、适配器实际状态和服务变化；独立快照任务避免阻塞控制事件。Android 使用 JNI 查询真实电源状态并桥接广播，解决 btleplug 0.12 固定返回 Unknown 的限制。订阅前准备通知接收器，限额暂存，失败回滚；溢出停止通知、保留连接，重订阅重建 pump。
3. **背压**：请求接纳前预留 started/terminal 容量；会话预留异常状态槽；控制循环不等待外发事件。控制与数据分别限容，扫描按设备合并并报告淘汰，过期扫描/通知数据在交付前被过滤。
4. **Android 与发行**：共享权限启动器、中英文示例、ADR 0005、内嵌源码摘要、最终产物校验及双阶段发布门禁。正式库不含测试注入接口。

## 已通过的检查

| 检查 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过 |
| 严格 Clippy，全部 targets/features，独立 `target/repair-build` | 通过 |
| Android ARM64 严格 Clippy、Kotlin 编译 | 通过 |
| Rust 测试 | Windows 57 项、Linux 56 项通过（Windows 多一项平台事件映射测试） |
| Python 测试 | 15 项通过；含陈旧库、缺 AAR、错误摘要/哈希、测试库禁止发行、最终库不重编译 |
| PowerShell 构建失败回归 | 5 类失败均保留已有产物、正确恢复目录 |
| 生命周期静态检查、actionlint、0.6.0 release notes 校验 | 通过 |
| Godot 4.2.2 / 4.6.1，新源码冒烟 | Windows 通过 |
| Godot 4.2.2 / 4.6.1，后端注入成功路径 | Windows 通过；读取 Manager、重入提交、其他设备操作、queue_free、保留设备拒绝 |
| Godot 4.2.2 / 4.6.1，权限状态机 | 通过；API 23/30/31/35，已授权、首次授权、部分/永久拒绝、前后台切换 |
| 最终 Windows DLL 原样冒烟 | 两个 Godot 版本均通过 |
| 最终 Linux SO 原样冒烟 | 两个 Godot 版本均通过 |
| Android 双 ABI 与两份 AAR | 重新构建通过；JNI 导出、插件方法、manifest、桥接类及内嵌源码摘要校验通过 |
| `BUILD_INFO.json` / `SHA256SUMS` | 已生成；Windows/Linux/Android 摘要与源码一致，完整文件哈希清单核验通过 |
| macOS x86_64 / ARM64 | `cargo check` 通过；未完成原生链接与加载验收 |

上述 Godot 夹具使用独立 Cargo 构建目录。`--mode artifact` 只复制最终 addon 到临时测试项目，绝不调用 Cargo；两个桌面发行库的内嵌摘要与当前源码一致。

## 发行缺口

- macOS 两份历史库尚未重新构建。本机没有 macOS SDK/运行环境；需要在原生 CI 完成构建与最终 addon 冒烟。源码检查不能替代该项。
- `adb devices -l` 未发现设备。API 23–30、API 31+ ARM64 真机 BLE 全流程和 x86_64 模拟器加载均未验收；不能用 AAR 结构校验替代。

最终 `BUILD_INFO.json` 的 `incomplete` 精确列出两份 macOS 旧库，`release_ready` 为 `false`。实际执行完整发行校验时，门禁按预期拒绝该不完整 addon。Windows、Linux 和 Android 产物均已更新，不使用旧库冒充新构建。

在平台产物和设备证据补齐之前，不应创建正式 tag。`artifact_provenance.py validate` 会拒绝陈旧或不完整的 addon；`--allow-incomplete` 仅用于明确记录本地进度，不会绕过发布门禁。

## 构建环境

- Windows：Rust 1.99.0，MSVC x86_64。
- Linux：WSL Ubuntu，Rust 1.99.1，x86_64 GNU。
- Android：Rust 1.99.0，NDK 28.2.13676358，SDK 34，Zulu JDK 17.0.20.1，Gradle 8.9（发行包 SHA-256 已与官方校验值比对）。

完整设计与容量规则见 [ADR 0005](adr/0005-session-epochs-and-event-reservations.md)，发行步骤见 [RELEASING](RELEASING.md)。
