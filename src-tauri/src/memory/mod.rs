//! 记忆系统模块
//! 实现分层记忆机制：短期记忆 -> 中期记忆 -> 长期记忆

use crate::config::LlmConfig;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const MEMORY_FILE: &str = "memory.json";
const SHORT_TERM_LIMIT: usize = 500;
const MEDIUM_TERM_LIMIT: usize = 2000;

/// 记忆类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    /// 用户对话
    ChatHistory,
    /// 用户事实信息
    UserFact,
    /// 互动事件
    Interaction,
    /// 工具经验
    ToolExperience,
    /// 压缩摘要
    Summary,
    /// 核心印象
    CoreImpression,
}

/// 单条记忆项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    /// 唯一ID
    pub id: String,
    /// 创建时间戳
    pub timestamp: i64,
    /// 记忆内容
    pub content: String,
    /// 记忆类型
    pub memory_type: MemoryType,
    /// 重要性分数 (0.0 - 1.0)
    pub importance: f64,
    /// 标签列表
    pub tags: Vec<String>,
    /// 访问次数
    pub access_count: u32,
    /// 最后访问时间戳
    pub last_accessed: i64,
}

impl MemoryItem {
    /// 创建新记忆
    pub fn new(content: String, memory_type: MemoryType, importance: f64) -> Self {
        let now = Utc::now().timestamp();
        let tags = Self::extract_tags(&content);
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now,
            content,
            memory_type,
            importance: (importance as f64).clamp(0.0, 1.0),
            tags,
            access_count: 0,
            last_accessed: now,
        }
    }

    /// 从内容中提取标签（简单关键词提取）
    fn extract_tags(content: &str) -> Vec<String> {
        let keywords = [
            ("名字", "name"),
            ("我叫", "name"),
            ("喜欢", "preference"),
            ("讨厌", "preference"),
            ("生日", "birthday"),
            ("工作", "work"),
            ("家", "home"),
            ("开心", "emotion"),
            ("难过", "emotion"),
            ("想你", "emotion"),
            ("记住", "important"),
            ("别忘了", "important"),
            ("约定", "promise"),
        ];

        let mut tags = Vec::new();
        for (keyword, tag) in keywords.iter() {
            if content.contains(keyword) {
                tags.push(tag.to_string());
            }
        }
        tags.dedup();
        tags
    }

    /// 标记为已访问
    pub fn touch(&mut self) {
        self.access_count += 1;
        self.last_accessed = Utc::now().timestamp();
    }

    /// 格式化日期
    pub fn date_str(&self) -> String {
        DateTime::from_timestamp(self.timestamp, 0)
            .unwrap_or_default()
            .format("%Y-%m-%d")
            .to_string()
    }
}

/// 记忆重要性评分器
pub struct MemoryScorer;

impl MemoryScorer {
    /// 轻量级规则评分（不用 LLM，毫秒级）
    pub fn fast_score(&self, content: &str) -> f64 {
        let mut score: f64 = 0.3; // 基础分

        // 用户自我介绍或个人信息
        let personal_keywords = [
            "我叫", "我的名字", "我今年", "我工作", "我家", "我喜欢", "我讨厌", "生日",
            "我是", "我住", "我养", "我的",
        ];
        if personal_keywords.iter().any(|k| content.contains(k)) {
            score += 0.4;
        }

        // 情绪表达
        let emotion_keywords = ["开心", "难过", "生气", "想你", "爱你", "谢谢你", "对不起"];
        if emotion_keywords.iter().any(|k| content.contains(k)) {
            score += 0.2;
        }

        // 包含重要信息的标记
        let important_keywords = ["记住", "别忘了", "重要", "约定", "必须", "一定要"];
        if important_keywords.iter().any(|k| content.contains(k)) {
            score += 0.2;
        }

        // 疑问句权重稍低
        if content.contains('?') || content.contains('？') {
            score -= 0.1;
        }

        // 太短的内容权重低
        if content.chars().count() < 5 {
            score -= 0.1;
        }

        score.clamp(0.0, 1.0)
    }

    /// 计算内容的关键词匹配分数
    pub fn keyword_match_score(&self, content: &str, query: &str) -> f64 {
        let content_lower = content.to_lowercase();
        let query_lower = query.to_lowercase();

        let query_words: Vec<&str> = query_lower
            .split_whitespace()
            .filter(|s| s.len() >= 2)
            .collect();

        if query_words.is_empty() {
            return 0.0;
        }

        let mut match_count = 0;
        for word in &query_words {
            if content_lower.contains(word) {
                match_count += 1;
            }
        }

        match_count as f64 / query_words.len() as f64
    }
}

/// 分层记忆引擎
pub struct LayeredMemoryEngine {
    /// 短期记忆（0-500条）：完整保存
    short_term: Vec<MemoryItem>,
    /// 中期记忆（500-2000条）：按天压缩
    medium_term: Vec<MemoryItem>,
    /// 长期记忆（2000+条）：核心印象
    long_term: Vec<MemoryItem>,

