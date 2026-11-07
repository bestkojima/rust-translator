# 第三方代码与许可

## Magpie（拾趣）

Windows 划词捕获实现改编自 Magpie 的 Windows 捕获模块。

- 来源：<https://github.com/demo007x/Magpie/blob/main/src-tauri/src/capture/windows.rs>
- 上游许可：<https://github.com/demo007x/Magpie/blob/main/LICENSE>
- 本项目对应文件：`src-tauri/src/capture.rs`

以下为上游 MIT 许可证全文：

```text
MIT License

Copyright (c) 2026 Magpie (拾趣)

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## 服务商品牌标识

- OpenRouter 的原色小图标来自其[官方品牌素材](https://openrouter.ai/brand)，文件为 `src/assets/brands/openrouter-glyph.svg`。请按官方要求使用原色，不拉伸或改造。
- Kimi 的原色 K 图标来自其[官方品牌指南](https://moonshotai.github.io/Branding-Guide/)，文件为 `src/assets/brands/kimi-k.svg`。

上述品牌标识的权利归各自所有者，仅用于识别应用内支持的服务；其他服务使用文字头像。

## 其他依赖

Rust 和 JavaScript 依赖及其版本分别由 `src-tauri/Cargo.lock` 和 `package-lock.json` 记录；各依赖使用各自的许可证。此文件列示直接改编的第三方源码，发布分发包前应一并处理依赖的许可要求。
