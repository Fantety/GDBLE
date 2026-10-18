# 0.6.0 修复与验收记录

## 2026-10-18：GitHub Actions 发布失败修复

[失败运行 39100223795](https://github.com/Fantety/GDBLE/actions/runs/39100223795) 的 8 个最终 addon 冒烟 job 均失败。共同原因是 `v0.6.0` 指向 `20ed4e0`，该提交的两份 macOS 库尚未更新，清单明确为 `release_ready=false`。Windows 在首条校验命令失败后还继续执行，暴露出第二个问题：setup-godot 创建没有 `.exe` 后缀的别名，Python 通过裸 `godot` 名称无法在 PATH 中找到它。

本次将完整平台构建与组装抽成 CI/Prepare 共用流程，将 4 个 OS/架构 × 2 个 Godot 版本的最终产物校验抽成 CI/Prepare/Publish 共用流程。校验和启动拆为独立步骤；Godot 启动器优先读取 action 导出的绝对路径；未准备好的标签在启动矩阵前失败并列出具体缺失产物。原始标签不会通过重跑自动获得主分支的新代码或产物。

修复提交 `690a833` 已推送；[GitHub CI 39101581741](https://github.com/Fantety/GDBLE/actions/runs/39101581741) 全部通过，包括 Windows、Linux、macOS x86_64/ARM64 原生构建、Android 双 ABI/AAR、组装后的完整 provenance 校验及全部 8 个 `final-addon-smoke` job。Windows 两个 Godot 版本均已在真实 GitHub runner 验证，不以本地测试替代。本地 actionlint、25 项 Python 回归及两版真实 Godot 的无扩展名别名启动也通过。硬件 BLE 与 Android 模拟器验收仍未完成。

已将该运行验证过的 `assembled-addon` 原样回写到 `addons/gdble`，包含两份新 macOS 库。GitHub artifact ID 为 `12127166311`，下载归档 SHA-256 与 GitHub API 公布值一致：`abdbc6215d1ed5a66700faa88d1553afb64961615cee5aab62ab273479fe6807`。回写后完整 provenance、AAR 结构和全部文件哈希校验通过，`release_ready=true`、`incomplete=[]`；这表示产物齐备，不代替硬件验收。内嵌构建提交为 `690a833`，源码 SHA-256 仍为 `eac97287ca3f62975d079b9e2b2f2c67984314d6f2f3282d0e9cacdcb6fc023e`。

## 2026-10-18：公开仓库复查修复

本轮基线：`13c49c7`。版本保持 `0.6.0`，未创建 tag 或发布 Release。当前源码内容 SHA-256：`eac97287ca3f62975d079b9e2b2f2c67984314d6f2f3282d0e9cacdcb6fc023e`。

### 修复内容

- 停止扫描直接读取核心的未结束请求登记，取消调用前已提交的扫描；旧终止事件未被 Godot 消费时也能正确停止新扫描。补测空闲、排队、重复提交、停止后新提交与主线程事件积压。
- 取消订阅先验证完整 GATT 身份；未订阅目标幂等成功且不调用后端。补测跨服务同名特征、不同特征、重复取消、后端失败保留订阅及订阅迁移后的迟到取消。
- 根目录和 addon 均携带同一许可证；实际 Git archive 回归覆盖归档内容，产物校验拒绝缺失或不一致的许可证。生成清单固定使用 LF，避免 Windows 的 CRLF 在 Git 提交时被规范化而破坏哈希。
- 中英文 README 区分构建、加载与实机验收；修正 `powered_on` 字符串载荷和 `resolve_device` 枚举。新增 [接口契约](API_CONTRACT.md)，说明请求关联、事件形状、错误码、事件转发和缓存生命周期。Release 包包含 docs。
- Actions 发布复查：产物自动提交到 release 分支并创建 PR，回读实际提交校验后主动 dispatch CI；补齐并发保护、构建期间源码变动检查、原生 Intel macOS 构建和仓库权限说明。标签发布提前校验 notes/version，正确标记 prerelease，缺失上传文件和旧格式产物均显式失败。

### 本轮已执行的检查

| 检查 | 结果 |
| --- | --- |
| fmt、严格 Clippy（全部 targets/features） | Windows 通过 |
| Rust 测试 | Windows 60 项、Linux 59 项通过 |
| Python 测试 | 21 项通过，含真实 Git archive、许可证校验、生成清单换行规范化及 Release 元数据输出 |
| 生命周期检查、actionlint、release notes | 通过 |
| 发布工作流 Bash 与归档 | 实际脚本接受两组合法布局，拒绝 10 组旧产物和已跟踪的 Demo addon；ZIP 内容核验通过 |
| PowerShell 构建失败回归 | 5 类失败均通过 |
| Godot 4.2.2 / 4.6.1 后端注入测试 | Windows 通过；新增扫描取消、同步拒绝的延迟请求关联、String 原因载荷 |
| Godot 4.6.1 source / debugger / permissions | Windows 通过 |
| 最终 Windows DLL / Linux SO | 两个 Godot 版本原样加载均通过，不重编译测试产物 |
| Android ARM64 / x86_64、双 AAR | Rust release 与 Gradle 构建通过；AAR 结构和内嵌摘要校验通过 |
| macOS x86_64 / ARM64 | 本轮源码 `cargo check` 均通过；原生链接和加载验收仍未完成 |
| BUILD_INFO / SHA256SUMS | 已更新；可用平台摘要与当前源码一致，全部文件哈希及许可证一致性核验通过 |

### 尚未完成的发行验收

当前主机没有 macOS 原生构建与运行环境，两份 macOS 库仍为历史产物；`BUILD_INFO.json.incomplete` 明确列出它们，`release_ready` 保持 `false`。完整发行校验继续拒绝该目录。`adb devices -l` 没有连接设备，API 23–30、API 31+ ARM64 真机 BLE 流程及 x86_64 模拟器加载尚未验收。上述缺口需要对应 CI/硬件补齐，不能以源码测试替代。

2026-10-18 通过公开 GitHub API 检查：远端 `master` 仍为 `13c49c7`，[最近 CI](https://github.com/Fantety/GDBLE/actions/runs/38913412592) 成功；最新 Release 为 `v0.5.5`。本次工作流修订尚未在 GitHub 执行，新版 Prepare/Publish 发布链路没有实际运行验收记录。仓库是否允许 Actions 创建 PR 无法从公开仓库信息确认，首次运行前需按 RELEASING 中的仓库设置核对。

## 历史验收：此前的核心重构

以下摘要与测试数对应此前修复，不代表本轮源码；当前结果以上方记录和 `BUILD_INFO.json` 为准。

基线提交：`864652a2d4d03971de07ebe7152b6e9b6914fd18`。版本保持 `0.6.0`；没有创建 tag 或发布 Release。此前的依赖与文档更新已保留。

本次源码内容 SHA-256：`b46c7421d90b6460874aa788ccbc169fbbaa5dd3be5eaa3e53663d63b6131ccb`。构建与发行提交可以不同，以该摘要和各产物哈希核验一致性。

### 四批改动

1. **回调与断连**：Manager 的设备信号调用持有重入保护；连接代际独立于请求 ID。主动断连回声、重复/迟到事件与真实远端断连分别处理，疑似旧事件通过有超时的连接状态查询核验。
2. **后端与通知**：补齐四类广播更新、适配器实际状态和服务变化；独立快照任务避免阻塞控制事件。Android 使用 JNI 查询真实电源状态并桥接广播，解决 btleplug 0.12 固定返回 Unknown 的限制。订阅前准备通知接收器，限额暂存，失败回滚；溢出停止通知、保留连接，重订阅重建 pump。
3. **背压**：请求接纳前预留 started/terminal 容量；会话预留异常状态槽；控制循环不等待外发事件。控制与数据分别限容，扫描按设备合并并报告淘汰，过期扫描/通知数据在交付前被过滤。
4. **Android 与发行**：共享权限启动器、中英文示例、ADR 0005、内嵌源码摘要、最终产物校验及双阶段发布门禁。正式库不含测试注入接口。

### 已通过的检查

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

### 发行缺口

- macOS 两份历史库尚未重新构建。本机没有 macOS SDK/运行环境；需要在原生 CI 完成构建与最终 addon 冒烟。源码检查不能替代该项。
- `adb devices -l` 未发现设备。API 23–30、API 31+ ARM64 真机 BLE 全流程和 x86_64 模拟器加载均未验收；不能用 AAR 结构校验替代。

最终 `BUILD_INFO.json` 的 `incomplete` 精确列出两份 macOS 旧库，`release_ready` 为 `false`。实际执行完整发行校验时，门禁按预期拒绝该不完整 addon。Windows、Linux 和 Android 产物均已更新，不使用旧库冒充新构建。

在平台产物和设备证据补齐之前，不应创建正式 tag。`artifact_provenance.py validate` 会拒绝陈旧或不完整的 addon；`--allow-incomplete` 仅用于明确记录本地进度，不会绕过发布门禁。

### 构建环境

- Windows：Rust 1.99.0，MSVC x86_64。
- Linux：WSL Ubuntu，Rust 1.99.1，x86_64 GNU。
- Android：Rust 1.99.0，NDK 28.2.13676358，SDK 34，Zulu JDK 17.0.20.1，Gradle 8.9（发行包 SHA-256 已与官方校验值比对）。

完整设计与容量规则见 [ADR 0005](adr/0005-session-epochs-and-event-reservations.md)，发行步骤见 [RELEASING](RELEASING.md)。
