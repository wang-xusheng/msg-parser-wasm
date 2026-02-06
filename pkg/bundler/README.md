# msg-parser-wasm

[github](https://github.com/wang-xusheng/msg-parser-wasm)

---

A Rust-based parser for Microsoft Outlook `.msg` files, compiled to WebAssembly (WASM) for high-performance use in the browser.

## Features

- Parse `.msg` files directly in the browser.
- Extract email metadata: Subject, Sender, Recipients (To/CC), Sent Time.
- Extract email body: Both Plain Text and HTML versions.
- Extract attachments: Filenames, Content-Types, Content-IDs (for inline images), and raw data.
- Support for multiple encodings (UTF-16, UTF-8, GBK).


## Usage

import msg-parser-wasm  
npm: `npm i msg-parser-wasm`  
pnpm: `pnpm add msg-parser-wasm`  

```javascript
import { parse_msg_file } from 'msg-parser-wasm'; // Or './pkg/bundler/msg_parser_wasm.js'

// Most bundlers allow direct usage if configured correctly
const response = await fetch(url)
const arrayBuffer = await response.arrayBuffer()
const uint8Array = new Uint8Array(arrayBuffer)
const result = parse_msg_file(uint8Array)
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