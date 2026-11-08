# 一划：性能 Skill 筛选与取舍

检索日期：2026-11-08。项目栈：Rust 2021、Tauri 2（当前 Cargo.lock 为 2.12.5）、原生 TypeScript + Vite，无 React。

**本轮保留现有划词图标外观，让图标 WebView 按需创建、隐藏 30 秒后回收；流式译文只在前端做 32ms 合批。** 主窗、译文窗已有的关闭销毁策略和事件驱动划词继续沿用。下面 3 个 Skill 提供取舍依据；本次只阅读作者仓库原文与官方资料，未安装 Skill、未运行第三方脚本。

## 入选的 3 个 Skill

### 1. `tauri-performance-optimization`：主参考

- 作者仓库：[prodBirdy/tauri-performance-optimization-skill](https://github.com/prodBirdy/tauri-performance-optimization-skill)。[SKILL.md 原文](https://github.com/prodBirdy/tauri-performance-optimization-skill/blob/master/tauri-performance-skill/SKILL.md)。
- 实际内容：先看前端/WebView，再看 IPC、后端和 Rust；明确把 allocator、运行时线程数等调整放在 profiling 之后。覆盖窗口按需创建、流使用 channel、减少重复序列化。
- 用于一划：处理隐藏但常驻的图标窗口及重复渲染。图标首次需要时创建，等待 ready 握手后显示；隐藏后保留 30 秒供连续划词复用，再销毁回收。
- 取舍：30 秒保温期换取连续划词响应，超时重开要支付 WebView 初始化延迟。保留用户已满意的 28px 圆图标与 hover，避免原生重写的外观回归。React、Supabase、Hono、channel 迁移和其整段 release profile 本轮均不采用。

### 2. `m10-performance`：Rust 优化的刹车与顺序

- 作者仓库：[actionbook/rust-skills](https://github.com/actionbook/rust-skills)。[SKILL.md 原文](https://github.com/actionbook/rust-skills/blob/main/skills/m10-performance/SKILL.md)。
- 实际内容：先找实际热点、明确预算，再决定是否值得增加复杂度；分别考虑 CPU/内存、延迟/吞吐的交换，优先算法、数据结构和分配。
- 用于一划：保留现有阻塞等待消息的鼠标钩子/UIA worker，先处理图标 WebView 常驻与重复 DOM 更新。本轮不改 IPC 事件数量和 Rust 运行时配置。
- 不采用：文中的倍数只是经验排序，不能当成本项目收益。`perf`、`heaptrack`、Valgrind 也不直接搬到 Windows。单次短文本翻译不引入 Rayon、对象池或 SIMD，少几个分配不值得养一套机器。

### 3. `javascript-pro`：覆盖 TypeScript 编译后的浏览器运行时

- 作者仓库：[Jeffallan/claude-skills](https://github.com/Jeffallan/claude-skills)。[SKILL.md 原文](https://github.com/Jeffallan/claude-skills/blob/main/skills/javascript-pro/SKILL.md)；另读了其 [browser-apis](https://github.com/Jeffallan/claude-skills/blob/main/skills/javascript-pro/references/browser-apis.md) 与 [async-patterns](https://github.com/Jeffallan/claude-skills/blob/main/skills/javascript-pro/references/async-patterns.md)。
- 实际内容：面向原生 JS、异步流程、浏览器内存诊断，禁止阻塞浏览器主线程；配套参考涵盖取消请求和受控并发。
- 用于一划：首块立即显示，后续字块在前端按 32ms 合并更新 DOM。停止同一次翻译和正常完成都刷出已收到的尾块；过期请求由请求 ID/窗口代际检查忽略，沿用后端取消机制。
- 不采用：不为短文本引入 Worker/PWA/Service Worker，也不迁移到 Jest 或追求该 Skill 的统一覆盖率。其 `Promise.race` 超时示例只结束等待，不会自动取消底层请求；本项目继续由 Rust 的取消机制负责请求生命周期。

这里选 JS 运行时 Skill 有明确理由：TypeScript 类型标注会被擦除，不改变运行行为。只优化 `tsc` 检查、复杂泛型或编辑器速度，解决不了用户安装包的常驻占用。[TypeScript 官方说明](https://www.typescriptlang.org/docs/handbook/2/basic-types.html#erased-types)

## 本轮实际采用的方案

本轮选择改生命周期与更新频率，保留已认可的外观和现有通信契约。

| 优先级 | 选择 | 收益与付出的代价 |
|---|---|---|
| 1 | 图标继续使用现有 WebView，按需创建；ready 后显示；隐藏 30 秒且仍未使用才销毁 | 目标是减少长时间闲置的 WebView 常驻；保温期内仍占内存，换取连续划词响应。超时重开存在冷启动成本；旧回收任务不能销毁新一代或重新显示的窗口 |
| 2 | 只在前端做 32ms 流式合批，首块立即；停止与完成都刷尾块，过期请求忽略 | 减少 DOM 写入与布局频率；后续文字多等一个 32ms 合批窗口，实际调度可能更迟。IPC 事件仍逐条接收，事件数不减少 |
| 3 | 沿用后端取消和请求 ID/窗口代际隔离 | 保留快速切换不串结果的语义；本轮不宣称由此新增网络或 IPC 节省 |
| 4 | 沿用主窗、译文窗的关闭销毁，以及消息驱动的划词捕获 | 保留既有资源释放路径与选区语义，不把已有能力算成本轮新增收益 |
| 5 | 本轮不改 allocator、panic 策略、Tokio worker 数，不把 `opt-level="z"` 当省内存按钮 | 保持维护成本和异常语义；把精力用在 WebView 与重复工作上 |

**原生轻量划词按钮仅列为未来可选项，本轮未实现。** 它可能进一步省去图标 WebView，但要重做 28px 圆图标和 hover，并承担外观回归。用户本轮不允许 CUA 测试，因此不冒险重写这块已满意的交互。

### 官方资料给出的边界

- **WebView 生命周期优先。** 微软确认 WebView2 的多进程架构带来内存和启动开销，建议避免冗余实例，并尽量复用仍会频繁使用的 WebView。轻量 Win32 UI 可绕开 WebView 冷启动。关闭一个窗口后的进程退出与内存回收仍要实测。[WebView2 性能最佳实践](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/performance)
- **隐藏、低内存、挂起各有不同语义。** `TrySuspend` 要求 WebView 不可见，属于尽力而为；会暂停脚本计时器和动画。`MemoryUsageTargetLevel.Low` 仍运行脚本，可能把内存换到磁盘并影响恢复性能。两套机制不应混用。本轮采用隐藏后延迟销毁，不增加原生挂起状态机。[TrySuspend](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2_3#trysuspend)、[MemoryUsageTargetLevel](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2_19#put_memoryusagetargetlevel)
- **高频流与控制消息分开。** Tauri event 适合小消息，channel 提供快速有序流；`listen` 返回的解除函数应在作用域结束时调用。一划本轮保留现有 event 契约，仅合批前端更新；channel 留给确有 IPC 瓶颈证据时再迁移。[Tauri 官方通信文档](https://v2.tauri.app/develop/calling-frontend/)
- **体积与运行占用分开。** `opt-level="z"` 优化二进制体积并关闭循环向量化；`panic="abort"` 会在 panic 时终止进程；LTO 增加链接成本。这些参数没有直接承诺减少 WebView 内存。[Cargo Profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)

## 已有基线与当前证据

记录：[performance-baseline.json](performance-baseline.json)，采样工具：[measure-resources.ps1](../scripts/measure-resources.ps1)。这次测量来自已经运行的旧实例，采样实际持续 **10.127941 秒**，覆盖一划和其进程树中的 WebView2；窗口可见性、内容与交互状态均未控制。

| 指标 | 旧实例观测 |
|---|---:|
| 进程数 | 8（其中 WebView2 为 7） |
| Working Set 合计 | 249.031 MiB |
| Private Bytes 合计 | 195.965 MiB |
| 采样期观测 CPU 时间增量 | 0.0 秒 |
| 按 16 个逻辑处理器归一化的平均 CPU | 0.0% |

Working Set 合计会重复计算共享页；Private Bytes 是私有提交内存，不能叫作实际驻留 RAM。CPU 为 0 仅代表此次采样未观测到增量，不能推出所有使用状态都零占用。**新版尚未运行测量，没有优化后数值，也不报告节省比例。**

实现后的代码级验证：28 项 Rust 测试通过，覆盖图标就绪、冷加载中取消、旧回收任务失效与加载失败回收；8 项 TypeScript 测试通过，覆盖流式首块、合批、尾块与关闭后的迟到事件。在每秒 100 个字块、共 1000 个字块的合成调度测试中，更新回调由逐块 1000 次变为 251 次，最终文字一致。这个计数说明合批行为，不代表实机 CPU 降幅。

## 后续运行对比只看相关指标

使用 release 构建、同一 WebView2 版本和同一条翻译输入；记录一划进程及属于它的 WebView2 子进程，别把整台电脑其他应用的 WebView2 算进账本。

1. **后台 60 秒**：累计 CPU 时间增量、Private Bytes、Working Set、进程数；计算平均 CPU 时明确是否除以逻辑处理器数。Private Bytes 与 Working Set 分开报告，避免把共享页重复相加称为真实物理占用。
2. **划词与重开**：选区触发至按钮可见、点击至译文窗可见、首字和完成时间分别记录；网络等待单列，不能把服务商波动算成本地性能收益。
3. **快速换选区并关闭/重开**：旧请求确实结束、最终译文无丢字/串窗，反复开关后内存和监听数量不持续上涨。

运行对比须使用可比的窗口状态，不能把这次状态未控制的基线直接当成严格 A/B。图标外观、hover、ready 显示与 30 秒回收后的真实冷启动体验，由用户在 Windows 上自行检查；本轮不使用 CUA 操作界面。
