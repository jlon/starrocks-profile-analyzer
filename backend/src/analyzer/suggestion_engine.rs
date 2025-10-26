use crate::models::*;
use std::time::Duration;

pub struct SuggestionEngine;

impl SuggestionEngine {
    pub fn quick_execution_overview_check(profile: &Profile) -> Vec<HotSpot> {
        let mut hotspots = Vec::new();

        if let Some(mem_usage_str) = profile.execution.metrics.get("QueryPeakMemoryUsagePerNode") {
            if let Ok(mem_bytes) = Self::parse_bytes(mem_usage_str) {
                let total_memory = 12_884_901_888u64;
                let mem_percentage = (mem_bytes as f64 / total_memory as f64) * 100.0;
                if mem_percentage > 80.0 {
                    hotspots.push(HotSpot {
                        node_path: "Execution.Overview".to_string(),
                        severity: HotSeverity::Critical,
                        issue_type: "MemoryUsage".to_string(),
                        description: format!("内存使用率过高: {:.1}% (超过80%阈值)", mem_percentage),
                        suggestions: vec![
                            "检查内存配置参数 (be.conf 中 mem_limit)".to_string(),
                            "考虑启用可溢出运算符 (spillable_operators)".to_string(),
                            "优化查询以减少内存占用 (减少JOIN大小或使用分区)".to_string(),
                            "增加BE节点内存或扩展集群".to_string(),
                        ],
                    });
                }
            }
        }

        if let Some(spill_str) = profile.execution.metrics.get("QuerySpillBytes") {
            if let Ok(spill_bytes) = Self::parse_bytes(spill_str) {
                if spill_bytes > 1_073_741_824 {
                    hotspots.push(HotSpot {
                        node_path: "Execution.Overview".to_string(),
                        severity: HotSeverity::High,
                        issue_type: "DiskSpill".to_string(),
                        description: format!("磁盘溢出严重: {} (超过1GB阈值)", Self::format_bytes(spill_bytes)),
                        suggestions: vec![
                            "增加内存分配给查询".to_string(),
                            "检查是否存在数据倾斜导致的内存不足".to_string(),
                            "启用自适应降级 (enable_adaptive_sink_dop)".to_string(),
                            "优化JOIN顺序以减少内存使用".to_string(),
                        ],
                    });
                }
            }
        }

        hotspots
    }

    pub fn find_slowest_operators(profile: &Profile) -> Vec<OperatorSummary> {
        let mut operators = Vec::new();

        for fragment in &profile.fragments {
            for pipeline in &fragment.pipelines {
                for operator in &pipeline.operators {
                    if let Some(time_str) = operator.common_metrics.get("OperatorTotalTime") {
                        if let Ok(duration) = Self::parse_duration(time_str) {
                            let time_percentage = Self::calculate_time_percentage(duration, &profile.summary.total_time);
                            operators.push(OperatorSummary {
                                operator_name: operator.name.clone(),
                                fragment_id: fragment.id.clone(),
                                pipeline_id: pipeline.id.clone(),
                                total_time: duration,
                                time_percentage,
                            });
                        }
                    }
                }
            }
        }


        operators.sort_by(|a, b| b.total_time.cmp(&a.total_time));
        operators.truncate(10);
        operators
    }

