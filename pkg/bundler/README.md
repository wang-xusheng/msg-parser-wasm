# msg-parser-wasm

[简体中文](./README.zh-CN.md)

---

A Rust-based parser for Microsoft Outlook `.msg` files, compiled to WebAssembly (WASM) for high-performance use in the browser.

## Features

- Parse `.msg` files directly in the browser.
- Extract email metadata: Subject, Sender, Recipients (To/CC), Sent Time.
- Extract email body: Both Plain Text and HTML versions.
- Extract attachments: Filenames, Content-Types, Content-IDs (for inline images), and raw data.
- Support for multiple encodings (UTF-16, UTF-8, GBK).

## Prerequisites

To build this project, you need:

- [Rust](https://www.rust-lang.org/tools/install)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)

## Build

You can build for different environments using the `--out-dir` flag:

### Web (Direct Browser Usage)
For usage directly in a browser via `<script type="module">`.
```bash
wasm-pack build --target web --out-dir pkg/web
```

### Bundler (Vite, Webpack, Rollup)
For usage with modern build tools. This is usually the default for npm packages.
```bash
wasm-pack build --target bundler --out-dir pkg/bundler
```

## Usage

### 1. Web Target (No Bundler)
When using the `web` target, you must manually initialize the WASM module.

```javascript
import init, { parse_msg_file } from './pkg/web/msg_parser_wasm.js';

async function run() {
    // Initialize the WASM module
    await init();

    const fileInput = document.getElementById('file-input');
    fileInput.addEventListener('change', async (e) => {
        const file = e.target.files[0];
        const uint8Array = new Uint8Array(await file.arrayBuffer());

        try {
            const emailData = parse_msg_file(uint8Array);
            console.log("Subject:", emailData.subject);
        } catch (err) {
            console.error("Parsing error:", err);
        }
    });
}
run();
```

### 2. Bundler Target (Vite/Webpack)
When using a bundler, the initialization is often handled automatically or through a simpler import.

```javascript
import { parse_msg_file } from 'msg-parser-wasm'; // Or './pkg/bundler/msg_parser_wasm.js'

// Most bundlers allow direct usage if configured correctly
const emailData = parse_msg_file(uint8Array);
```

## Data Structure

The `parse_msg_file` function returns a JavaScript object with the following structure:

```typescript
interface MsgEmail {
    subject: string | null;
    sender_name: string | null;
    sender_email: string | null;
    recipients: string[];
    cc_recipients: string[];
    sent_time: string | null;
    body_text: string | null;
    body_html: string | null;
    attachments: Attachment[];
}

interface Attachment {
    filename: string;
    content_type: string | null;
    content_id: string | null;
    data: Uint8Array;
}
```

## Optimization

The release build is optimized for size using:
- Link Time Optimization (LTO)
- `opt-level = 'z'` (optimize for size)

## License

MIT