    /// 评分器
    scorer: MemoryScorer,
    /// 存储路径
    storage_path: PathBuf,
    /// 短期记忆容量限制
    short_term_limit: usize,
    /// 中期记忆容量限制
    medium_term_limit: usize,
}

impl LayeredMemoryEngine {
    /// 创建新的记忆引擎
    pub fn new() -> Self {
        let storage_path = Self::default_memory_path();

        // 尝试从磁盘加载
        if let Ok(mem) = Self::load_from_path(&storage_path) {
            return mem;
        }

        Self {
            short_term: Vec::new(),
            medium_term: Vec::new(),
            long_term: Vec::new(),
            scorer: MemoryScorer,
            storage_path,
            short_term_limit: SHORT_TERM_LIMIT,
            medium_term_limit: MEDIUM_TERM_LIMIT,
        }
    }

    /// 默认记忆文件路径
    pub fn default_memory_path() -> PathBuf {
        if let Some(dir) = dirs::config_dir() {
            dir.join("desktop-pet").join(MEMORY_FILE)
        } else {
            PathBuf::from(".").join(MEMORY_FILE)
        }
    }

    /// 从文件加载
    pub fn load_from_path(path: &PathBuf) -> Result<Self, String> {
        if !path.exists() {
            return Err("Memory file not found".to_string());
        }

        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let raw: RawMemoryData = serde_json::from_str(&content).map_err(|e| e.to_string())?;

        Ok(Self {
            short_term: raw.short_term,
            medium_term: raw.medium_term,
            long_term: raw.long_term,
            scorer: MemoryScorer,
            storage_path: path.clone(),
            short_term_limit: SHORT_TERM_LIMIT,
            medium_term_limit: MEDIUM_TERM_LIMIT,
        })
    }

    /// 保存到文件
    pub fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let raw = RawMemoryData {
            short_term: self.short_term.clone(),
            medium_term: self.medium_term.clone(),
            long_term: self.long_term.clone(),
        };

        let content = serde_json::to_string_pretty(&raw).map_err(|e| e.to_string())?;
        fs::write(&self.storage_path, content).map_err(|e| e.to_string())?;