    pub fn generate_official_recipes(hotspot_type: &str, _context: &OperatorContext) -> Vec<String> {
        match hotspot_type {
            "cold_storage" => vec![
                "将热数据迁移到NVMe/SSD存储".to_string(),
                "启用存储缓存配置 (storage_root_path 指定SSD路径)".to_string(),
                "提升远程缓存容量 (remote_cache_capacity 参数)".to_string(),
                "检查存储系统IOPS是否充足".to_string(),
            ],
            "missing_predicate_pushdown" => vec![
                "重写谓词为简单比较条件 (避免 LIKE '%xxx%')".to_string(),
                "添加zonemap索引 (适合范围查询)".to_string(),
                "添加Bloom索引 (适合等值查询)".to_string(),
                "创建物化视图以便谓词下推".to_string(),
            ],
            "thread_pool_starvation" => vec![
                "增加BE I/O线程池大小 (max_io_threads_per_disk)".to_string(),
                "增加扫描线程池大小 (num_io_threads_backlog)".to_string(),
                "启用存储缓存以减少I/O压力".to_string(),
                "考虑数据预热减少冷读".to_string(),
            ],
            "data_skew" => vec![
                "重新选择hash分桶键 (选择分布均匀的列)".to_string(),
                "增加分桶数量以分散数据".to_string(),
                "使用倾斜数据处理 (enable_skew_optimization)".to_string(),
                "检查统计信息是否最新".to_string(),
            ],
            "fragmented_rowsets" => vec![
                "触发手动compaction (ALTER TABLE ... COMPACT)".to_string(),
                "批量导入小文件合并".to_string(),
                "调整compaction参数 (cumulative_compaction_num_deltas)".to_string(),
                "启用自动compaction调度".to_string(),
            ],
            "suboptimal_aggregation" => vec![
                "检查是否使用一阶段聚合 (large dataset不适合)".to_string(),
                "考虑分布式预聚合 (enable_distributed_aggregation)".to_string(),
                "启用可溢出聚合 (spillable_operators)".to_string(),
                "优化GROUP BY键的选择".to_string(),
            ],
            _ => vec![
                "检查操作符具体指标和调优文档".to_string(),
                "分析操作符时间占比确定优化优先级".to_string(),
            ]
        }
    }

    pub fn generate_conclusion(hotspots: &[HotSpot], profile: &Profile) -> String {
        if hotspots.is_empty() {
            return "查询执行良好，未发现明显性能问题。".to_string();
        }

        let severe_count = hotspots.iter().filter(|h| matches!(h.severity, HotSeverity::Severe | HotSeverity::Critical)).count();
        let moderate_count = hotspots.iter().filter(|h| matches!(h.severity, HotSeverity::Moderate)).count();
        let _mild_count = hotspots.iter().filter(|h| matches!(h.severity, HotSeverity::Mild)).count();

        let total_time = Self::parse_total_time(&profile.summary.total_time).unwrap_or(0.0);

        let conclusion = if severe_count > 0 {
            format!(
                "查询存在{}个严重性能问题，执行时间较长（{}）。主要问题是{}。建议优先解决严重问题。",
                severe_count,
                Self::format_duration(total_time),
                hotspots.first().unwrap().issue_type
            )
        } else if moderate_count > 2 {
            format!(
                "查询存在{}个中等程度性能问题，整体性能需优化。执行时间{}。",
                moderate_count,
                Self::format_duration(total_time)
            )
        } else if total_time > 300.0f64 {
            format!("查询执行时间较长（{}），建议关注性能热点。", Self::format_duration(total_time))
        } else {
            format!("查询发现{}个小问题，整体性能可接受。", hotspots.len())
        };

        conclusion
    }

    pub fn generate_suggestions(hotspots: &[HotSpot]) -> Vec<String> {
        let mut suggestions = Vec::new();
        let mut unique_suggestions = std::collections::HashSet::new();

        for hotspot in hotspots {
            for suggestion in &hotspot.suggestions {
                if unique_suggestions.insert(suggestion.clone()) {
                    suggestions.push(suggestion.clone());
                }
            }
        }


        let general_suggestions = vec![
            "考虑启用查询缓存以提高重复查询的性能".to_string(),
            "检查硬件资源（CPU、内存、存储）是否充足".to_string(),
            "定期维护表统计信息以优化查询计划".to_string(),
            "考虑使用查询队列管理来避免资源争用".to_string(),
        ];

        for suggestion in general_suggestions {
            if unique_suggestions.insert(suggestion.clone()) {
                suggestions.push(suggestion);
            }
        }

        suggestions
    }

