# 划词翻译

在 Windows 上选中一段文字，点击旁边的翻译图标，即可查看译文。遇到无法读取选区的应用，可以打开主窗口，粘贴文字进行手动翻译。

## 开始使用

1. 启动应用，打开侧边栏「模型接口」，从列表选择服务：阿里云百炼、硅基流动、DeepSeek、智谱 AI、Kimi、火山方舟、OpenRouter、OpenAI、Google Gemini、Anthropic Claude，或免密的 MyMemory。
2. 在服务详情页切换当前服务；使用云端模型时，在服务商控制台创建 API Key 并在应用中保存。各服务的 Key 分开存入当前 Windows 用户的凭据管理器。
3. 点击「获取模型列表」，输入模型名筛选，选择后点击「保存模型」；也可直接填写模型 ID。阿里云北京地域的列表接口还需要百炼工作空间 ID，翻译本身不需要这个 ID。智谱请手动填写模型 ID。
4. 在浏览器或文档中选中文字，点击出现的图标。译文会显示在浮窗中，也可以复制。
5. 在「偏好设置」中选择原文与目标语言、暂停划词或排除程序；退出程序请使用系统托盘菜单。

使用支持流式输出的云端模型时，译文会随着服务商返回的内容逐段显示。MyMemory 接口返回完整结果后再显示；不同服务和模型的响应速度取决于其接口与网络。

硅基流动默认使用 `tencent/Hunyuan-MT-7B`。截至 2026 年 11 月 7 日，其[官方价格页](https://siliconflow.cn/pricing)将输入和输出标为免费；免费模型有速率限制，政策可能调整。申请和使用前请再次核对价格及账号条件。

阿里云百炼默认使用北京地域的 `deepseek-v4.1-flash`，需使用该地域普通模型 API Key。请在[百炼免费额度页面](https://help.aliyun.com/zh/model-studio/new-free-quota)核对账号的模型额度和期限，并启用该模型的「免费额度用完即停」；额度耗尽或到期后可能转为按量收费。Coding Plan Key 不适用于本应用。

MyMemory 匿名额度为每天 5,000 字符、每次最多 500 UTF-8 字节和一个段落，额度按网络出口计算；用完后需等待恢复或手动切换服务。它要求指定源语言。服务商说明提交的文本片段可能长期保存并与合作方共享，请勿用它翻译敏感内容，详见[隐私条款](https://mymemory.translated.net/terms-and-conditions)。

「免费接口发现」提供硅基流动、阿里云和 OpenRouter 的官方入口。OpenRouter 的「刷新免费模型」读取公开模型目录，列出符合当前免费条件的文本模型。云端服务返回的模型列表可能包含收费模型或非文本模型，获取列表本身不会翻译；使用前请在对应平台核对模型权限、价格和免费额度。

## 开发运行

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

构建安装包：

```powershell
npm run tauri build
```

## 隐私与兼容性

- 程序在本机读取选中文字以判断是否显示图标；点击图标翻译或提交手动翻译后，原文才发送至所选翻译服务。
- 自动取词通过 Windows UI Automation 完成，不模拟复制，也不改动剪贴板。点击「复制译文」会主动写入剪贴板。
- 默认排除常见终端、密码管理器和密码控件；可在设置中按程序名或完整路径添加排除项。UI Automation 依赖目标程序正确提供控件信息，无法保证识别所有敏感内容。
- 拖选和双击选词可触发图标。自绘控件、某些 PDF 阅读器、图片，以及以管理员身份运行的程序可能无法取词；请使用手动翻译。
- API Key 不要提交到仓库，也不要粘贴进问题反馈或截图。

## 当前验证状态

Rust 桌面应用和前端已完成编译；在 Codex 内置浏览器中双击英文后出现过划词图标。新版桌面程序已通过硅基流动获取真实模型列表，并验证手动翻译的实时增量输出及停止操作；划词图标到流式译文的完整链路、鼠标拖选及其他应用兼容性仍需验证。MyMemory 在此前测试的网络出口返回额度耗尽。其他云服务的真实账号调用尚未逐一验证。

## 第三方代码

Windows 划词捕获实现参考并改编自 MIT 许可的 Magpie（拾趣）。版权声明、完整许可和来源见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。项目本身的许可证尚未指定；发布前应另行确定。
