//! # ValueParser - 值解析器
//! 
//! 基于 StarRocks 源码 (`RuntimeProfile.java`, `DebugUtil.java`) 的值格式化规则实现解析。
//! 
//! ## 支持的格式
//! 
//! ### 时间 (TIME_NS)
//! - `1h30m` - 小时+分钟 (忽略秒和毫秒)
//! - `45m30s` - 分钟+秒 (忽略毫秒)
//! - `30s500ms` - 秒+毫秒
//! - `7s854ms` - 秒+毫秒
//! - `123ms` 或 `123.456ms` - 毫秒
//! - `500us` 或 `5.540us` - 微秒
//! - `390ns` - 纳秒
//! 
//! ### 字节 (BYTES)
//! - `2.167 KB` - 千字节
//! - `12.768 GB` - 吉字节
//! - `0.000 B` - 字节
//! 
//! ### 大数字单位 (UNIT)
//! - `2.174K (2174)` - 格式化值 + 原始值 (优先使用原始值)
//! - `334` - 小数字直接显示
//! - `1.234M (1234567)` - 百万

use crate::parser::error::{ParseError, ParseResult};
use once_cell::sync::Lazy;
use regex::Regex;
use std::time::Duration;

// ========== 预编译正则表达式 ==========

/// 匹配时间组件: `1h`, `30m`, `45s`, `123ms`, `5.540us`, `390ns`
/// 注意：必须先匹配 ms, us, ns (长单位)，再匹配 m, s, h (短单位)，否则 "123ms" 会被错误匹配为 "123m" + "s"
static TIME_COMPONENT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\d+(?:\.\d+)?)\s*(ms|us|μs|ns|h|m|s)").unwrap()
});

/// 匹配字节格式: `2.167 KB`, `12.768 GB`
static BYTES_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([\d,.]+)\s*(TB|GB|MB|KB|K|M|G|T|B)$").unwrap()
});

/// 匹配带括号的数字格式: `2.174K (2174)`
static NUMBER_WITH_PAREN_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[\d,.]+[KMGB]?\s*\((\d+)\)").unwrap()
});

/// 匹配普通数字（可能带逗号）
static NUMBER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([\d,.]+)").unwrap()
});

pub struct ValueParser;

impl ValueParser {
    // ========== 时间解析 ==========
    
    /// 解析时间字符串为 Duration
    /// 
    /// 基于 `RuntimeProfile.printCounter()` 和 `DebugUtil.printTimeMs()` 的格式。
    /// 
    /// # 支持的格式
    /// - `1h30m` -> 90 minutes
    /// - `5s500ms` -> 5.5 seconds
    /// - `123.456ms` -> 123.456 milliseconds
    /// - `5.540us` -> 5.540 microseconds
    /// - `390ns` -> 390 nanoseconds
    /// - `0ns` -> 0
    /// 
    /// # Examples
    /// ```
    /// use starrocks_profile_analyzer::parser::ValueParser;
    /// let d = ValueParser::parse_duration("1h30m").unwrap();
    /// assert_eq!(d.as_secs(), 5400);
    /// 
    /// let d = ValueParser::parse_duration("7s854ms").unwrap();
    /// assert_eq!(d.as_millis(), 7854);
    /// ```
    pub fn parse_duration(input: &str) -> ParseResult<Duration> {
        let input = input.trim();
        
        // 特殊处理 SR 生成的纯 "0" (无单位)
        // SR源码: DebugUtil.printTimeMs(0) -> "0"
        if input == "0" {
            return Ok(Duration::from_nanos(0));
        }
        
        let mut total_ns: f64 = 0.0;
        let mut found_any = false;
        
        // 使用正则匹配所有时间组件
        for cap in TIME_COMPONENT_REGEX.captures_iter(input) {
            found_any = true;
            
            let num_str = cap.get(1).unwrap().as_str();
            let num: f64 = num_str.parse().map_err(|_| ParseError::ParseDurationError(
                format!("Invalid number '{}' in duration '{}'", num_str, input)
            ))?;
            
            let unit = cap.get(2).unwrap().as_str();
            
            // 转换为纳秒
            let ns = match unit {
                "h" => num * 3600.0 * 1_000_000_000.0,
                "m" => num * 60.0 * 1_000_000_000.0,
                "s" => num * 1_000_000_000.0,
                "ms" => num * 1_000_000.0,
                "us" | "μs" => num * 1_000.0,
                "ns" => num,
                _ => 0.0,
            };
            
            total_ns += ns;
        }
        
        if !found_any {
            return Err(ParseError::ParseDurationError(
                format!("No valid time components found in '{}'", input)
            ));
        }
        
        Ok(Duration::from_nanos(total_ns as u64))
    }
    
    /// 解析时间为毫秒数 (f64)
    /// 
    /// 便捷方法，用于存储到数据模型。保留小数精度以确保准确的百分比计算。
    pub fn parse_time_to_ms(input: &str) -> ParseResult<f64> {
        let duration = Self::parse_duration(input)?;
        Ok(duration.as_nanos() as f64 / 1_000_000.0)
    }
    
