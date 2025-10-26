/**
 * Bytes 格式化工具
 * 对应后端: backend/src/parser/core/value_parser.rs::parse_bytes
 *
 * ⚠️ 与后端解析逻辑保持一致
 */

/**
 * 格式化字节数为可读字符串
 * @param {number|string} bytes - 字节数
 * @param {number} decimals - 小数位数
 * @returns {string} 格式化后的字符串
 */
export function formatBytes(bytes, decimals = 3) {
  // 处理各种输入类型
  if (bytes === null || bytes === undefined || bytes === "") {
    return "N/A";
  }

  // 转换为数字
  const bytesNum = typeof bytes === "string" ? parseFloat(bytes) : bytes;

  if (isNaN(bytesNum) || bytesNum === 0) {
    return "0.000B";
  }

  const k = 1024; // StarRocks uses 1024, not 1000
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ["B", "KB", "MB", "GB", "TB"];

  const i = Math.floor(Math.log(bytesNum) / Math.log(k));
  const value = bytesNum / Math.pow(k, i);

  return `${value.toFixed(dm)}${sizes[i]}`;
}

/**
 * 格式化带括号的字节数（StarRocks 格式）
 * @param {number} bytes - 字节数
 * @returns {string} 格式化后的字符串，如 "2.174K (2174)"
 */
export function formatBytesWithParen(bytes) {
  if (!bytes || bytes === 0) return "0 (0)";

  const readable = formatBytes(bytes, 3);
  return `${readable} (${bytes})`;
}

/**
 * 格式化为简短形式（只保留1位小数）
 * @param {number} bytes - 字节数
 * @returns {string} 格式化后的字符串
 */
export function formatBytesShort(bytes) {
  return formatBytes(bytes, 1);
}
