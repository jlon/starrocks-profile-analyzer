//! # ValueParser 集成测试
//! 
//! 测试基于 StarRocks 实际 Profile 格式的值解析

use starrocks_profile_analyzer::parser::core::value_parser::ValueParser;

#[test]
fn test_parse_time_formats() {
    // 测试各种时间格式 (基于实际 Profile)
    
    // 小时+分钟格式
    let d = ValueParser::parse_duration("1h30m").unwrap();
    assert_eq!(d.as_secs(), 5400);
    
    // 分钟格式
    let d = ValueParser::parse_duration("9m41s").unwrap();
    assert_eq!(d.as_secs(), 581);
    
    // 秒+毫秒格式
    let d = ValueParser::parse_duration("7s854ms").unwrap();
    assert_eq!(d.as_millis(), 7854);
    
    // 毫秒格式 (带小数)
    let d = ValueParser::parse_duration("123.456ms").unwrap();
    assert_eq!(d.as_micros(), 123456);
    
    // 微秒格式 (带小数)
    let d = ValueParser::parse_duration("5.540us").unwrap();
    assert_eq!(d.as_nanos(), 5540);
    
    // 纳秒格式
    let d = ValueParser::parse_duration("390ns").unwrap();
    assert_eq!(d.as_nanos(), 390);
    
    // 零值
    let d = ValueParser::parse_duration("0ns").unwrap();
    assert_eq!(d.as_nanos(), 0);
}

#[test]
fn test_parse_bytes_formats() {
    // 测试各种字节格式 (基于实际 Profile)
    
    // 带小数的 KB
    let bytes = ValueParser::parse_bytes("2.167 KB").unwrap();
    assert_eq!(bytes, 2219); // 2.167 * 1024 (向下取整)
    
    // GB 格式
    let bytes = ValueParser::parse_bytes("12.768 GB").unwrap();
    assert_eq!(bytes, 13709535608); // 12.768 * 1024^3
    
    // 零字节
    let bytes = ValueParser::parse_bytes("0.000 B").unwrap();
    assert_eq!(bytes, 0);
    
    // 括号格式 (优先使用原始值)
    let bytes = ValueParser::parse_bytes("2.174K (2174)").unwrap();
    assert_eq!(bytes, 2174);
    
    // MB 格式
    let bytes = ValueParser::parse_bytes("1.768 MB").unwrap();
    assert_eq!(bytes, 1853882); // 1.768 * 1024^2
}

#[test]
fn test_parse_number_formats() {
    // 普通整数
    let n: u64 = ValueParser::parse_number("334").unwrap();
    assert_eq!(n, 334);
    
    // 带逗号的数字
    let n: u64 = ValueParser::parse_number("1,234,567").unwrap();
    assert_eq!(n, 1234567);
    
    // 括号格式 (优先使用原始值)
    let n: u64 = ValueParser::parse_number("2.174K (2174)").unwrap();
    assert_eq!(n, 2174);
    
    let n: u64 = ValueParser::parse_number("1.234M (1234567)").unwrap();
    assert_eq!(n, 1234567);
    
    // 浮点数
    let n: f64 = ValueParser::parse_number("12.34").unwrap();
    assert!((n - 12.34).abs() < 0.001);
}

#[test]
fn test_parse_percentage() {
    // 带百分号
    let pct = ValueParser::parse_percentage("85.5%").unwrap();
    assert!((pct - 85.5).abs() < 0.001);
    
    // 不带百分号
    let pct = ValueParser::parse_percentage("12.34").unwrap();
    assert!((pct - 12.34).abs() < 0.001);
}

#[test]
fn test_time_to_ms_conversion() {
    // 测试转换为毫秒
    let ms = ValueParser::parse_time_to_ms("1h30m").unwrap();
    assert_eq!(ms, 5400000);
    
    let ms = ValueParser::parse_time_to_ms("7s854ms").unwrap();
    assert_eq!(ms, 7854);
    
    let ms = ValueParser::parse_time_to_ms("123.456ms").unwrap();
    assert_eq!(ms, 123);
}

#[test]
fn test_real_profile_values() {
    // 测试实际 Profile 中出现的值
    
    // 从 test_profile.txt
    let bytes = ValueParser::parse_bytes("259.547 GB").unwrap();
    assert!(bytes > 250_000_000_000);
    
    let bytes = ValueParser::parse_bytes("2.174K (2174)").unwrap();
    assert_eq!(bytes, 2174);
    
    let d = ValueParser::parse_duration("1h30m").unwrap();
    assert_eq!(d.as_secs(), 5400);
    
    let d = ValueParser::parse_duration("753.846us").unwrap();
    assert_eq!(d.as_micros(), 753);
    
    // 从 profile1.txt
    let d = ValueParser::parse_duration("9m41s").unwrap();
    assert_eq!(d.as_secs(), 581);
    
    let bytes = ValueParser::parse_bytes("558.156 GB").unwrap();
    assert!(bytes > 500_000_000_000);
}

#[test]
fn test_edge_cases() {
    // 空格处理
    let n: u64 = ValueParser::parse_number("  123  ").unwrap();
    assert_eq!(n, 123);
    
    // 大小写不敏感
    let bytes = ValueParser::parse_bytes("2.167 kb").unwrap();
    assert_eq!(bytes, 2219);
    
    let bytes = ValueParser::parse_bytes("1 gb").unwrap();
    assert_eq!(bytes, 1073741824);
}