    // ========== 字节解析 ==========
    
    /// 解析字节大小字符串为 u64
    /// 
    /// 基于 `DebugUtil.getByteUint()` 的格式。
    /// 
    /// # 支持的格式
    /// - `2.167 KB` -> 2218 bytes
    /// - `12.768 GB` -> ~13.7 billion bytes
    /// - `0.000 B` -> 0 bytes
    /// - `2.174K (2174)` -> 优先使用括号内的 2174
    /// 
    /// # Examples
    /// ```
    /// use starrocks_profile_analyzer::parser::ValueParser;
    /// let bytes = ValueParser::parse_bytes("2.167 KB").unwrap();
    /// assert_eq!(bytes, 2219);
    /// 
    /// let bytes = ValueParser::parse_bytes("2.174K (2174)").unwrap();
    /// assert_eq!(bytes, 2174);
    /// ```
    pub fn parse_bytes(input: &str) -> ParseResult<u64> {
        let original = input.trim();
        let input = original.to_uppercase();
        
        // 优先检查括号内的原始值
        if let Some(cap) = NUMBER_WITH_PAREN_REGEX.captures(&input) {
            let raw = cap.get(1).unwrap().as_str();
            return raw.parse::<u64>().map_err(|e| ParseError::ParseBytesError(
                format!("Failed to parse raw bytes '{}': {}", raw, e)
            ));
        }
        
        // 使用正则解析格式化的字节值
        if let Some(cap) = BYTES_REGEX.captures(&input) {
            let num_str = cap.get(1).unwrap().as_str().replace(",", "");
            let num: f64 = num_str.parse().map_err(|e| ParseError::ParseBytesError(
                format!("Invalid number '{}': {}", num_str, e)
            ))?;
            
            let unit = cap.get(2).unwrap().as_str();
            
            // 基于 1024 的倍数 (StarRocks 使用 1024)
            let multiplier: f64 = match unit {
                "B" => 1.0,
                "K" | "KB" => 1024.0,
                "M" | "MB" => 1024.0 * 1024.0,
                "G" | "GB" => 1024.0 * 1024.0 * 1024.0,
                "T" | "TB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
                _ => return Err(ParseError::ParseBytesError(
                    format!("Unknown byte unit: {}", unit)
                )),
            };
            
            // 统一采用向下取整（与用例和SR显示逻辑对齐）
            return Ok((num * multiplier).floor() as u64);
        }
        
        // 尝试作为纯数字解析
        let temp = input.replace(",", "");
        let cleaned = temp.split_whitespace().next().unwrap_or(&input);
        cleaned.parse::<u64>().map_err(|e| ParseError::ParseBytesError(
            format!("Cannot parse bytes from '{}': {}", input, e)
        ))
    }
    
    // ========== 数字解析 ==========
    
    /// 解析通用数字
    /// 
    /// 支持：
    /// - 带括号的格式化数字: `2.174K (2174)` -> 优先返回 2174
    /// - 逗号分隔: `1,234,567` -> 1234567
    /// - 普通整数: `334` -> 334
    /// - 浮点数: `12.34` -> 解析为相应类型
    /// 
    /// # Examples
    /// ```
    /// use starrocks_profile_analyzer::parser::ValueParser;
    /// let n: u64 = ValueParser::parse_number("2.174K (2174)").unwrap();
    /// assert_eq!(n, 2174);
    /// 
    /// let n: u64 = ValueParser::parse_number("1,234").unwrap();
    /// assert_eq!(n, 1234);
    /// ```
    pub fn parse_number<T>(input: &str) -> ParseResult<T>
    where
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::fmt::Display,
    {
        let input = input.trim();
        
        // 1. 优先检查括号内的原始值 (针对 2.174K (2174) 格式)
        if let Some(cap) = NUMBER_WITH_PAREN_REGEX.captures(input) {
            let raw = cap.get(1).unwrap().as_str();
            return raw.parse::<T>().map_err(|e| ParseError::ParseNumberError(
                format!("Failed to parse number from parentheses '{}': {}", raw, e)
            ));
        }
        
        // 2. 提取第一个数字部分（去除逗号）
        if let Some(cap) = NUMBER_REGEX.captures(input) {
            let num_str = cap.get(1).unwrap().as_str().replace(",", "");
            return num_str.parse::<T>().map_err(|e| ParseError::ParseNumberError(
                format!("Failed to parse number '{}': {}", num_str, e)
            ));
        }
        
        Err(ParseError::ParseNumberError(
            format!("Cannot extract number from '{}'", input)
        ))
    }
    
    // ========== 百分比解析 ==========
    
