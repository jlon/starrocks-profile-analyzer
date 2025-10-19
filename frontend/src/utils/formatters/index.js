/**
 * Formatters 统一导出
 */

export * from './durationFormatter';
export * from './bytesFormatter';

/**
 * 格式化数字（添加千位分隔符）
 * @param {number} num - 数字
 * @returns {string} 格式化后的字符串
 */
export function formatNumber(num) {
  if (num === null || num === undefined || num === '') {
    return 'N/A';
  }
  
  const numValue = typeof num === 'string' ? parseFloat(num) : num;
  
  if (isNaN(numValue)) {
    return 'N/A';
  }
  
  return numValue.toLocaleString();
}

/**
 * 格式化百分比
 * @param {number} value - 数值 (0-1 或 0-100)
 * @param {boolean} isDecimal - 是否为小数形式 (0-1)
 * @param {number} decimals - 小数位数
 * @returns {string} 格式化后的字符串
 */
export function formatPercentage(value, isDecimal = true, decimals = 2) {
  if (value === null || value === undefined || value === '') {
    return 'N/A';
  }
  
  const numValue = typeof value === 'string' ? parseFloat(value) : value;
  
  if (isNaN(numValue)) {
    return 'N/A';
  }
  
  const percent = isDecimal ? numValue * 100 : numValue;
  return `${percent.toFixed(decimals)}%`;
}

