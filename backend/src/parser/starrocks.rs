use crate::models::*;
use std::collections::HashMap;

pub struct StarRocksProfileParser;

impl StarRocksProfileParser {
    pub fn parse(input: &str) -> Result<Profile, String> {
        let mut profile = Profile {
            summary: ProfileSummary {
                query_id: String::new(),
                start_time: String::new(),
                end_time: String::new(),
                total_time: String::new(),
                query_state: String::new(),
                starrocks_version: String::new(),
                sql_statement: String::new(),
                variables: HashMap::new(),
            },
            planner: PlannerInfo {
                details: HashMap::new(),
            },
            execution: ExecutionInfo {
                topology: String::new(),
                metrics: HashMap::new(),
            },
            fragments: Vec::new(),
            execution_tree: None,
        };

        // 按行分割输入
        let lines: Vec<&str> = input.lines().collect();
        let mut current_section = Section::None;
        let mut current_fragment: Option<Fragment> = None;
        let _current_pipeline: Option<Pipeline> = None;
        let _current_operator: Option<Operator> = None;

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // 识别主要部分
            if trimmed.starts_with("Query:") {
                current_section = Section::Query;
                continue;
            } else if trimmed.starts_with("Planner:") {
                current_section = Section::Planner;
                continue;
            } else if trimmed.starts_with("Execution:") {
                current_section = Section::Execution;
                continue;
            } else if trimmed.starts_with("Fragment ") {
                // 保存之前的fragment
                if let Some(fragment) = current_fragment.take() {
                    profile.fragments.push(fragment);
                }
                
                // 创建新的fragment
                let id = Self::extract_fragment_id(trimmed);
                current_fragment = Some(Fragment {
                    id,
                    backend_addresses: Vec::new(),
                    instance_ids: Vec::new(),
                    pipelines: Vec::new(),
                });
                current_section = Section::Fragment;
                continue;
            }

            // 根据当前部分处理行
            match current_section {
                Section::Query => {
                    Self::parse_query_line(trimmed, &mut profile);
                }
                Section::Planner => {
                    Self::parse_planner_line(trimmed, &mut profile);
                }
                Section::Execution => {
                    Self::parse_execution_line(trimmed, &mut profile);
                }
                Section::Fragment => {
                    if let Some(ref mut fragment) = current_fragment {
                        Self::parse_fragment_line(trimmed, fragment);
                    }
                }
                Section::None => {}
            }
        }

        // 保存最后一个fragment
        if let Some(fragment) = current_fragment.take() {
            profile.fragments.push(fragment);
        }

        Ok(profile)
    }

    fn extract_fragment_id(line: &str) -> String {
        // 提取 "Fragment 0:" 中的 "0"
        line.split_whitespace()
            .nth(1)
            .unwrap_or("")
            .trim_end_matches(':')
            .to_string()
    }

    fn parse_query_line(line: &str, profile: &mut Profile) {
        if let Some((key, value)) = line.split_once(": ") {
            match key.trim() {
                "- Query ID" => profile.summary.query_id = value.trim().to_string(),
                "- Start Time" => {
                    // 这里应该解析时间，但为了简化我们只存储字符串
                },
                "- End Time" => {
                    // 这里应该解析时间，但为了简化我们只存储字符串
                },
                "- Total" => profile.summary.total_time = value.trim().to_string(),
                "- Query State" => profile.summary.query_state = value.trim().to_string(),
                "- StarRocks Version" => profile.summary.starrocks_version = value.trim().to_string(),
                "- Sql Statement" => profile.summary.sql_statement = value.trim().to_string(),
                _ => {}
            }
        }
    }

    fn parse_planner_line(line: &str, profile: &mut Profile) {
        // 简化处理，将整个行作为键值对存储
        profile.planner.details.insert(
            line.to_string(),
            String::new()
        );
    }

    fn parse_execution_line(line: &str, profile: &mut Profile) {
        // 优先检查Topology
        if line.contains("Topology:") {
            if let Some(topology_start) = line.find('{') {
                profile.execution.topology = line[topology_start..].to_string();
            }
        } else if let Some((key, value)) = line.split_once(": ") {
            profile.execution.metrics.insert(
                key.trim().to_string(),
                value.trim().to_string()
            );
        }
    }

    fn parse_fragment_line(line: &str, fragment: &mut Fragment) {
        if line.starts_with("- BackendAddresses:") {
            // 解析后端地址
            let addresses: Vec<String> = line
                .split(": ")
                .nth(1)
                .unwrap_or("")
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            fragment.backend_addresses = addresses;
        } else if line.starts_with("- InstanceIds:") {
            // 解析实例ID
            let ids: Vec<String> = line
                .split(": ")
                .nth(1)
                .unwrap_or("")
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            fragment.instance_ids = ids;
        } else if line.trim().starts_with("Pipeline (id=") {
            // 创建新的pipeline
            let id = line
                .split('=')
                .nth(1)
                .unwrap_or("")
                .trim_end_matches("):")
                .to_string();
            
            fragment.pipelines.push(Pipeline {
                id,
                metrics: HashMap::new(),
                operators: Vec::new(),
            });
        } else if line.contains("):") && (line.contains("RESULT_SINK") || 
                                          line.contains("CHUNK_ACCUMULATE") || 
                                          line.contains("LIMIT") || 
                                          line.contains("EXCHANGE_SOURCE") ||
                                          line.contains("EXCHANGE_SINK") ||
                                          line.contains("CONNECTOR_SCAN")) {
            // 这是一个操作符定义行
            // 简化处理，只提取操作符名称
        } else if line.trim().starts_with("- ") {
            // 这是指标行
            // 简化处理
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Section {
    None,
    Query,
    Planner,
    Execution,
    Fragment,
}
