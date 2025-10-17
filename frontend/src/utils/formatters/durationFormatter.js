/**
 * Duration 格式化工具
 * 对应后端: backend/src/parser/core/value_parser.rs
 * 
 * ⚠️ 与后端解析逻辑保持一致
 */

/**
 * 格式化纳秒为可读字符串
 * @param {number|string} nanos - 纳秒数（可能是字符串）
 * @returns {string} 格式化后的字符串
 */
export function formatDuration(nanos) {
  // 处理各种输入类型
  if (nanos === null || nanos === undefined || nanos === '') {
    return 'N/A';
  }
  
  // 转换为数字
  const nanosNum = typeof nanos === 'string' ? parseFloat(nanos) : nanos;
  
  if (isNaN(nanosNum) || nanosNum === 0) {
    return '0ns';
  }
  
  const units = [
    { name: 'h', value: 3600 * 1e9 },
    { name: 'm', value: 60 * 1e9 },
    { name: 's', value: 1e9 },
    { name: 'ms', value: 1e6 },
    { name: 'us', value: 1e3 },
    { name: 'ns', value: 1 },
  ];
  
  const parts = [];
  let remaining = nanosNum;
  
  for (const unit of units) {
    if (remaining >= unit.value) {
      const count = Math.floor(remaining / unit.value);
      parts.push(`${count}${unit.name}`);
      remaining -= count * unit.value;
      
      // 只显示前两个单位
      if (parts.length >= 2) break;
    }
  }
  
  return parts.length > 0 ? parts.join('') : '0ns';
}

/**
 * 格式化毫秒为可读字符串
 * @param {number} millis - 毫秒数
 * @returns {string} 格式化后的字符串
 */
export function formatMillis(millis) {
  if (!millis || millis === 0) return '0ms';
  return formatDuration(millis * 1e6);
}

/**
 * 格式化秒为可读字符串
 * @param {number} seconds - 秒数
 * @returns {string} 格式化后的字符串
 */
export function formatSeconds(seconds) {
  if (!seconds || seconds === 0) return '0s';
  return formatDuration(seconds * 1e9);
}

/**
 * 解析 Duration 对象（如果后端返回的是对象格式）
 * @param {Object|number|string} duration - Duration 对象或纳秒数
 * @returns {string} 格式化后的字符串
 */
export function parseDuration(duration) {
  if (!duration) return 'N/A';
  
  // 如果是对象，尝试提取 nanos 或 secs_nsecs
  if (typeof duration === 'object') {
    if (duration.nanos !== undefined) {
      return formatDuration(duration.nanos);
    }
    if (duration.secs !== undefined && duration.nanos !== undefined) {
      const totalNanos = duration.secs * 1e9 + duration.nanos;
      return formatDuration(totalNanos);
    }
  }
  
  // 如果是数字或字符串，直接格式化
  return formatDuration(duration);
}

