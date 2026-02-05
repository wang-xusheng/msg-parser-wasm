use wasm_bindgen::prelude::*;
use serde::{Serialize};
use std::io::{Read, Cursor};
use cfb::CompoundFile;
use std::path::PathBuf;
use encoding_rs;


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
    pub attachments: Vec<Attachment>,
}

/// 附件结构体
#[derive(Debug, Serialize)]
pub struct Attachment {
    pub filename: String,
    pub content_type: Option<String>,
    /// Content-ID，对应 HTML 中 src="cid:xxx" 的 xxx，用于定位正文引用的内嵌附件
    pub content_id: Option<String>,
    // data 字段在序列化为 JSON 时可能会很大，
    // 在 JS 端这会变成一个数字数组。
    // 如果想要优化，可以单独处理，但在 WASM 中这样最简单。
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>, 
}

/// WASM 导出接口
/// 解析 MSG 文件并返回邮件结构体
/// # Arguments
/// * `file_data` - Uint8Array 类型的 MSG 文件数据
/// # Returns - 邮件结构体
/// - subject: 主题
/// - sender_name: 发件人名称
/// - sender_email: 发件人邮箱
/// - recipients: 收件人
/// - cc_recipients: 抄送
/// - sent_time: 发送时间
/// - body_text: 正文
/// - body_html: HTML 正文
/// - attachments: 附件
///   - filename: 文件名
///   - content_type: 内容类型
///   - data: 文件数据
///   - content_id: Content-ID
#[wasm_bindgen]
pub fn parse_msg_file(file_data: &[u8]) -> Result<JsValue, JsValue> {
    // 使用 Cursor 包装内存数据，模拟文件读取
    let cursor = Cursor::new(file_data);
    
    // cfb::CompoundFile::open 接受任何实现了 Read + Seek 的类型
    let mut comp = CompoundFile::open(cursor)
        .map_err(|e| JsValue::from_str(&format!("无法打开 MSG 结构: {}", e)))?;
    
    let mut email = MsgEmail::default();
    
    // 收集流和附件目录
    let mut streams: Vec<(String, PathBuf)> = Vec::new();
    let mut attachment_dirs: Vec<(String, PathBuf)> = Vec::new();
    
    // 注意：在 WASM 环境中，PathBuf 操作的是虚拟路径字符串，这没问题
    comp.walk().for_each(|entry| {
        let name = entry.name().to_string();
        let path = entry.path().to_path_buf();
        
        if name.starts_with("__substg1.0_") {
            streams.push((name.clone(), path));
        } else if name.starts_with("__attach_version1.0_") {
            attachment_dirs.push((name, path));
        }
    });
    
    // 解析属性
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
        // 由于 parse_attachment 需要借用 comp，我们稍微调整内部逻辑
        if let Ok(attachment) = parse_attachment_internal(&mut comp, att_dir) {
            email.attachments.push(attachment);
        }
    }
    
    // 将 Rust 结构体转换为 JS 对象
    serde_wasm_bindgen::to_value(&email)
        .map_err(|e| JsValue::from_str(&format!("序列化失败: {}", e)))
}


/// 解析属性
/// # Arguments
/// * `email` - 邮件结构体
/// * `prop_name` - 属性名称
/// * `data` - 属性数据
fn parse_property(email: &mut MsgEmail, prop_name: &str, data: &[u8]) {
    // 主题 (PR_SUBJECT)
    if prop_name.contains("0037001") {
        if let Some((text, _)) = decode_with_encoding(data) {
            email.subject = Some(text);
        }
    }
    // 发件人名称 (PR_SENDER_NAME)
    else if prop_name.contains("0C1A001") {
        if let Some((text, _)) = decode_with_encoding(data) {
            email.sender_name = Some(text);
        }
    }
    // 发件人邮箱
    else if prop_name.contains("0C1F001") || prop_name.contains("5D01001") || prop_name.contains("0065001") {
        if let Some((text, _)) = decode_with_encoding(data) {
            email.sender_email = Some(text);
        }
    }
    // 收件人
    else if prop_name.contains("0E04001") {
        if let Some((text, _)) = decode_with_encoding(data) {
            for recipient in text.split(';') {
                let r = recipient.trim().to_string();
                if !r.is_empty() {
                    email.recipients.push(r);
                }
            }
        }
    }
    // 收件人邮箱
    else if prop_name.contains("0E03001") || prop_name.contains("0076001") {
        if let Some((text, _)) = decode_with_encoding(data) {
            for recipient in text.split(';') {
                let r = recipient.trim().to_string();
                if !r.is_empty() && r.contains("@") {
                    email.recipients.push(r);
                }
            }
        }
    }
    // 抄送
    else if prop_name.contains("0E02001") {
        if let Some((text, _)) = decode_with_encoding(data) {
            for cc in text.split(';') {
                let c = cc.trim().to_string();
                if !c.is_empty() {
                    email.cc_recipients.push(c);
                }
            }
        }
    }
    // 邮件头日期解析
    else if prop_name.contains("007D001") {
        if let Some((text, _)) = decode_with_encoding(data) {
            if email.sent_time.is_none() {
                for line in text.lines() {
                    if line.starts_with("Date:") {
                        email.sent_time = Some(line[5..].trim().to_string());
                        break;
                    }
                }
            }
        }
    }
    // FileTime 时间解析
    else if prop_name.contains("00390040") || prop_name.contains("0E060040") {
        if data.len() >= 8 {
            let filetime = u64::from_le_bytes([
                data[0], data[1], data[2], data[3],
                data[4], data[5], data[6], data[7],
            ]);
            if let Some(datetime) = filetime_to_string(filetime) {
                // 优先保留第一次解析到的时间，或者覆盖
                if email.sent_time.is_none() || prop_name.contains("00390040") {
                     email.sent_time = Some(datetime);
                }
            }
        }
    }
    // 正文
    else if prop_name.contains("1000001") {
        if let Some((text, _)) = decode_with_encoding(data) {
            if !text.trim().is_empty() {
                email.body_text = Some(text);
            }
        }
    }
    // HTML 正文
    else if prop_name.contains("1013001") || prop_name.contains("10130102") {
        if let Some((text, _)) = decode_with_encoding(data) {
            if !text.trim().is_empty() {
                email.body_html = Some(text);
            }
        }
    }
}

