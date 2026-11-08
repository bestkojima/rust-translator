<img src="src-tauri/icons/icon.png" alt="一划应用图标" width="112" height="112">

# 一划 · Wordglance

**选中文字，点一下，译文就在眼前。**

一划是一款 Windows 跨应用划词翻译工具。阅读网页、文档时，选中内容并点击划词图标，即可在浮窗里查看译文；也可以打开主窗口，粘贴文字进行翻译。

[![Windows](https://img.shields.io/badge/Windows-10%20%2F%2011-000000?logo=windows11&logoColor=white)](https://github.com/bestkojima/rust-translator/releases)
[![Release](https://img.shields.io/github/v/release/bestkojima/rust-translator?include_prereleases&label=release&color=000000)](https://github.com/bestkojima/rust-translator/releases)
[![Rust](https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2-000000?logo=tauri&logoColor=white)](https://v2.tauri.app/)

[下载安装包](https://github.com/bestkojima/rust-translator/releases) · [服务商与费用](#服务商与费用) · [从源码运行](#从源码运行) · [Gitee 镜像](https://gitee.com/ddddr_wew/rust-translator)

> 当前为测试版。安装包尚未签名；自动关闭、图钉、语言菜单和按需释放窗口仍需 Windows 实机联测。

## 能做什么

- **划词即译**：在支持的应用中选中文字，点击出现的图标查看译文；也支持手动粘贴翻译。
- **译文浮窗**：切换原文和目标语言、复制译文；图钉可固定浮窗，默认切换到其他窗口后关闭。
- **按需接入模型**：支持多个云端服务商，提供模型列表获取与手动输入模型 ID；支持流式输出的模型逐段显示译文。
- **寻找免费接口**：内置免费接口发现入口；免费额度和服务价格以各平台当前规则为准。
- **开机自动启动**：在偏好设置中开启，登录后静默运行于系统托盘；手动启动会打开已有窗口。
- **本机保存密钥**：各服务商的 API Key 分别存放在当前 Windows 用户的凭据管理器。

## 安装与快速开始

从 [Releases](https://github.com/bestkojima/rust-translator/releases) 下载 Windows x64 安装包，按提示安装并启动。需要 Windows 10/11 和 Microsoft Edge WebView2 Runtime。若使用独立程序，将 `wordglance.exe` 放到固定位置后运行（压缩包需先解压）。

1. 启动应用，打开侧边栏「模型接口」，从列表选择服务：阿里云百炼、硅基流动、DeepSeek、智谱 AI、Kimi、火山方舟、OpenRouter、OpenAI、Google Gemini、Anthropic Claude，或免密的 MyMemory。
2. 在服务详情页切换当前服务；使用云端模型时，在服务商控制台创建 API Key 并在应用中保存。各服务的 Key 分开存入当前 Windows 用户的凭据管理器。
3. 点击「获取模型列表」，输入模型名筛选，选择后点击「保存模型」；也可直接填写模型 ID。阿里云北京地域的列表接口还需要百炼工作空间 ID，翻译本身不需要这个 ID。智谱请手动填写模型 ID。
4. 在浏览器或文档中选中文字，点击出现的图标。译文会显示在浮窗中，也可以复制；在浮窗中切换语言会重新翻译当前选区，不改变默认设置。
5. 译文浮窗默认在切换到其他窗口时关闭。点击图钉可固定，再次点击取消；关闭浮窗后下次打开默认不固定。浮窗内的语言选择只影响当前翻译。
6. 在「偏好设置」中选择原文与目标语言、暂停划词、排除程序或开启开机自动启动；启动开关立即生效，其他设置请点击「保存设置」。退出程序请使用系统托盘菜单。

开机自动启动只对当前 Windows 用户生效，也可右键系统托盘中的「一划」图标快速切换。若移动独立程序，打开新位置的程序后，关闭再开启启动项以更新路径；如果 Windows 启动设置中禁用了「一划」，请先在那里重新允许。

使用支持流式输出的云端模型时，译文会随着服务商返回的内容逐段显示。MyMemory 接口返回完整结果后再显示；不同服务和模型的响应速度取决于其接口与网络。

## 服务商与费用

硅基流动默认使用 `tencent/Hunyuan-MT-7B`。截至 2026 年 11 月 7 日，其[官方价格页](https://siliconflow.cn/pricing)将输入和输出标为免费；免费模型有速率限制，政策可能调整。申请和使用前请再次核对价格及账号条件。

阿里云百炼默认使用北京地域的 `deepseek-v4.1-flash`，需使用该地域普通模型 API Key。请在[百炼免费额度页面](https://help.aliyun.com/zh/model-studio/new-free-quota)核对账号的模型额度和期限，并启用该模型的「免费额度用完即停」；额度耗尽或到期后可能转为按量收费。Coding Plan Key 不适用于本应用。

MyMemory 匿名额度为每天 5,000 字符、每次最多 500 UTF-8 字节和一个段落，额度按网络出口计算；用完后需等待恢复或手动切换服务。它要求指定源语言。服务商说明提交的文本片段可能长期保存并与合作方共享，请勿用它翻译敏感内容，详见[隐私条款](https://mymemory.translated.net/terms-and-conditions)。

「免费接口发现」提供硅基流动、阿里云和 OpenRouter 的官方入口。OpenRouter 的「刷新免费模型」读取公开模型目录，列出符合当前免费条件的文本模型。云端服务返回的模型列表可能包含收费模型或非文本模型，获取列表本身不会翻译；使用前请在对应平台核对模型权限、价格和免费额度。

## 从源码运行

系统与工具：

- Windows 10 或 Windows 11，x64。
- Node.js 和 npm。
- Rust stable（MSVC 工具链）、Microsoft C++ Build Tools / Visual Studio 的「使用 C++ 的桌面开发」工作负载和 Windows SDK。
- Microsoft Edge WebView2 Runtime。

安装步骤可参考 [Tauri Windows 开发环境文档](https://v2.tauri.app/start/prerequisites/#windows)。在项目根目录执行：

```powershell
npm install
npm run tauri dev
```

构建 Windows 安装包：

```powershell
npm run tauri build
```

默认构建产物位于 `src-tauri/target/release/`，NSIS 安装包位于其 `bundle/nsis/` 子目录。前端检查使用 `npm run build`；Rust 测试使用 `cargo test --manifest-path src-tauri/Cargo.toml`。使用支持直接运行 TypeScript 的 Node.js 版本时，前端状态与流式合批测试通过 `npm test` 运行。

## 后台占用与响应速度

划词图标按需创建，隐藏后保留 30 秒，连续使用时复用；闲置后销毁。开机静默启动不预先创建图标网页，主窗口和译文浮窗关闭后也会释放。久置后第一次划词需要重新加载图标，换取更低的长期后台占用；图标尺寸、底色和 hover 保持原样。

流式译文首块立即显示，后续增量按 32 毫秒合批更新界面，停止和完成时立即提交尾块。合批减少 DOM 更新，后续文字增加少量本地显示延迟；接口、模型、原文和最终译文不变。

测量正在运行的一划及其 WebView2 子进程：

```powershell
.\scripts\measure-resources.ps1 -SampleSeconds 10
```

对比时使用相同窗口状态：从托盘退出旧版、启动新版，关闭主窗口和译文窗，等待图标隐藏超过 30 秒后采样，再检查首次划词与连续划词的响应。测量脚本只读取进程统计，不操作界面。Skill 筛选、优化取舍和优化前基线见 [性能记录](docs/performance-skills.md)；新版占用尚待实机复测。

## 项目结构

```text
rust-translator/
├── src/
│   ├── main.ts                # 前端界面、翻译流程与窗口交互
│   ├── language-menu.ts       # 浮窗语言菜单状态与键盘操作
│   ├── stream-batcher.ts      # 首块即时、后续流式合批
│   ├── style.css              # 页面与浮窗样式
│   └── assets/brands/         # 部分服务商图标
├── src-tauri/
│   ├── src/
│   │   ├── main.rs            # Tauri 命令、窗口、配置和服务商接入
│   │   ├── autostart.rs       # 当前用户 Windows 启动项
│   │   ├── capture.rs         # Windows 跨应用划词捕获
│   │   ├── overlay_lifecycle.rs # 图标就绪、隐藏与闲置回收状态
│   │   └── streaming.rs       # 流式响应处理
│   ├── capabilities/          # Tauri 权限配置
│   ├── windows/               # Windows 安装与卸载钩子
│   ├── icons/                 # 应用图标
│   ├── providers.json         # 内置服务商接口和默认模型（不存密钥）
│   ├── Cargo.toml             # Rust 依赖
│   └── tauri.conf.json        # 窗口和打包配置
├── docs/                      # Skill 调研、取舍与占用基线
├── scripts/                   # 只读进程资源采样
├── tests/                     # 语言菜单与流式合批测试
├── index.html                 # 前端入口
├── package.json               # 前端依赖与命令
└── THIRD_PARTY_NOTICES.md     # 第三方代码与许可声明
```

`dist/`、`node_modules/` 和 `src-tauri/target/` 是构建或依赖目录，不纳入版本管理。

需要 API Key 的模型服务商，其名称、翻译接口、模型列表接口和默认模型统一维护在 `src-tauri/providers.json`；该文件只放公开配置，不放 API Key。修改后重新构建应用才能生效。百炼模型列表地址需要工作空间 ID，仍由程序按固定的官方地域地址生成；智谱目前使用手动模型 ID；免密的 MyMemory 接口单独处理。新增服务商还需要实现相应的前端和协议处理，不能只添加一条 JSON 记录。

## 隐私与兼容性

- 程序在本机读取选中文字以判断是否显示图标；点击图标翻译或提交手动翻译后，原文才发送至所选翻译服务。
- 自动取词通过 Windows UI Automation 完成，不模拟复制，也不改动剪贴板。点击「复制译文」会主动写入剪贴板。
- 默认排除常见终端、密码管理器和密码控件；可在设置中按程序名或完整路径添加排除项。UI Automation 依赖目标程序正确提供控件信息，无法保证识别所有敏感内容。
- 拖选和双击选词可触发图标。自绘控件、某些 PDF 阅读器、图片，以及以管理员身份运行的程序可能无法取词；请使用手动翻译。
- API Key 不要提交到仓库，也不要粘贴进问题反馈或截图。

## 当前验证状态

Windows 跨应用选词、图标和译文浮窗已有用户实机使用反馈。当前代码中的自动关闭、图钉、语言菜单以及按需释放窗口已通过构建和自动测试，仍待 Windows 实机联测；不同服务商、模型及目标应用的兼容性可能有所不同。

## 第三方代码

Windows 划词捕获实现参考并改编自 MIT 许可的 Magpie（拾趣）。版权声明、完整许可和来源见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。项目本身的许可证尚未指定；发布前应另行确定。