    pub fn calculate_performance_score(hotspots: &[HotSpot], profile: &Profile) -> f64 {
        let mut score: f64 = 100.0;


        for hotspot in hotspots {
            let penalty = match hotspot.severity {
                HotSeverity::Critical => 25.0,
                HotSeverity::Severe => 15.0,
                HotSeverity::High => 12.0,
                HotSeverity::Moderate => 8.0,
                HotSeverity::Mild => 3.0,
                HotSeverity::Normal => 0.0,
            };
            score -= penalty;
        }


        if let Ok(total_seconds) = Self::parse_total_time(&profile.summary.total_time) {
            if total_seconds > 3600.0 {
                score -= 20.0;
            } else if total_seconds > 1800.0 {
                score -= 10.0;
            } else if total_seconds > 300.0 {
                score -= 5.0;
            }
        }

        score.max(0.0)
    }

    fn parse_total_time(time_str: &str) -> Result<f64, ()> {
        if let Some(hours_part) = time_str.split("h").next() {
            if let Ok(hours) = hours_part.parse::<f64>() {
                let minutes_str = time_str.split("h").nth(1).unwrap_or("0").replace("m", "");
                if let Ok(minutes) = minutes_str.parse::<f64>() {
                    return Ok(hours * 3600.0 + minutes * 60.0);
                }
            }
        }
        Err(())
    }

    fn format_duration(seconds: f64) -> String {
        if seconds >= 3600.0 {
            format!("{:.1}小时", seconds / 3600.0)
        } else if seconds >= 60.0 {
            format!("{:.0}分钟", seconds / 60.0)
        } else {
            format!("{:.1}秒", seconds)
        }
    }

    pub fn parse_memory_percentage(mem_str: &str) -> Result<f64, ()> {
        if mem_str.contains('%') {
            mem_str.replace('%', "").trim().parse::<f64>().map_err(|_| ())
        } else {
            Err(())
        }
    }

    pub fn parse_bytes(bytes_str: &str) -> Result<u64, ()> {
        let trimmed = bytes_str.trim();
        let parts: Vec<&str> = trimmed.split_whitespace().collect();

        if parts.len() != 2 {
            return Err(());
        }

        let value_str = parts[0];
        let unit = parts[1];

        let value = value_str.parse::<f64>().map_err(|_| ())?;

        match unit {
            "B" => Ok(value as u64),
            "KB" => Ok((value * 1024.0) as u64),
            "MB" => Ok((value * 1024.0 * 1024.0) as u64),
            "GB" => Ok((value * 1024.0 * 1024.0 * 1024.0) as u64),
            "TB" => Ok((value * 1024.0 * 1024.0 * 1024.0 * 1024.0) as u64),
            _ => Err(()),
        }
    }

    fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        format!("{:.3} {}", size, UNITS[unit_index])
    }

    pub fn parse_duration(duration_str: &str) -> Result<Duration, ()> {
        let mut total_nanos: u128 = 0;

        let re = regex::Regex::new(r"(\d+)([hms]|ms|us|ns)").map_err(|_| ())?;
        for cap in re.captures_iter(duration_str) {
            let value: u128 = cap.get(1).ok_or(())?.as_str().parse().map_err(|_| ())?;
            let unit = cap.get(2).ok_or(())?.as_str();

            let multiplier = match unit {
                "ns" => 1,
                "us" => 1_000,
                "ms" => 1_000_000,
                "s" => 1_000_000_000,
                "m" => 60_000_000_000,
                "h" => 3_600_000_000_000,
                _ => return Err(()),
            };

            total_nanos += value * multiplier;
        }

        if total_nanos > u64::MAX as u128 {
            return Err(());
        }

        Ok(Duration::from_nanos(total_nanos as u64))
    }

    fn calculate_time_percentage(operator_time: Duration, total_time_str: &str) -> f64 {
        if let Ok(total_duration) = Self::parse_total_time(total_time_str) {
            if total_duration > 0.0 {
                (operator_time.as_secs_f64() / total_duration) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
}
