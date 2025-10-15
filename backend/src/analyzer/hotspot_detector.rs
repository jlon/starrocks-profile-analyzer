use crate::models::*;
use std::collections::HashMap;

pub struct HotSpotDetector;

impl HotSpotDetector {
    pub fn analyze(profile: &Profile) -> Vec<HotSpot> {
        let mut hotspots = Vec::new();

        // 分析整体执行时间
        if let Ok(total_time_seconds) = Self::parse_duration(&profile.summary.total_time) {
            if total_time_seconds > 3600.0 { // 超过1小时
                hotspots.push(HotSpot {
                    node_path: "Query".to_string(),
                    severity: HotSeverity::Severe,
                    issue_type: "LongRunning".to_string(),
                    description: format!("查询总执行时间过长: {}s", total_time_seconds),
                    suggestions: vec![
                        "检查是否存在数据倾斜".to_string(),
                        "考虑优化查询计划".to_string(),
                        "查看是否存在硬件瓶颈".to_string(),
                    ],
                });
            }
        }

        // 优先分析execution_tree中的操作符 (如果存在)
        if let Some(execution_tree) = &profile.execution_tree {
            println!("🔍 Analyzing execution tree with {} nodes", execution_tree.nodes.len());
            for node in &execution_tree.nodes {
                hotspots.extend(Self::analyze_execution_tree_node(node));
            }
        } else {
            // 回退到分析Fragment结构
            println!("⚠️  No execution tree found, analyzing fragments");
            for fragment in &profile.fragments {
                hotspots.extend(Self::analyze_fragment(fragment));
            }
        }

        // 按严重度排序
        hotspots.sort_by(|a, b| {
            let severity_order = |severity: &HotSeverity| match severity {
                HotSeverity::Normal => 0,
                HotSeverity::Mild => 1,
                HotSeverity::Moderate => 2,
                HotSeverity::Severe => 3,
                HotSeverity::Critical => 4,
                HotSeverity::High => 3, // High 和 Severe 同级
            };
            severity_order(&b.severity).cmp(&severity_order(&a.severity))
        });

        hotspots
    }

    fn analyze_fragment(fragment: &Fragment) -> Vec<HotSpot> {
        let mut hotspots = Vec::new();

        for pipeline in &fragment.pipelines {
            hotspots.extend(Self::analyze_pipeline(fragment.id.as_str(), pipeline));
        }

        hotspots
    }

    fn analyze_pipeline(fragment_id: &str, pipeline: &Pipeline) -> Vec<HotSpot> {
        let mut hotspots = Vec::new();

        for operator in &pipeline.operators {
            hotspots.extend(Self::analyze_operator(fragment_id, &pipeline.id, operator));
        }

        hotspots
    }

    fn analyze_operator(fragment_id: &str, pipeline_id: &str, operator: &Operator) -> Vec<HotSpot> {
        let mut hotspots = Vec::new();
        let node_path = format!("Fragment{}.Pipeline{}.{}", fragment_id, pipeline_id, operator.name);

        // 检查OperatorTotalTime
        if let Some(time_str) = operator.common_metrics.get("OperatorTotalTime") {
            if let Ok(time_seconds) = Self::parse_duration(time_str) {
                if time_seconds > 300.0 { // 超过5分钟
                    hotspots.push(HotSpot {
                        node_path: node_path.clone(),
                        severity: HotSeverity::Severe,
                        issue_type: "HighTimeCost".to_string(),
                        description: format!("算子 {} 耗时过高: {}s", operator.name, time_seconds),
                        suggestions: vec![
                            "检查该算子是否处理数据量过大".to_string(),
                            "考虑是否需要添加索引".to_string(),
                            "查看是否遇到数据倾斜".to_string(),
                        ],
                    });
                }
            }
        }

        // 检查内存使用
        if let Some(mem_bytes) = Self::parse_bytes(operator.common_metrics.get("MemoryUsage")) {
            if mem_bytes > 1024 * 1024 * 1024 { // 超过1GB
                hotspots.push(HotSpot {
                    node_path: node_path.clone(),
                    severity: HotSeverity::Moderate,
                    issue_type: "HighMemoryUsage".to_string(),
                    description: format!("算子 {} 内存使用过高: {}", operator.name, Self::format_bytes(mem_bytes)),
                    suggestions: vec![
                        "检查是否内存泄漏".to_string(),
                        "考虑调整内存配置参数".to_string(),
                        "优化数据结构使用".to_string(),
                    ],
                });
            }
        }

        // 检查输出数据量异常
        if let Some(bytes_str) = operator.common_metrics.get("OutputChunkBytes") {
            if let Ok(bytes) = Self::parse_bytes_from_starrock(bytes_str) {
                if bytes > 10 * 1024 * 1024 * 1024 { // 超过10GB
                    hotspots.push(HotSpot {
                        node_path: node_path.clone(),
                        severity: HotSeverity::Moderate,
                        issue_type: "LargeDataOutput".to_string(),
                        description: format!("算子 {} 输出数据量过大: {}", operator.name, Self::format_bytes(bytes)),
                        suggestions: vec![
                            "检查是否存在不必要的列选择".to_string(),
                            "考虑添加过滤条件".to_string(),
                            "查看数据分布是否均匀".to_string(),
                        ],
                    });
                }
            }
        }

        // 根据操作符类型进行专门分析
        match operator.name.as_str() {
            "CONNECTOR_SCAN" => {
                println!("🚨 Found CONNECTOR_SCAN! Analyzing with metrics count: {}", operator.unique_metrics.len());
                hotspots.extend(Self::analyze_connector_scan(fragment_id, pipeline_id, operator));
            }
            "OLAP_SCAN" => {
                hotspots.extend(Self::analyze_olap_scan(fragment_id, pipeline_id, operator));
            }
            "HASH_JOIN" => {
                hotspots.extend(Self::analyze_join_operator(fragment_id, pipeline_id, operator));
            }
            "AGGREGATE" => {
                hotspots.extend(Self::analyze_aggregate_operator(fragment_id, pipeline_id, operator));
            }
            _ => {
                // 通用操作符分析
                println!("📝 Unknown operator type: {}", operator.name);
            }
        }

        hotspots
    }

