use cfb::CompoundFile;
use encoding_rs;
use serde::Serialize;
use std::io::{Cursor, Read};
use std::path::PathBuf;
use wasm_bindgen::prelude::*;

// MAPI Property Tags (first 4 characters of the stream name after __substg1.0_)
const TAG_SUBJECT: &str = "0037";
const TAG_SENDER_NAME: &str = "0C1A";
const TAG_SENDER_EMAIL_1: &str = "0C1F";
const TAG_SENDER_EMAIL_2: &str = "5D01";
const TAG_SENDER_EMAIL_3: &str = "0065";
const TAG_DISPLAY_TO: &str = "0E04";
const TAG_RECIPIENT_EMAIL_1: &str = "0E03";
const TAG_RECIPIENT_EMAIL_2: &str = "0076";
const TAG_DISPLAY_CC: &str = "0E02";
const TAG_TRANSPORT_HEADERS: &str = "007D";
const TAG_CLIENT_SUBMIT_TIME: &str = "0039";
const TAG_MESSAGE_DELIVERY_TIME: &str = "0E06";
const TAG_BODY: &str = "1000";
const TAG_BODY_RTF: &str = "1009";
const TAG_BODY_HTML: &str = "1013";

// Attachment Tags
const TAG_ATTACH_FILENAME_LONG: &str = "3707";
const TAG_ATTACH_FILENAME_SHORT: &str = "3704";
const TAG_ATTACH_DISPLAY_NAME: &str = "3001";
const TAG_ATTACH_EXTENSION: &str = "3703";
const TAG_ATTACH_MIME_TAG: &str = "370E";
const TAG_ATTACH_CONTENT_ID: &str = "3712";
const TAG_ATTACH_DATA_BIN: &str = "3701";

/// 邮件结构体
#[derive(Debug, Default, Serialize)]
pub struct MsgEmail {
    pub subject: Option<String>,
    pub sender_name: Option<String>,
    pub sender_email: Option<String>,
    pub recipients: Vec<String>,
    pub cc_recipients: Vec<String>,
    pub sent_time: Option<String>,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub body_rtf: Option<String>,
    pub attachments: Vec<Attachment>,
}

