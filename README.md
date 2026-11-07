# 划词翻译

在 Windows 上选中一段文字，点击旁边的翻译图标，即可查看译文。遇到无法读取选区的应用，可以打开主窗口，粘贴文字进行手动翻译。

## 开始使用

1. 前往[硅基流动控制台](https://cloud.siliconflow.cn/)注册、完成平台要求的认证，并创建自己的 API Key。
2. 启动应用，在「开始翻译」页面粘贴 API Key，点击「保存密钥」。密钥保存在当前 Windows 用户的凭据管理器中。
3. 在浏览器或文档中选中文字，点击出现的图标。译文会显示在浮窗中，也可以复制。
4. 需要暂停或排除某个程序时，打开「偏好设置」；退出程序请使用系统托盘菜单。

默认翻译模型为硅基流动的 `tencent/Hunyuan-MT-7B`。截至 2026 年 11 月 7 日，其[官方价格页](https://siliconflow.cn/pricing)将输入和输出标为免费；免费模型有速率限制，政策可能调整。申请和使用前请再次核对价格及账号条件。

「免费接口发现」提供硅基流动、阿里云和 OpenRouter 的官方入口。OpenRouter 的「刷新免费模型」读取公开模型目录，列出符合当前免费条件的文本模型；列表用于发现，翻译功能目前使用硅基流动。各服务实际价格、账号资格和额度以服务商官方页面为准，阿里云免费额度用完后可能产生费用。

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

- 程序在本机读取选中文字以判断是否显示图标；点击图标翻译或提交手动翻译后，原文才发送至硅基流动。
- 自动取词通过 Windows UI Automation 完成，不模拟复制，也不改动剪贴板。点击「复制译文」会主动写入剪贴板。
- 默认排除常见终端、密码管理器和密码控件；可在设置中按程序名或完整路径添加排除项。UI Automation 依赖目标程序正确提供控件信息，无法保证识别所有敏感内容。
- 拖选和双击选词可触发图标。自绘控件、某些 PDF 阅读器、图片，以及以管理员身份运行的程序可能无法取词；请使用手动翻译。
- API Key 不要提交到仓库，也不要粘贴进问题反馈或截图。

## 当前验证状态

Rust 桌面应用和前端已完成编译，并已查看主界面与免费接口页面。完整划词交互、各类应用的兼容性，以及使用真实硅基流动账号的翻译调用，仍需在目标 Windows 环境中验证。

## 第三方代码

Windows 划词捕获实现参考并改编自 MIT 许可的 Magpie（拾趣）。版权声明、完整许可和来源见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。项目本身的许可证尚未指定；发布前应另行确定。