    /// 解析百分比字符串为 f64 (0.0 - 100.0)
    /// 
    /// # Examples
    /// ```
    /// use starrocks_profile_analyzer::parser::ValueParser;
    /// let pct = ValueParser::parse_percentage("85.5%").unwrap();
    /// assert_eq!(pct, 85.5);
    /// 
    /// let pct = ValueParser::parse_percentage("12.34").unwrap();
    /// assert_eq!(pct, 12.34);
    /// ```
    pub fn parse_percentage(input: &str) -> ParseResult<f64> {
        let input = input.trim().trim_end_matches('%');
        input.parse::<f64>().map_err(|e| ParseError::ParseNumberError(
            format!("Failed to parse percentage '{}': {}", input, e)
        ))
    }
    
    // ========== 布尔值解析 ==========
    
    /// 解析布尔值
    /// 
    /// 支持: `true`, `false`, `yes`, `no`, `1`, `0` (不区分大小写)
    pub fn parse_bool(input: &str) -> ParseResult<bool> {
        match input.trim().to_lowercase().as_str() {
            "true" | "yes" | "1" => Ok(true),
            "false" | "no" | "0" => Ok(false),
            _ => Err(ParseError::ValueError {
                value: input.to_string(),
                reason: "Invalid boolean value".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // ========== 时间解析测试 ==========
    
    #[test]
    fn test_parse_duration_hours_minutes() {
        let d = ValueParser::parse_duration("1h30m").unwrap();
        assert_eq!(d.as_secs(), 5400); // 90 minutes
    }
    
    #[test]
    fn test_parse_duration_seconds_millis() {
        let d = ValueParser::parse_duration("7s854ms").unwrap();
        assert_eq!(d.as_nanos(), 7_854_000_000); // 7s854ms = 7,854,000,000ns
    }
    
    #[test]
    fn test_parse_duration_millis() {
        let d = ValueParser::parse_duration("123ms").unwrap();
        assert_eq!(d.as_nanos(), 123_000_000); // 123ms = 123,000,000ns
        
        let d = ValueParser::parse_duration("123.456ms").unwrap();
        assert_eq!(d.as_nanos(), 123_456_000); // 123.456ms = 123,456,000ns
    }
    
    #[test]
    fn test_parse_duration_micros() {
        let d = ValueParser::parse_duration("5.540us").unwrap();
        assert_eq!(d.as_nanos(), 5540);
    }
    
    #[test]
    fn test_parse_duration_nanos() {
        let d = ValueParser::parse_duration("390ns").unwrap();
        assert_eq!(d.as_nanos(), 390);
    }
    
    #[test]
    fn test_parse_duration_zero() {
        let d = ValueParser::parse_duration("0ns").unwrap();
        assert_eq!(d.as_nanos(), 0);
    }
    
    #[test]
    fn test_parse_duration_zero_without_unit() {
        // SR源码特殊情况: printTimeMs(0) -> "0" (无单位)
        let d = ValueParser::parse_duration("0").unwrap();
        assert_eq!(d.as_nanos(), 0);
    }
    
    // ========== 字节解析测试 ==========
    
    #[test]
    fn test_parse_bytes_with_unit() {
        // Note: KB in SR = 1024 bytes
        assert_eq!(ValueParser::parse_bytes("2.167KB").unwrap(), 2219); // 2.167 * 1024 = 2219.008 → floor = 2219
        // 实际值：12.768 * 1024^3 = 13,709,535,608 (不是预期的 13,707,190,067)
        assert_eq!(ValueParser::parse_bytes("12.768GB").unwrap(), 13709535608); // 12.768 * 1024^3
        assert_eq!(ValueParser::parse_bytes("0.000B").unwrap(), 0);
    }
    
    #[test]
    fn test_parse_bytes_with_parentheses() {
        assert_eq!(ValueParser::parse_bytes("2.174K (2174)").unwrap(), 2174);
        assert_eq!(ValueParser::parse_bytes("1.234M (1234567)").unwrap(), 1234567);
    }
    
    #[test]
    fn test_parse_bytes_plain_number() {
        assert_eq!(ValueParser::parse_bytes("1024").unwrap(), 1024);
    }
    
    // ========== 数字解析测试 ==========
    
    #[test]
    fn test_parse_number_with_parentheses() {
        let n: u64 = ValueParser::parse_number("2.174K (2174)").unwrap();
        assert_eq!(n, 2174);
    }
    
    #[test]
    fn test_parse_number_with_commas() {
        let n: u64 = ValueParser::parse_number("1,234,567").unwrap();
        assert_eq!(n, 1234567);
    }
    
    #[test]
    fn test_parse_number_plain() {
        let n: u64 = ValueParser::parse_number("334").unwrap();
        assert_eq!(n, 334);
    }
    
    #[test]
    fn test_parse_number_float() {
        let n: f64 = ValueParser::parse_number("12.34").unwrap();
        assert!((n - 12.34).abs() < 0.001);
    }
    
    // ========== 百分比解析测试 ==========
    
    #[test]
    fn test_parse_percentage() {
        assert!((ValueParser::parse_percentage("85.5%").unwrap() - 85.5).abs() < 0.001);
        assert!((ValueParser::parse_percentage("12.34").unwrap() - 12.34).abs() < 0.001);
    }
}