/// 附件结构体
#[derive(Debug, Serialize, Default)]
pub struct Attachment {
    pub filename: String,
    pub content_type: Option<String>,
    /// Content-ID，对应 HTML 中 src="cid:xxx" 的 xxx，用于定位正文引用的内嵌附件
    pub content_id: Option<String>,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

/// WASM 导出接口
/// 解析 MSG 文件并返回邮件结构体
#[wasm_bindgen]
pub fn parse_msg_file(file_data: &[u8]) -> Result<JsValue, JsValue> {
    let email = parse_msg_to_struct(file_data).map_err(|e| JsValue::from_str(&e.to_string()))?;

    serde_wasm_bindgen::to_value(&email)
        .map_err(|e| JsValue::from_str(&format!("序列化失败: {}", e)))
}

/// 内部解析函数，方便在 Rust 单元测试中调用
pub fn parse_msg_to_struct(file_data: &[u8]) -> Result<MsgEmail, Box<dyn std::error::Error>> {
    let cursor = Cursor::new(file_data);

    let mut comp = CompoundFile::open(cursor)?;

    let mut email = MsgEmail::default();

    let mut streams: Vec<(String, PathBuf)> = Vec::new();
    let mut attachment_dirs: Vec<(String, PathBuf)> = Vec::new();

    comp.walk().for_each(|entry| {
        let name = entry.name().to_string();
        let path = entry.path().to_path_buf();

        if name.starts_with("__substg1.0_") {
            streams.push((name, path));
        } else if name.starts_with("__attach_version1.0_") {
            attachment_dirs.push((name, path));
        }
    });

    // 解析顶级属性
    for (name, path) in &streams {
        if let Ok(mut stream) = comp.open_stream(path) {
            let mut data = Vec::new();
            if stream.read_to_end(&mut data).is_ok() && !data.is_empty() {
                parse_property(&mut email, name, &data);
            }
        }
    }

    // 解析附件
    for (att_dir, _) in &attachment_dirs {
        if let Ok(attachment) = parse_attachment_internal(&mut comp, att_dir) {
            email.attachments.push(attachment);
        }
    }

    Ok(email)
}

fn parse_property(email: &mut MsgEmail, prop_name: &str, data: &[u8]) {
    let tag = if prop_name.len() >= 20 {
        &prop_name[12..16]
    } else {
        return;
    };

    match tag {
        TAG_SUBJECT => {
            if let Some((text, _)) = decode_with_encoding(data) {
                email.subject = Some(text);
            }
        }
        TAG_SENDER_NAME => {
            if let Some((text, _)) = decode_with_encoding(data) {
                email.sender_name = Some(text);
            }
        }
        TAG_SENDER_EMAIL_1 | TAG_SENDER_EMAIL_2 | TAG_SENDER_EMAIL_3 => {
            if let Some((text, _)) = decode_with_encoding(data) {
                email.sender_email = Some(text);
            }
        }
        TAG_DISPLAY_TO => {
            if let Some((text, _)) = decode_with_encoding(data) {
                for recipient in text.split(';') {
                    let r = recipient.trim().to_string();
                    if !r.is_empty() {
                        email.recipients.push(r);
                    }
                }
            }
        }
        TAG_RECIPIENT_EMAIL_1 | TAG_RECIPIENT_EMAIL_2 => {
            if let Some((text, _)) = decode_with_encoding(data) {
                for recipient in text.split(';') {
                    let r = recipient.trim().to_string();
                    if !r.is_empty() && r.contains('@') {
                        email.recipients.push(r);
                    }
                }
            }
        }
        TAG_DISPLAY_CC => {
            if let Some((text, _)) = decode_with_encoding(data) {
                for cc in text.split(';') {
                    let c = cc.trim().to_string();
                    if !c.is_empty() {
                        email.cc_recipients.push(c);
                    }
                }
            }
        }
        TAG_TRANSPORT_HEADERS => {
            if email.sent_time.is_none() {
                if let Some((text, _)) = decode_with_encoding(data) {
                    for line in text.lines() {
                        if line.to_lowercase().starts_with("date:") {
                            email.sent_time = Some(line[5..].trim().to_string());
                            break;
                        }
                    }
                }
            }
        }
        TAG_CLIENT_SUBMIT_TIME | TAG_MESSAGE_DELIVERY_TIME => {
            if data.len() >= 8 {
                let filetime = u64::from_le_bytes([
                    data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
                ]);
                if let Some(datetime) = filetime_to_string(filetime) {
                    if email.sent_time.is_none() || tag == TAG_CLIENT_SUBMIT_TIME {
                        email.sent_time = Some(datetime);
                    }
                }
            }
        }
        TAG_BODY => {
            if let Some((text, _)) = decode_with_encoding(data) {
                if !text.trim().is_empty() {
                    email.body_text = Some(text);
                }
            }
        }
        TAG_BODY_HTML => {
            if let Some((text, _)) = decode_with_encoding(data) {
                if !text.trim().is_empty() {
                    email.body_html = Some(text);
                }
            }
        }
        TAG_BODY_RTF => {
            if let Ok(decompressed) = compressed_rtf::decompress_rtf(data) {
                if !decompressed.trim().is_empty() {
                    email.body_rtf = Some(decompressed);
                }
            }
        }
        _ => {}
    }
}

fn parse_attachment_internal<R: Read + std::io::Seek>(
    comp: &mut CompoundFile<R>,
    attach_dir: &str,
) -> Result<Attachment, Box<dyn std::error::Error>> {
    let mut attachment = Attachment {
        filename: "未命名附件".to_string(),
        ..Default::default()
    };

    let mut attachment_streams: Vec<(String, PathBuf)> = Vec::new();

    comp.walk().for_each(|entry| {
        let full_path = entry.path();
        let path_str = full_path.to_string_lossy();

        if path_str.contains(attach_dir) && entry.is_stream() {
            let name = entry.name().to_string();
            attachment_streams.push((name, full_path.to_path_buf()));
        }
    });

    for (name, path) in attachment_streams {
        if let Ok(mut stream) = comp.open_stream(&path) {
            let mut stream_data = Vec::new();
            if stream.read_to_end(&mut stream_data).is_ok() {
                let tag = if name.len() >= 8 {
                    &name[name.len() - 8..name.len() - 4]
                } else {
                    continue;
                };

                match tag {
                    TAG_ATTACH_FILENAME_LONG => {
                        if let Some((text, _)) = decode_with_encoding(&stream_data) {
                            attachment.filename = text;
                        }
                    }
                    TAG_ATTACH_FILENAME_SHORT | TAG_ATTACH_DISPLAY_NAME
                        if attachment.filename == "未命名附件" =>
                    {
                        if let Some((text, _)) = decode_with_encoding(&stream_data) {
                            attachment.filename = text;
                        }
                    }
                    TAG_ATTACH_EXTENSION if attachment.filename == "未命名附件" => {
                        if let Some((ext, _)) = decode_with_encoding(&stream_data) {
                            if !ext.is_empty() {
                                attachment.filename = format!("attachment{}", ext);
                            }
                        }
                    }
                    TAG_ATTACH_MIME_TAG => {
                        if let Some((text, _)) = decode_with_encoding(&stream_data) {
                            attachment.content_type = Some(text);
                        }
                    }
                    TAG_ATTACH_CONTENT_ID => {
                        if let Some((text, _)) = decode_with_encoding(&stream_data) {
                            let cid = text
                                .trim()
                                .trim_matches(|c| c == '<' || c == '>')
                                .to_string();
                            if !cid.is_empty() {
                                attachment.content_id = Some(cid);
                            }
                        }
                    }
                    TAG_ATTACH_DATA_BIN => {
                        attachment.data = stream_data;
                    }
                    _ => {}
                }
            }
        }
    }

    if attachment.data.is_empty() && attachment.filename == "未命名附件" {
        return Err("附件数据为空".into());
    }

    Ok(attachment)
}

fn filetime_to_string(filetime: u64) -> Option<String> {
    if filetime == 0 {
        return None;
    }
    const FILETIME_TO_UNIX_EPOCH: u64 = 116444736000000000;
    if filetime < FILETIME_TO_UNIX_EPOCH {
        return None;
    }

    let unix_time = (filetime - FILETIME_TO_UNIX_EPOCH) / 10000000;

    // Improved time calculation
    let total_days = unix_time / 86400;
    let remaining_seconds = unix_time % 86400;
    let hours = remaining_seconds / 3600;
    let minutes = (remaining_seconds % 3600) / 60;
    let seconds = remaining_seconds % 60;

    // Simplistic year/month calculation (good enough for basic display)
    let year = 1970 + total_days / 365;
    let day_of_year = total_days % 365;
    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;

    Some(format!(
        "{}-{:02}-{:02} {:02}:{:02}:{:02} (UTC)",
        year, month, day, hours, minutes, seconds
    ))
}

fn decode_with_encoding(data: &[u8]) -> Option<(String, String)> {
    if data.is_empty() {
        return None;
    }

    // 1. Try UTF-16 LE (most common for modern MSG)
    if data.len() >= 2 && data.len() % 2 == 0 {
        let u16_vec: Vec<u16> = data
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .take_while(|&val| val != 0) // Stop at null terminator
            .collect();

        if !u16_vec.is_empty() {
            let text = String::from_utf16_lossy(&u16_vec);
            let trimmed = text.trim();
            // Heuristic: check if it looks like reasonable text
            if !trimmed.is_empty()
                && trimmed
                    .chars()
                    .any(|c| c.is_alphanumeric() || c.is_whitespace())
            {
                return Some((trimmed.to_string(), "UTF-16 LE".to_string()));
            }
        }
    }

    // 2. Try UTF-8
    if let Ok(text) = String::from_utf8(data.to_vec()) {
        let text = text.trim_end_matches('\0').trim();
        if !text.is_empty() {
            return Some((text.to_string(), "UTF-8".to_string()));
        }
    }

    // 3. Try GBK (common in Chinese environments)
    let (decoded, _, had_errors) = encoding_rs::GBK.decode(data);
    if !had_errors {
        let text = decoded.trim_end_matches('\0').trim();
        if !text.is_empty() {
            return Some((text.to_string(), "GBK".to_string()));
        }
    }

    // 4. Fallback to Lossy UTF-8
    let text = String::from_utf8_lossy(data).to_string();
    let text = text.trim_end_matches('\0').trim();
    if !text.is_empty() {
        return Some((text.to_string(), "UTF-8 (lossy)".to_string()));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filetime_to_string() {
        // 2023-10-27 08:44:20 (UTC) approx
        let ft: u64 = 133428698600000000;
        let s = filetime_to_string(ft).unwrap();
        assert!(s.contains("2023"));
        assert!(s.contains("UTC"));
    }

    #[test]
    fn test_decode_utf16() {
        let data = vec![0x48, 0x00, 0x65, 0x00, 0x6c, 0x00, 0x6c, 0x00, 0x6f, 0x00]; // "Hello" in UTF-16 LE
        let (text, enc) = decode_with_encoding(&data).unwrap();
        assert_eq!(text, "Hello");
        assert_eq!(enc, "UTF-16 LE");
    }

    #[test]
    fn test_decode_utf8() {
        let data = b"Hello UTF-8".to_vec();
        let (text, _) = decode_with_encoding(&data).unwrap();
        assert_eq!(text, "Hello UTF-8");
    }

    #[test]
    fn test_parse_property_subject() {
        let mut email = MsgEmail::default();
        let data = vec![0x54, 0x00, 0x65, 0x00, 0x73, 0x00, 0x74, 0x00]; // "Test" in UTF-16 LE
        parse_property(&mut email, "__substg1.0_0037001F", &data);
        assert_eq!(email.subject, Some("Test".to_string()));
    }

    #[test]

    fn test_parse_property_time() {
        let mut email = MsgEmail::default();

        // 0x01DA08A5E7A0A000 = approx recent time

        let data = 133428698600000000u64.to_le_bytes().to_vec();

        parse_property(&mut email, "__substg1.0_00390040", &data);

        assert!(email.sent_time.is_some());
    }

    #[test]

    fn test_parse_real_msg_file() {
        let file_data = include_bytes!("../target/e990525095f52ef1fadf5cef4fc4864c.msg");

        let result = parse_msg_to_struct(file_data);

        assert!(
            result.is_ok(),
            "Failed to parse MSG file: {:?}",
            result.err()
        );

        let email = result.unwrap();

        println!("Subject: {:?}", email.subject);

        println!("Sender: {:?}", email.sender_name);

        println!("Attachments: {}", email.attachments.len());

        assert!(email.subject.is_some());
    }
}