    fn parse_duration(duration_str: &str) -> Result<f64, ()> {
        // 解析StarRocks格式的持续时间，如 "1h30m", "5s499ms", "0ns"
        if duration_str.contains("h") {
            let hours: f64 = duration_str.split("h").next().unwrap_or("0").parse().unwrap_or(0.0);
            let minutes: f64 = duration_str.split("h").nth(1).unwrap_or("0").split("m").next().unwrap_or("0").parse().unwrap_or(0.0);
            Ok(hours * 3600.0 + minutes * 60.0)
        } else if duration_str.contains("m") {
            let minutes: f64 = duration_str.split("m").next().unwrap_or("0").parse().unwrap_or(0.0);
            Ok(minutes * 60.0)
        } else if duration_str.contains("s") {
            let seconds: f64 = duration_str.split("s").next().unwrap_or("0").parse().unwrap_or(0.0);
            Ok(seconds)
        } else if duration_str.contains("ms") {
            let ms: f64 = duration_str.split("ms").next().unwrap_or("0").parse().unwrap_or(0.0);
            Ok(ms / 1000.0)
        } else if duration_str.contains("us") || duration_str.contains("μs") {
            let us: f64 = duration_str.split("us").next().unwrap_or("0").replace("μ", "").parse().unwrap_or(0.0);
            Ok(us / 1_000_000.0)
        } else if duration_str.contains("ns") {
            let ns: f64 = duration_str.split("ns").next().unwrap_or("0").parse().unwrap_or(0.0);
            Ok(ns / 1_000_000_000.0)
        } else {
            Err(())
        }
    }

    fn parse_bytes(bytes_str: Option<&String>) -> Option<u64> {
        bytes_str.and_then(|s| Self::parse_bytes_from_starrock(s).ok())
    }

