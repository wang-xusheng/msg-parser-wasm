# msg-parser-wasm

[English](./README.md)

---

基于 Rust 开发的 Microsoft Outlook `.msg` 文件解析器，已编译为 WebAssembly (WASM)，适用于浏览器端的高性能文件处理。

## 功能特性

- 直接在浏览器中解析 `.msg` 文件。
- 提取邮件元数据：主题、发件人、收件人 (To/CC)、发送时间。
- 提取邮件正文：支持纯文本 (Plain Text) 和 HTML 格式。
- 提取附件：包括文件名、Content-Type、Content-ID（用于匹配内嵌图片）以及原始二进制数据。
- 支持多种编码：UTF-16, UTF-8, GBK 等。

## 环境准备

在开始构建之前，请确保已安装：

- [Rust](https://www.rust-lang.org/tools/install)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)

## 打包构建

你可以通过 `--out-dir` 参数将不同目标的构建产物输出到指定目录：

### Web 模式 (浏览器直接使用)
适用于直接通过 `<script type="module">` 引入。
```bash
wasm-pack build --target web --out-dir pkg/web
```

### Bundler 模式 (Vite, Webpack, Rollup)
适用于现代前端构建工具。
```bash
wasm-pack build --target bundler --out-dir pkg/bundler
```

## 使用示例

### 1. Web 模式 (原生 JS)
在 Web 模式下，你需要手动调用 `init` 函数。

```javascript
import init, { parse_msg_file } from './pkg/web/msg_parser_wasm.js';

async function run() {
    // 初始化 WASM 模块
    await init();

    const fileInput = document.getElementById('file-input');
    fileInput.addEventListener('change', async (e) => {
        const file = e.target.files[0];
        const uint8Array = new Uint8Array(await file.arrayBuffer());

        try {
            // 解析 MSG 文件
            const emailData = parse_msg_file(uint8Array);
            console.log("主题:", emailData.subject);
        } catch (err) {
            console.error("解析错误:", err);
        }
    });
}
run();
```

### 2. Bundler 模式 (Vite/Webpack)
在构建工具环境下，通常不需要手动调用 `init`。

```javascript
import { parse_msg_file } from 'msg-parser-wasm'; // 或路径引用

// 直接使用解析函数
const emailData = parse_msg_file(uint8Array);
```

## 数据结构

`parse_msg_file` 函数返回的 JavaScript 对象结构如下：

```typescript
interface MsgEmail {
    subject: string | null;      // 主题
    sender_name: string | null;  // 发件人姓名
    sender_email: string | null; // 发件人邮箱
    recipients: string[];        // 收件人列表
    cc_recipients: string[];     // 抄送人列表
    sent_time: string | null;    // 发送时间
    body_text: string | null;    // 文本正文
    body_html: string | null;    // HTML 正文
    attachments: Attachment[];   // 附件列表
}

interface Attachment {
    filename: string;            // 文件名
    content_type: string | null; // 内容类型
    content_id: string | null;   // Content-ID (用于 HTML 内嵌资源)
    data: Uint8Array;            // 原始二进制数据
}
```

## 优化说明

Release 版本已针对 WASM 体积进行了优化：
- 开启 LTO (Link Time Optimization)
- 使用 `opt-level = 'z'` (极致体积优化)

## 开源协议

MIT