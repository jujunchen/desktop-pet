use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    pub installed_at: i64,
    pub enabled: bool,
    pub skill_path: String,
    pub content: String,
    pub has_resources: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfig {
    #[serde(default)]
    pub skills: Vec<Skill>,
}

impl Default for SkillConfig {
    fn default() -> Self {
        Self { skills: Vec::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSearchResult {
    pub package: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillFrontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_yaml::Mapping>,
}