/// 解析附件
/// # Arguments
/// * `comp` - 复合文件
/// * `attach_dir` - 附件目录
/// # Returns
/// * `Result<Attachment, Box<dyn std::error::Error>>` - 附件结构体或错误信息
fn parse_attachment_internal<R: Read + std::io::Seek>(
    comp: &mut CompoundFile<R>, 
    attach_dir: &str
) -> Result<Attachment, Box<dyn std::error::Error>> {
    let mut filename = String::from("未命名附件");
    let mut content_type: Option<String> = None;
    let mut content_id: Option<String> = None;
    let mut data: Vec<u8> = Vec::new();
    
    let mut attachment_streams: Vec<(String, PathBuf)> = Vec::new();
    
    // walk 需要借用 comp，收集完路径后再处理
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
                if name.contains("3707001") { // Long filename
                    if let Some((text, _)) = decode_with_encoding(&stream_data) { filename = text; }
                }
                else if name.contains("3704001") && filename == "未命名附件" { // Short filename
                     if let Some((text, _)) = decode_with_encoding(&stream_data) { filename = text; }
                }
                else if name.contains("3001001") && filename == "未命名附件" { // Display name
                     if let Some((text, _)) = decode_with_encoding(&stream_data) { filename = text; }
                }
                else if name.contains("3703001") { // Extension
                    if let Some((ext, _)) = decode_with_encoding(&stream_data) {
                        if !ext.is_empty() && filename == "未命名附件" {
                            filename = format!("attachment{}", ext);
                        }
                    }
                }
                else if name.contains("370E001") { // Mime tag
                    if let Some((text, _)) = decode_with_encoding(&stream_data) { content_type = Some(text); }
                }
                else if name.contains("3712001") { // PR_ATTACH_CONTENT_ID (0x3712)，用于匹配 body_html 中的 cid:xxx
                    if let Some((text, _)) = decode_with_encoding(&stream_data) {
                        let cid = text.trim().trim_matches(|c| c == '<' || c == '>').to_string();
                        if !cid.is_empty() {
                            content_id = Some(cid);
                        }
                    }
                }
                else if name.contains("37010102") { // Data bin
                    data = stream_data;
                }
            }
        }
    }
    
    if data.is_empty() && filename == "未命名附件" {
        return Err("附件数据为空".into());
    }
    
    Ok(Attachment {
        filename,
        content_type,
        content_id,
        data,
    })
}

fn filetime_to_string(filetime: u64) -> Option<String> {
    if filetime == 0 { return None; }
    const FILETIME_TO_UNIX_EPOCH: u64 = 116444736000000000;
    if filetime < FILETIME_TO_UNIX_EPOCH { return None; }
    
    let unix_time = (filetime - FILETIME_TO_UNIX_EPOCH) / 10000000;
    
    // 简化版时间计算，仅用于展示
    let total_days = unix_time / 86400;
    let remaining_seconds = unix_time % 86400;
    let hours = remaining_seconds / 3600;
    let minutes = (remaining_seconds % 3600) / 60;
    let seconds = remaining_seconds % 60;
    let years = total_days / 365;
    let remaining_days = total_days % 365;
    let year = 1970 + years;
    let month = (remaining_days / 30) + 1;
    let day = (remaining_days % 30) + 1;
    
    Some(format!("{}-{:02}-{:02} {:02}:{:02}:{:02} (UTC)", 
        year, month, day, hours, minutes, seconds))
}

fn decode_with_encoding(data: &[u8]) -> Option<(String, String)> {
    if data.is_empty() { return None; }
    
    // 1. UTF-16 LE
    if data.len() >= 2 && data.len() % 2 == 0 {
        let mut u16_vec = Vec::new();
        for chunk in data.chunks_exact(2) {
            let val = u16::from_le_bytes([chunk[0], chunk[1]]);
            if val == 0 { break; } // 处理空终止符
            u16_vec.push(val);
        }
        if !u16_vec.is_empty() {
            let text = String::from_utf16_lossy(&u16_vec);
            let trimmed = text.trim();
            if !trimmed.is_empty() && trimmed.chars().any(|c| c.is_alphanumeric() || c.is_whitespace()) {
                return Some((trimmed.to_string(), "UTF-16 LE".to_string()));
            }
        }
    }
    
    // 2. UTF-8
    if let Ok(text) = String::from_utf8(data.to_vec()) {
        let text = text.trim_end_matches('\0').trim();
        if !text.is_empty() {
            return Some((text.to_string(), "UTF-8".to_string()));
        }
    }
    
    // 3. GBK
    let (decoded, _, had_errors) = encoding_rs::GBK.decode(data);
    if !had_errors {
        let text = decoded.trim_end_matches('\0').trim();
        if !text.is_empty() {
            return Some((text.to_string(), "GBK".to_string()));
        }
    }
    
    // 4. Lossy UTF-8
    let text = String::from_utf8_lossy(data).to_string();
    let text = text.trim_end_matches('\0').trim();
    if !text.is_empty() {
        return Some((text.to_string(), "UTF-8 (lossy)".to_string()));
    }
    
    None
}