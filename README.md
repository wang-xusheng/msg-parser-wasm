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

To compile the Rust code to WebAssembly for web use, run the following command in the project root:

```bash
wasm-pack build --target web
```

This will generate a `pkg/` directory containing the compiled WASM binary and the JavaScript glue code.

## Usage

### 1. Include in your Project

After building, you can use the generated package in your web application.

```javascript
import init, { parse_msg_file } from './pkg/msg_parser_wasm.js';

async function run() {
    // Initialize the WASM module
    await init();

    const fileInput = document.getElementById('file-input');
    
    fileInput.addEventListener('change', async (e) => {
        const file = e.target.files[0];
        if (!file) return;

        const arrayBuffer = await file.arrayBuffer();
        const uint8Array = new Uint8Array(arrayBuffer);

        try {
            // Parse the MSG file
            const emailData = parse_msg_file(uint8Array);
            
            console.log("Subject:", emailData.subject);
            console.log("Sender:", emailData.sender_name);
            console.log("Body:", emailData.body_text);
            
            // Handle attachments
            emailData.attachments.forEach(att => {
                console.log(`Attachment: ${att.filename} (${att.data.length} bytes)`);
                // att.data is a Uint8Array
            });
        } catch (err) {
            console.error("Parsing error:", err);
        }
    });
}

run();
```

### 2. Data Structure

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
