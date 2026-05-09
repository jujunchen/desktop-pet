//! 技能检索引擎
//! 根据用户查询动态检索最相关的技能
//! 复用记忆系统的关键词匹配算法

use super::models::Skill;

/// 技能检索器
pub struct SkillRetriever;

impl SkillRetriever {
    /// 检索与查询最相关的 N 个技能
    pub fn retrieve<'a>(query: &str, skills: &'a [Skill], top_n: usize) -> Vec<&'a Skill> {
        if skills.is_empty() || top_n == 0 {
            return Vec::new();
        }

        // 计算每个技能的相关性分数
        let mut scored: Vec<(f64, &Skill)> = skills
            .iter()
            .map(|skill| (Self::score_skill(query, skill), skill))
            .collect();

        // 按分数降序排序
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // 只返回分数 > 0 的技能（即有相关性的），最多 top_n 个
        scored
            .into_iter()
            .filter(|(score, _)| *score > 0.0)
            .take(top_n)
            .map(|(_, skill)| skill)
            .collect()
    }

    /// 计算技能与查询的相关性分数
    /// 范围: 0.0 ~ 1.0，分数越高越相关
    pub fn score_skill(query: &str, skill: &Skill) -> f64 {
        // 1. 技能名称的匹配分数（权重最高）
        let name_score = Self::keyword_match_score(&skill.name, query);

        // 2. 技能描述的匹配分数（次高权重）
        let desc_score = Self::keyword_match_score(&skill.description, query);

        // 3. 技能内容的匹配分数（较低权重）
        let content_score = Self::keyword_match_score(&skill.content, query);

        // 加权求和
        let total_score = name_score * 0.5 + desc_score * 0.35 + content_score * 0.15;

        total_score.clamp(0.0, 1.0)
    }

    /// 关键词匹配分数
    /// 简单但高效的中/英文混合关键词匹配算法
    fn keyword_match_score(content: &str, query: &str) -> f64 {
        let content_lower = content.to_lowercase();
        let query_lower = query.to_lowercase();

        // 对于中文，我们按字符而不是空格分割，提取连续的有意义片段
        // 对于英文，按空格分割
        let query_tokens = Self::tokenize(&query_lower);

        if query_tokens.is_empty() {
            return 0.0;
        }

        let mut match_count = 0;
        for token in &query_tokens {
            if content_lower.contains(token) {
                match_count += 1;
            }
        }

        match_count as f64 / query_tokens.len() as f64
    }

    /// 简单的分词，支持中/英文混合
    fn tokenize(text: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current_word = String::new();

        for c in text.chars() {
            if c.is_ascii_alphanumeric() {
                // 英文字符/数字，累积成词
                current_word.push(c.to_ascii_lowercase());
            } else {
                // 非英文字符
                if !current_word.is_empty() {
                    if current_word.len() >= 2 {
                        tokens.push(std::mem::take(&mut current_word));
                    } else {
                        current_word.clear();
                    }
                }

                // 中文字符：单字作为 token，但有一些过滤
                if Self::is_cjk(c) && !c.is_whitespace() && !Self::is_punctuation(c) {
                    tokens.push(c.to_string());
                }
            }
        }

        if !current_word.is_empty() && current_word.len() >= 2 {
            tokens.push(current_word);
        }

        tokens
    }

    /// 判断是否为中日韩字符
    const fn is_cjk(c: char) -> bool {
        (c >= '\u{4E00}' && c <= '\u{9FFF}')
            || (c >= '\u{3400}' && c <= '\u{4DBF}')
            || (c >= '\u{20000}' && c <= '\u{2A6DF}')
    }

    /// 判断是否为标点符号
    const fn is_punctuation(c: char) -> bool {
        c.is_ascii_punctuation()
            || matches!(c, '，' | '。' | '！' | '？' | '；' | '：' | '「' | '」' | '、' | '…')
    }
}

/// 便捷函数：检索相关技能
pub fn retrieve_relevant_skills<'a>(query: &str, skills: &'a [Skill], top_n: usize) -> Vec<&'a Skill> {
    SkillRetriever::retrieve(query, skills, top_n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_skill(name: &str, description: &str, content: &str) -> Skill {
        Skill {
            name: name.to_string(),
            description: description.to_string(),
            author: None,
            version: None,
            installed_at: 0,
            enabled: true,
            skill_path: String::new(),
            content: content.to_string(),
            has_resources: false,
        }
    }

    #[test]
    fn test_retrieve_relevant_skills() {
        let skills = vec![
            create_test_skill("代码审查", "帮你审查代码质量和最佳实践", "当用户发送代码时，检查语法错误，提出改进建议..."),
            create_test_skill("翻译助手", "多语言翻译和本地化支持", "当用户需要翻译时，支持中英日韩等多语言互译..."),
            create_test_skill("部署工具", "帮你部署应用到各种平台", "当用户提到部署时，支持Vercel、Docker等部署方式..."),
        ];

        // 使用"翻译"作为关键词，明确匹配到翻译助手
        let result = retrieve_relevant_skills("翻译", &skills, 2);
        assert!(!result.is_empty());
        assert_eq!(result[0].name, "翻译助手");

        // 使用"代码"作为关键词
        let result = retrieve_relevant_skills("代码", &skills, 2);
        assert!(!result.is_empty());
        assert_eq!(result[0].name, "代码审查");
    }

    #[test]
    fn test_score_skill() {
        let skill = create_test_skill("代码审查", "审查代码质量", "检查语法错误");

        // 名称匹配应该得到高分
        let score1 = SkillRetriever::score_skill("代码", &skill);
        assert!(score1 > 0.4);

        // 完全不相关
        let score2 = SkillRetriever::score_skill("做饭", &skill);
        assert_eq!(score2, 0.0);
    }

    #[test]
    fn test_tokenize() {
        // 英文
        let tokens = SkillRetriever::tokenize("hello world");
        assert_eq!(tokens, vec!["hello", "world"]);

        // 中文
        let tokens = SkillRetriever::tokenize("代码审查");
        assert_eq!(tokens, vec!["代", "码", "审", "查"]);

        // 混合
        let tokens = SkillRetriever::tokenize("code 代码");
        assert_eq!(tokens, vec!["code", "代", "码"]);
    }

    #[test]
    fn test_empty_skills() {
        let result = retrieve_relevant_skills("test", &[], 2);
        assert!(result.is_empty());
    }

    #[test]
    fn test_no_matching_skills() {
        let skills = vec![create_test_skill("代码审查", "审查代码", "...")];
        let result = retrieve_relevant_skills("做饭炒菜", &skills, 2);
        assert!(result.is_empty());
    }
}
