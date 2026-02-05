/* tslint:disable */
/* eslint-disable */
/**
 * WASM 导出接口
 * 解析 MSG 文件并返回邮件结构体
 * # Arguments
 * * `file_data` - Uint8Array 类型的 MSG 文件数据
 * # Returns - 邮件结构体
 * - subject: 主题
 * - sender_name: 发件人名称
 * - sender_email: 发件人邮箱
 * - recipients: 收件人
 * - cc_recipients: 抄送
 * - sent_time: 发送时间
 * - body_text: 正文
 * - body_html: HTML 正文
 * - attachments: 附件
 *   - filename: 文件名
 *   - content_type: 内容类型
 *   - data: 文件数据
 *   - content_id: Content-ID
 */
export function parse_msg_file(file_data: Uint8Array): any;