    fn parse_bytes_from_starrock(bytes_str: &str) -> Result<u64, ()> {
        // 解析StarRocks格式的字节数，如 "2.174K (2174)", "1.463 KB", "18.604 MB"
        let clean_str = bytes_str
            .split_whitespace()
            .next()
            .unwrap_or(bytes_str)
            .replace(",", "");

        if clean_str.contains("GB") {
            let gb: f64 = clean_str.replace("GB", "").parse().unwrap_or(0.0);
            Ok((gb * 1024.0 * 1024.0 * 1024.0) as u64)
        } else if clean_str.contains("MB") {
            let mb: f64 = clean_str.replace("MB", "").parse().unwrap_or(0.0);
            Ok((mb * 1024.0 * 1024.0) as u64)
        } else if clean_str.contains("KB") || clean_str.contains("K") {
            let kb: f64 = clean_str.replace("KB", "").replace("K", "").parse().unwrap_or(0.0);
            Ok((kb * 1024.0) as u64)
        } else if clean_str.contains("B") {
            let bytes: f64 = clean_str.replace("B", "").parse().unwrap_or(0.0);
            Ok(bytes as u64)
        } else {
            Err(())
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

        format!("{:.2} {}", size, UNITS[unit_index])
    }

    /// 分析CONNECTOR_SCAN操作符的热点
    fn analyze_connector_scan(fragment_id: &str, pipeline_id: &str, operator: &Operator) -> Vec<HotSpot> {
        let mut hotspots = Vec::new();
        let node_path = format!("Fragment{}.Pipeline{}.{}", fragment_id, pipeline_id, operator.name);

        // 1. CreateSegmentIter时间过长 (核心瓶颈：Segment迭代器初始化耗时)
        if let Some(create_iter_time_str) = operator.unique_metrics.get("CreateSegmentIter") {
            if let Ok(create_seconds) = Self::parse_duration(create_iter_time_str) {
                if create_seconds > 1800.0 { // 超过30分钟
                    hotspots.push(HotSpot {
                        node_path: node_path.clone(),
                        severity: HotSeverity::Critical,
                        issue_type: "fragmented_rowsets".to_string(),
                        description: format!("Segment迭代器初始化耗时过长: {}s - 表碎片过多导致", create_seconds),
                        suggestions: vec![
                            "触发手动compaction (ALTER TABLE ... COMPACT)".to_string(),
                            "检查compaction配置 (cumulative_compaction_num_deltas)".to_string(),
                            "重做表结构减少小文件数量".to_string(),
                            "定期监控table元数据大小".to_string(),
                        ],
                    });
                } else if create_seconds > 300.0 { // 超过5分钟
                    hotspots.push(HotSpot {
                        node_path: node_path.clone(),
                        severity: HotSeverity::Severe,
                        issue_type: "fragmented_rowsets".to_string(),
                        description: format!("Segment迭代器初始化耗时较长: {}s - 检查表compaction状态", create_seconds),
                        suggestions: vec![
                            "检查表compaction状态和参数".to_string(),
                            "考虑调整compaction频率".to_string(),
                            "监控Segment数量变化趋势".to_string(),
                        ],
                    });
                }
            }
        }

        // 2. SegmentsReadCount过多 (碎片化检测)
        if let Some(segment_count_str) = operator.unique_metrics.get("SegmentsReadCount") {
            if let Ok(segment_count) = segment_count_str.parse::<u64>() {
                if segment_count > 100000 { // 超过10万个Segment
                    hotspots.push(HotSpot {
                        node_path: node_path.clone(),
                        severity: HotSeverity::Critical,
                        issue_type: "fragmented_rowsets".to_string(),
                        description: format!("太多元信息段需要读取: {} 个 - 严重表碎片化", segment_count),
                        suggestions: vec![
                            "紧急执行表compaction操作".to_string(),
                            "检查导入策略减少小文件生成".to_string(),
                            "调整compaction触发阈值".to_string(),
                            "考虑分区重构减少热点分区的Segment数量".to_string(),
                        ],
                    });
                } else if segment_count > 50000 { // 超过5万个Segment
                    hotspots.push(HotSpot {
                        node_path: node_path.clone(),
                        severity: HotSeverity::Severe,
                        issue_type: "fragmented_rowsets".to_string(),
                        description: format!("大量元信息段需要读取: {} 个 - 表碎片化严重", segment_count),
                        suggestions: vec![
                            "优先执行compaction操作".to_string(),
                            "优化导入参数减少Segment分片".to_string(),
                            "考虑调整cumulative_compaction_num_deltas参数".to_string(),
                        ],
                    });
                } else if segment_count > 10000 { // 超过1万个Segment
                    hotspots.push(HotSpot {
                        node_path: node_path.clone(),
                        severity: HotSeverity::Moderate,
                        issue_type: "fragmented_rowsets".to_string(),
                        description: format!("较多元信息段需要读取: {} 个 - 注意表碎片化", segment_count),
                        suggestions: vec![
                            "规划执行compaction维护任务".to_string(),
                            "定期监控table的Segment数量".to_string(),
                        ],
                    });
                }
            }
        }

        // 3. 远程存储瓶颈检测 (LakeDataSource分析)
        if let Some(remote_io_time_str) = operator.unique_metrics.get("IOTimeRemote") {
            if let Ok(remote_io_seconds) = Self::parse_duration(remote_io_time_str) {
                if let Some(total_scan_time_str) = operator.common_metrics.get("ScanTime") {
                    if let Ok(total_scan_seconds) = Self::parse_duration(total_scan_time_str) {
                        if remote_io_seconds > total_scan_seconds * 0.8 { // 远程IO占扫描时间的80%以上
                            hotspots.push(HotSpot {
                                node_path: node_path.clone(),
                                severity: HotSeverity::Severe,
                                issue_type: "cold_storage_overhead".to_string(),
                                description: format!("远程存储IO耗时占比过高: {:.1}% - 网络成为主要瓶颈",
                                                (remote_io_seconds / total_scan_seconds * 100.0)),
                                suggestions: vec![
                                    "加速网络链路带宽和延迟优化".to_string(),
                                    "启用数据预热策略减少冷读".to_string(),
                                    "考虑将热点数据迁移到本地存储".to_string(),
                                    "存储系统IOPS和带宽性能评估".to_string(),
                                ],
                            });
                        }
                    }
                }
            }
        }

        // 4. 扫描时间过长 (综合时间检测)
        if let Some(scan_time_str) = operator.common_metrics.get("ScanTime") {
            if let Ok(scan_seconds) = Self::parse_duration(scan_time_str) {
                if scan_seconds > 3600.0 { // 超过1小时
                    hotspots.push(HotSpot {
                        node_path: node_path.clone(),
                        severity: HotSeverity::Critical,
                        issue_type: "excessive_scan_time".to_string(),
                        description: format!("扫描操作耗时极长: {}s - 严重性能问题", scan_seconds),
                        suggestions: vec![
                            "紧急优化查询条件缩小扫描范围".to_string(),
                            "检查表索引完整性和有效性".to_string(),
                            "评估数据分片策略合理性".to_string(),
                            "考虑分区裁剪和谓词下推优化".to_string(),
                        ],
                    });
                } else if scan_seconds > 1800.0 { // 超过30分钟
                    hotspots.push(HotSpot {
                        node_path: node_path.clone(),
                        severity: HotSeverity::Severe,
                        issue_type: "high_scan_time".to_string(),
                        description: format!("扫描操作耗时过长: {}s", scan_seconds),
                        suggestions: vec![
                            "优化查询WHERE条件".to_string(),
                            "添加适当的索引".to_string(),
                            "检查分区键选择".to_string(),
                        ],
                    });
                }
            }
        }

        // 5. I/O时间过长 (详细IO分析)
        if let Some(io_time_str) = operator.unique_metrics.get("IOTime") {
            if let Ok(io_seconds) = Self::parse_duration(io_time_str) {
                if io_seconds > 1200.0 { // 超过20分钟
                    hotspots.push(HotSpot {
                        node_path: node_path.clone(),
                        severity: HotSeverity::Severe,
                        issue_type: "high_io_time".to_string(),
                        description: format!("I/O操作耗时过长: {}s", io_seconds),
                        suggestions: vec![
                            "检查存储系统性能指标".to_string(),
                            "考虑调整I/O相关参数".to_string(),
                            "查看数据是否本地化".to_string(),
                        ],
                    });
                }
            }
        }

        // 6. 远程读取完全依赖 (100%远程读取)
        if let Some(remote_count_str) = operator.unique_metrics.get("IOCountRemote") {
            if let Ok(remote_count) = remote_count_str.parse::<u64>() {
                if let Some(local_count_str) = operator.unique_metrics.get("IOCountLocalDisk") {
                    if let Ok(local_count) = local_count_str.parse::<u64>() {
                        if remote_count > 0 && local_count == 0 {
                            // 100% 远程读取
                            hotspots.push(HotSpot {
                                node_path: node_path.clone(),
                                severity: HotSeverity::High,
                                issue_type: "cold_storage".to_string(),
                                description: "所有数据从远程存储读取，未使用本地缓存".to_string(),
                                suggestions: vec![
                                    "启用存储缓存配置 (storage_root_path 指定SSD路径)".to_string(),
                                    "提升远程缓存容量 (remote_cache_capacity 参数)".to_string(),
                                    "检查存储系统IOPS是否充足".to_string(),
                                    "优化数据存储层级策略".to_string(),
                                ],
                            });
                        } else if remote_count > local_count * 10 {
                            // 远程读取远超本地
                            hotspots.push(HotSpot {
                                node_path: node_path.clone(),
                                severity: HotSeverity::Moderate,
                                issue_type: "high_remote_io_ratio".to_string(),
                                description: format!("远程I/O过多: 远程={}, 本地={}", remote_count, local_count),
                                suggestions: vec![
                                    "考虑数据预热减少冷读".to_string(),
                                    "优化数据分布策略".to_string(),
                                    "增加本地缓存容量".to_string(),
                                ],
                            });
                        }
                    }
                }
            }
        }

        // 7. 无谓词过滤但读取大量数据
        let has_effective_filtering = operator.unique_metrics.get("ShortKeyFilterRows")
            .and_then(|s| s.parse::<u64>().ok())
            .map(|rows| rows > 0)
            .unwrap_or(false);

        if !has_effective_filtering {
            if let Some(raw_rows_str) = operator.unique_metrics.get("RawRowsRead") {
                if let Ok(raw_rows) = raw_rows_str.parse::<u64>() {
                    if raw_rows > 100000 { // 读取大量原始数据但无过滤
                        hotspots.push(HotSpot {
                            node_path: node_path.clone(),
                            severity: HotSeverity::High,
                            issue_type: "missing_predicate_pushdown".to_string(),
                            description: format!("读取海量数据但无有效谓词过滤: {} 行", raw_rows),
                            suggestions: vec![
                                "添加WHERE条件进行数据筛选".to_string(),
                                "创建索引支持快速定位".to_string(),
                                "使用分区键进行数据裁剪".to_string(),
                                "创建物化视图以便谓词下推".to_string(),
                            ],
                        });
                    } else if raw_rows > 10000 {
                        hotspots.push(HotSpot {
                            node_path: node_path.clone(),
                            severity: HotSeverity::Moderate,
                            issue_type: "missing_predicate_pushdown".to_string(),
                            description: format!("读取大量数据但未使用谓词过滤: {} 行", raw_rows),
                            suggestions: vec![
                                "考虑添加过滤条件".to_string(),
                                "检查查询是否需要全表扫描".to_string(),
                            ],
                        });
                    }
                }
            }
        }

        // 8. 线程池资源不足 (扫描任务队列积压)
        if let Some(pending_tasks_str) = operator.unique_metrics.get("PeakScanTaskQueueSize") {
            if let Ok(queue_size) = pending_tasks_str.parse::<u64>() {
                if queue_size > 50 { // 队列积压严重
                    hotspots.push(HotSpot {
                        node_path: node_path.clone(),
                        severity: HotSeverity::Severe,
                        issue_type: "thread_pool_starvation".to_string(),
                        description: format!("扫描任务队列严重积压: {} 个任务等待 - I/O线程不足", queue_size),
                        suggestions: vec![
                            "增加BE I/O线程池大小 (max_io_threads_per_disk)".to_string(),
                            "增加扫描线程池大小 (num_io_threads_backlog)".to_string(),
                            "减少并发查询负载".to_string(),
                            "检查I/O子系统是否过载".to_string(),
                        ],
                    });
                } else if queue_size > 20 {
                    hotspots.push(HotSpot {
                        node_path: node_path.clone(),
                        severity: HotSeverity::Moderate,
                        issue_type: "thread_pool_starvation".to_string(),
                        description: format!("扫描任务队列积压: {} 个任务等待", queue_size),
                        suggestions: vec![
                            "考虑增加I/O线程池大小".to_string(),
                            "监控并发查询压力".to_string(),
                        ],
                    });
                }
            }
        }

        // 9. 并行度过低检测 (来自Fragment/Pipeline级别)
        // 这个需要在更上层做，但这里可以检查扫描相关的并行度指标
        if let Some(parallelism_str) = operator.common_metrics.get("DegreeOfParallelism") {
            if let Ok(parallelism) = parallelism_str.parse::<u64>() {
                if parallelism == 1 && operator.common_metrics.get("ScanTime").is_some() {
                    // 并行度为1但有扫描操作，可能是问题
                    hotspots.push(HotSpot {
                        node_path: node_path.clone(),
                        severity: HotSeverity::Moderate,
                        issue_type: "insufficient_parallelism".to_string(),
                        description: "查询并行度过低，仅使用1个线程执行扫描".to_string(),
                        suggestions: vec![
                            "增加parallel_fragment_exec_instance_num参数".to_string(),
                            "检查pipeline_dop设置".to_string(),
                            "确认并行执行计划的正确性".to_string(),
                        ],
                    });
                }
            }
        }

        hotspots
    }

    /// 分析OLAP_SCAN操作符的热点
    fn analyze_olap_scan(fragment_id: &str, pipeline_id: &str, operator: &Operator) -> Vec<HotSpot> {
        let mut hotspots = Vec::new();
        let _node_path = format!("Fragment{}.Pipeline{}.{}", fragment_id, pipeline_id, operator.name);

        // OLAP_SCAN专用的检查逻辑
        // 检查扫描时间和数据量等指标
        // TODO: 实现OLAP_SCAN特定的热点检测规则

        hotspots
    }

    /// 分析JOIN操作符的热点
    fn analyze_join_operator(fragment_id: &str, pipeline_id: &str, operator: &Operator) -> Vec<HotSpot> {
        let mut hotspots = Vec::new();
        let node_path = format!("Fragment{}.Pipeline{}.{}", fragment_id, pipeline_id, operator.name);

        // 检查构建侧和探测侧的数据量比例
        let build_rows = operator.unique_metrics.get("BuildRows")
            .and_then(|s| s.parse::<u64>().ok());
        let probe_rows = operator.unique_metrics.get("ProbeRows")
            .and_then(|s| s.parse::<u64>().ok());

        if let (Some(build), Some(probe)) = (build_rows, probe_rows) {
            if build > probe * 100 || probe > build * 100 {
                // 数据倾斜严重
                hotspots.push(HotSpot {
                    node_path: node_path.clone(),
                    severity: HotSeverity::High,
                    issue_type: "data_skew".to_string(),
                    description: format!("JOIN数据倾斜严重: 构建侧={}, 探测侧={}", build, probe),
                    suggestions: vec![
                        "重新选择hash分桶键 (选择分布均匀的列)".to_string(),
                        "增加分桶数量以分散数据".to_string(),
                        "使用倾斜数据处理 (enable_skew_optimization)".to_string(),
                        "检查统计信息是否最新".to_string(),
                    ],
                });
            }

            // 检查内存使用是否合理
            if let Some(mem_usage) = Self::parse_bytes(operator.common_metrics.get("MemoryUsage")) {
                let expected_mem = (build + probe) * 100; // 粗略估计每行100字节
                if mem_usage > expected_mem * 2 {
                    hotspots.push(HotSpot {
                        node_path: node_path.clone(),
                        severity: HotSeverity::Moderate,
                        issue_type: "HighJoinMemory".to_string(),
                        description: format!("JOIN内存使用异常: {} (预期约为{})", Self::format_bytes(mem_usage), Self::format_bytes(expected_mem)),
                        suggestions: vec![
                            "考虑使用broadcast join代替shuffle join".to_string(),
                            "检查是否可以减少JOIN列".to_string(),
                            "优化查询逻辑减少JOIN规模".to_string(),
                        ],
                    });
                }
            }
        }

        hotspots
    }

    /// 分析AGGREGATE操作符的热点
    fn analyze_aggregate_operator(fragment_id: &str, pipeline_id: &str, operator: &Operator) -> Vec<HotSpot> {
        let mut hotspots = Vec::new();
        let node_path = format!("Fragment{}.Pipeline{}.{}", fragment_id, pipeline_id, operator.name);

        // 检查聚合模式
        if let Some(agg_mode) = operator.unique_metrics.get("AggMode") {
            if agg_mode == "two_phase" {
                // 使用了两阶段聚合，检查是否有优化空间
                if let Some(chunk_by_chunk) = operator.unique_metrics.get("ChunkByChunk") {
                    if chunk_by_chunk == "false" {
                        hotspots.push(HotSpot {
                            node_path: node_path.clone(),
                            severity: HotSeverity::Mild,
                            issue_type: "suboptimal_aggregation".to_string(),
                            description: "聚合未使用chunk-by-chunk模式，可能影响性能".to_string(),
                            suggestions: vec![
                                "检查是否可以使用streaming聚合".to_string(),
                                "考虑调整聚合参数".to_string(),
                            ],
                        });
                    }
                }
            }
        }

        // 检查预聚合效果
        let input_rows = operator.unique_metrics.get("InputRows")
            .and_then(|s| s.parse::<u64>().ok());
        let output_rows = operator.common_metrics.get("PushRowNum")
            .and_then(|s| s.parse::<u64>().ok());

        if let (Some(input), Some(output)) = (input_rows, output_rows) {
            if input > 0 && output > 0 {
                let agg_ratio = input as f64 / output as f64;
                if agg_ratio < 2.0 {
                    // 聚合效果差
                    hotspots.push(HotSpot {
                        node_path: node_path.clone(),
                        severity: HotSeverity::Mild,
                        issue_type: "LowAggregationRatio".to_string(),
                        description: format!("聚合效果不佳: 输入{}行, 输出{}行 (聚合比: {:.2})",
                                           input, output, agg_ratio),
                        suggestions: vec![
                            "检查GROUP BY列的选择是否合适".to_string(),
                            "考虑使用预聚合优化".to_string(),
                            "检查数据分布是否造成低效聚合".to_string(),
                        ],
                    });
                }
            }
        }

        hotspots
    }

    /// 分析ExecutionTree中的单个节点
    fn analyze_execution_tree_node(node: &ExecutionTreeNode) -> Vec<HotSpot> {
        let mut hotspots = Vec::new();
        let node_path = node.id.clone(); // 使用节点的id作为路径

        // 将ExecutionTreeNode的metrics转换为临时Operator结构，便于复用现有逻辑
        let temp_operator = Operator {
            name: node.operator_name.clone(),
            operator_id: Some(node.id.to_string()),
            plan_node_id: Some(node.plan_node_id.unwrap_or(-1).to_string()),
            common_metrics: Self::convert_metrics_to_map(&node.metrics),
            unique_metrics: Self::convert_specialized_metrics_to_map(&node.metrics.specialized),
            children: vec![], // 留空
        };

        // 检查节点是否是CONNECTOR_SCAN类型
        if node.node_type == NodeType::ConnectorScan {
            println!("🎯 Found CONNECTOR_SCAN in execution tree! Analyzing...");
            // 对于ExecutionTreeNode，我们需要传递临时fragment_id和pipeline_id
            hotspots.extend(Self::analyze_connector_scan_from_node(&node, &temp_operator));
        }

        // 可以扩展其他节点类型的分析

        hotspots
    }

    /// 从ExecutionTreeNode分析CONNECTOR_SCAN
    fn analyze_connector_scan_from_node(node: &ExecutionTreeNode, operator: &Operator) -> Vec<HotSpot> {
        let mut hotspots = Vec::new();
        let node_path = node.id.clone();

        // 调试：检查指标
        println!("🔍 Node {} has unique metrics count: {}", node.operator_name, operator.unique_metrics.len());
        for (key, value) in &operator.unique_metrics {
            println!("  📊 {}: {}", key, value);
        }

        // 1. CreateSegmentIter时间过长 (核心瓶颈：Segment迭代器初始化耗时)
        if let Some(create_iter_time_str) = operator.unique_metrics.get("CreateSegmentIter") {
            println!("✅ Found CreateSegmentIter [ExecutionTree]: {}", create_iter_time_str);
            if let Ok(create_seconds) = Self::parse_duration(create_iter_time_str) {
                println!("✅ Parsed CreateSegmentIter duration [ExecutionTree]: {}s", create_seconds);
                if create_seconds > 1800.0 { // 超过30分钟
                    println!("🚨 Creating CRITICAL hotspot for CreateSegmentIter [ExecutionTree]");
                    hotspots.push(HotSpot {
                        node_path: node_path.clone(),
                        severity: HotSeverity::Critical,
                        issue_type: "fragmented_rowsets".to_string(),
                        description: format!("Segment迭代器初始化耗时过长: {}s - 表碎片过多导致 (ExecutionTree)", create_seconds),
                        suggestions: vec![
                            "触发手动compaction (ALTER TABLE ... COMPACT)".to_string(),
                            "检查compaction配置 (cumulative_compaction_num_deltas)".to_string(),
                            "重做表结构减少小文件数量".to_string(),
                            "定期监控table元数据大小".to_string(),
                        ],
                    });
                } else if create_seconds > 300.0 { // 超过5分钟
                    println!("🚨 Creating SEVERE hotspot for CreateSegmentIter [ExecutionTree]");
                    hotspots.push(HotSpot {
                        node_path: node_path.clone(),
                        severity: HotSeverity::Severe,
                        issue_type: "fragmented_rowsets".to_string(),
                        description: format!("Segment迭代器初始化耗时较长: {}s - 检查表compaction状态 (ExecutionTree)", create_seconds),
                        suggestions: vec![
                            "检查表compaction状态和参数".to_string(),
                            "考虑调整compaction频率".to_string(),
                            "监控Segment数量变化趋势".to_string(),
                        ],
                    });
                }
            }
        }

        // 2. SegmentsReadCount过多 (碎片化检测)
        if let Some(segment_count_str) = operator.unique_metrics.get("SegmentsReadCount") {
            if let Ok(segment_count) = segment_count_str.parse::<u64>() {
                if segment_count > 10000 { // 超过1万个Segment (放宽阈值用于测试)
                    println!("🚨 Creating SEGMENT COUNT hotspot [ExecutionTree]: {}", segment_count);
                    hotspots.push(HotSpot {
                        node_path: node_path.clone(),
                        severity: HotSeverity::Critical,
                        issue_type: "fragmented_rowsets".to_string(),
                        description: format!("太多元信息段需要读取: {} 个 - 严重表碎片化 (ExecutionTree)", segment_count),
                        suggestions: vec![
                            "紧急执行表compaction操作".to_string(),
                            "检查导入策略减少小文件生成".to_string(),
                            "调整compaction触发阈值".to_string(),
                        ],
                    });
                }
            }
        }

        hotspots
    }

    /// 转换OperatorMetrics为HashMap (简化的实现)
    fn convert_metrics_to_map(metrics: &OperatorMetrics) -> HashMap<String, String> {
        let mut map = HashMap::new();

        if let Some(time) = metrics.operator_total_time {
            map.insert("OperatorTotalTime".to_string(), format!("{}ms", time.as_millis()));
        }
        if let Some(num) = metrics.push_chunk_num {
            map.insert("PushChunkNum".to_string(), num.to_string());
        }
        if let Some(num) = metrics.push_row_num {
            map.insert("PushRowNum".to_string(), num.to_string());
        }
        if let Some(num) = metrics.pull_chunk_num {
            map.insert("PullChunkNum".to_string(), num.to_string());
        }
        if let Some(num) = metrics.pull_row_num {
            map.insert("PullRowNum".to_string(), num.to_string());
        }
        if let Some(time) = metrics.push_total_time {
            map.insert("PushTotalTime".to_string(), format!("{}ms", time.as_millis()));
        }
        if let Some(time) = metrics.pull_total_time {
            map.insert("PullTotalTime".to_string(), format!("{}ms", time.as_millis()));
        }
        if let Some(bytes) = metrics.memory_usage {
            map.insert("MemoryUsage".to_string(), Self::format_bytes(bytes));
        }
        if let Some(bytes) = metrics.output_chunk_bytes {
            map.insert("OutputChunkBytes".to_string(), Self::format_bytes(bytes));
        }

        map
    }

    /// 转换OperatorSpecializedMetrics为HashMap (简化的实现)
    fn convert_specialized_metrics_to_map(specialized: &OperatorSpecializedMetrics) -> HashMap<String, String> {
        let mut map = HashMap::new();

        // 这里需要根据实际的OperatorSpecializedMetrics结构来转换
        // 目前这是一个简化的实现，可能需要根据实际数据结构调整

        match specialized {
            OperatorSpecializedMetrics::ConnectorScan(scan_metrics) => {
                // 对于扫描操作符，提取相关指标
                if let Some(time) = scan_metrics.io_task_exec_time {
                    map.insert("IOTaskExecTime".to_string(), format!("{}ms", time.as_millis()));
                }
                if let Some(counts) = scan_metrics.segment_read_count {
                    map.insert("SegmentsReadCount".to_string(), counts.to_string());
                }
                if let Some(time) = scan_metrics.segment_init {
                    map.insert("SegmentInit".to_string(), format!("{}ms", time.as_millis()));
                }
                if let Some(time) = scan_metrics.segment_read {
                    map.insert("SegmentRead".to_string(), format!("{}ms", time.as_millis()));
                }
                if let Some(time) = scan_metrics.io_time {
                    map.insert("IOTime".to_string(), format!("{}ms", time.as_millis()));
                }
                if let Some(time) = scan_metrics.scan_time {
                    map.insert("ScanTime".to_string(), format!("{}ms", time.as_millis()));
                }
                if let Some(bytes) = scan_metrics.bytes_read {
                    map.insert("BytesRead".to_string(), Self::format_bytes(bytes));
                }
                if let Some(rows) = scan_metrics.rows_read {
                    map.insert("RowsRead".to_string(), rows.to_string());
                }
                if let Some(local) = scan_metrics.io_count_local_disk {
                    map.insert("IOCountLocalDisk".to_string(), local.to_string());
                }
                if let Some(remote) = scan_metrics.io_count_remote {
                    map.insert("IOCountRemote".to_string(), remote.to_string());
                }
                if let Some(remote_time) = scan_metrics.io_time_remote {
                    map.insert("IOTimeRemote".to_string(), format!("{}ms", remote_time.as_millis()));
                }
                if let Some(local_bytes) = scan_metrics.compressed_bytes_read_local_disk {
                    map.insert("CompressedBytesReadLocal".to_string(), Self::format_bytes(local_bytes));
                }
                if let Some(remote_bytes) = scan_metrics.compressed_bytes_read_remote {
                    map.insert("CompressedBytesReadRemote".to_string(), Self::format_bytes(remote_bytes));
                }
            }
            _ => {
                // 其他类型的操作符暂时不处理
            }
        }

        map
    }
}