        Ok(())
    }

    /// 添加新记忆
    pub async fn add_memory(
        &mut self,
        content: &str,
        memory_type: MemoryType,
        _llm_config: Option<&LlmConfig>,
    ) -> Result<String, String> {
        // 1. 快速评分
        let importance = self.scorer.fast_score(content);

        let item = MemoryItem::new(content.to_string(), memory_type, importance);
        let id = item.id.clone();

        // 2. 加入短期记忆层
        self.short_term.push(item);

        // 3. 检查是否需要触发压缩
        if self.short_term.len() >= self.short_term_limit {
            self.compact_short_to_medium();
        }

        if self.medium_term.len() >= self.medium_term_limit {
            self.compact_medium_to_long();
        }

        // 4. 保存到磁盘
        self.save()?;

        Ok(id)
    }

    /// 添加对话对（用户提问 + 宠物回答）
    pub async fn add_chat_pair(
        &mut self,
        user_message: &str,
        pet_response: &str,
        llm_config: Option<&LlmConfig>,
    ) -> Result<(String, String), String> {
        let user_id = self
            .add_memory(
                &format!("主人说：{}", user_message),
                MemoryType::ChatHistory,
                llm_config,
            )
            .await?;

        let pet_id = self
            .add_memory(
                &format!("我回答：{}", pet_response),
                MemoryType::ChatHistory,
                llm_config,
            )
            .await?;

        Ok((user_id, pet_id))
    }

    /// 短期记忆 → 中期记忆 压缩
    fn compact_short_to_medium(&mut self) {
        eprintln!(
            "[Memory] 触发短期 → 中期压缩，当前: {} 条",
            self.short_term.len()
        );

        // 保留最新的 100 条在短期层
        let keep_in_short = 100;
        if self.short_term.len() <= keep_in_short {
            return;
        }

        let to_compact: Vec<_> = self.short_term.drain(0..self.short_term.len() - keep_in_short).collect();

        // 按天分组
        let mut groups: HashMap<String, Vec<MemoryItem>> = HashMap::new();
        for m in to_compact {
            groups.entry(m.date_str()).or_default().push(m);
        }

        for (_, group) in groups {
            // 一天少于 20 条直接保留
            if group.len() < 20 {
                self.medium_term.extend(group);
                continue;
            }

            // 超过 20 条：保留重要的，其余生成摘要
            let mut sorted: Vec<_> = group.iter().collect();
            sorted.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap_or(std::cmp::Ordering::Equal));

            // 保留前 5 条高重要性的
            let keep_count = 5.min(sorted.len());
            let to_keep: Vec<MemoryItem> = sorted.iter().take(keep_count).map(|m| (*m).clone()).collect();
            self.medium_term.extend(to_keep);
        }
    }

    /// 中期记忆 → 长期记忆 压缩
    fn compact_medium_to_long(&mut self) {
        eprintln!(
            "[Memory] 触发中期 → 长期压缩，当前: {} 条",
            self.medium_term.len()
        );

        // 保留中期记忆中重要性 > 0.7 的
        let (important, to_summarize): (Vec<_>, Vec<_>) = self
            .medium_term
            .drain(..)
            .partition(|m| m.importance > 0.7);

        self.long_term.extend(important);

        // 如果摘要积累太多，也清理掉
        if self.long_term.len() > 500 {
            self.long_term.truncate(500);
        }
    }

    /// 在单层记忆中搜索
    fn search_in_layer<'a>(
        &'a self,
        layer: &'a [MemoryItem],
        query: &str,
        limit: usize,
    ) -> Vec<&'a MemoryItem> {
        if limit == 0 {
            return Vec::new();
        }

        let mut scored: Vec<(f64, &MemoryItem)> = layer
            .iter()
            .map(|m| {
                let keyword_score = self.scorer.keyword_match_score(&m.content, query);
                let recency_score = 1.0 - (Utc::now().timestamp() - m.timestamp).max(0) as f64 / 86400.0 / 30.0; // 30天衰减
                let total_score = keyword_score * 0.6 + recency_score * 0.2 + m.importance * 0.2;
                (total_score, m)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        scored
            .into_iter()
            .take(limit)
            .map(|(_, m)| m)
            .collect()
    }

    /// 检索记忆（跨层搜索）
    pub fn retrieve(&mut self, query: &str, limit: usize) -> Vec<String> {
        if limit == 0 {
            return Vec::new();
        }

        // 1. 先在短期记忆搜索（最高优先级）
        let mut scored_results = Vec::new();
        scored_results.extend(self.score_layer(&self.short_term, query));
        scored_results.extend(self.score_layer(&self.medium_term, query));
        scored_results.extend(self.score_layer(&self.long_term, query));

        // 按分数排序
        scored_results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // 取前 limit 个，标记为已访问，并收集内容
        let mut contents = Vec::new();
        for (_, id) in scored_results.into_iter().take(limit) {
            // 标记为已访问
            if let Some(m) = self.short_term.iter_mut().find(|x| x.id == id) {
                m.touch();
                contents.push(m.content.clone());
            } else if let Some(m) = self.medium_term.iter_mut().find(|x| x.id == id) {
                m.touch();
                contents.push(m.content.clone());
            } else if let Some(m) = self.long_term.iter_mut().find(|x| x.id == id) {
                m.touch();
                contents.push(m.content.clone());
            }
        }

        contents
    }

    /// 给单层记忆打分，返回 (score, id)
    fn score_layer(&self, layer: &[MemoryItem], query: &str) -> Vec<(f64, String)> {
        layer
            .iter()
            .map(|m| {
                let keyword_score = self.scorer.keyword_match_score(&m.content, query);
                let recency_score = 1.0 - (Utc::now().timestamp() - m.timestamp).max(0) as f64 / 86400.0 / 30.0; // 30天衰减
                let total_score = keyword_score * 0.6 + recency_score * 0.2 + m.importance * 0.2;
                (total_score, m.id.clone())
            })
            .collect()
    }

    /// 获取最近的 N 条记忆
    pub fn get_recent(&self, limit: usize) -> Vec<&MemoryItem> {
        let mut all: Vec<&MemoryItem> = Vec::new();
        all.extend(&self.short_term);
        all.extend(&self.medium_term);
        all.extend(&self.long_term);

        all.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all.truncate(limit);
        all
    }

    /// 构建给 LLM 的记忆提示词
    pub fn build_memory_prompt(&mut self, query: &str, limit: usize) -> String {
        let memories = self.retrieve(query, limit);

        if memories.is_empty() {
            return String::new();
        }

        let mut prompt = "【关于主人的记忆】\n".to_string();

        for (i, content) in memories.iter().enumerate() {
            prompt.push_str(&format!("{}. {}\n", i + 1, content));
        }

        prompt.push_str("\n请根据这些记忆回复主人，要体现出我们熟悉的感觉~");
        prompt
    }

    /// 获取记忆统计信息
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            short_term_count: self.short_term.len(),
            medium_term_count: self.medium_term.len(),
            long_term_count: self.long_term.len(),
            total_count: self.short_term.len() + self.medium_term.len() + self.long_term.len(),
        }
    }
}

/// 内存统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub short_term_count: usize,
    pub medium_term_count: usize,
    pub long_term_count: usize,
    pub total_count: usize,
}

/// 原始内存数据结构（用于序列化）
#[derive(Debug, Serialize, Deserialize)]
struct RawMemoryData {
    short_term: Vec<MemoryItem>,
    medium_term: Vec<MemoryItem>,
    long_term: Vec<MemoryItem>,
}

impl Default for LayeredMemoryEngine {
    fn default() -> Self {
        Self::new()
    }
}
