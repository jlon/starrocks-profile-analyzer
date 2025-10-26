/**
 * 全局常量配置
 * 集中管理所有魔法数字和字符串
 */

// API 端点
export const API_ENDPOINTS = {
  UPLOAD_PROFILE: "/api/upload",
  ANALYZE_PROFILE: "/api/analyze",
};

// Severity 颜色配置
export const SEVERITY_COLORS = {
  Severe: "#fa541a",
  High: "#722ed1",
  Moderate: "#fa8c16",
  Mild: "#faad14",
  Normal: "#52c41a",
};

// Severity 标签配置（中文）
export const SEVERITY_LABELS = {
  Severe: "严重",
  High: "高",
  Moderate: "中等",
  Mild: "轻微",
  Normal: "正常",
};

// 默认配置
export const DEFAULT_CONFIG = {
  MAX_FILE_SIZE: 10 * 1024 * 1024, // 10MB
  SUPPORTED_FILE_TYPES: [".txt", ".log"],
  CHART_COLORS: [
    "#1890ff",
    "#13c2c2",
    "#722ed1",
    "#52c41a",
    "#fa8c16",
    "#fa541c",
  ],
};

/**
 * 获取 Severity 颜色
 * @param {string} severity - Severity 枚举值
 * @returns {string} 颜色值
 */
export function getSeverityColor(severity) {
  return SEVERITY_COLORS[severity] || "#1890ff";
}

/**
 * 获取 Severity 标签
 * @param {string} severity - Severity 枚举值
 * @returns {string} 显示标签
 */
export function getSeverityLabel(severity) {
  return SEVERITY_LABELS[severity] || severity;
}
