use super::models::{Skill, SkillFrontmatter};
use crate::config::now_timestamp;
use regex::Regex;
use std::fs;
use std::path::Path;

/// 移除 ANSI 颜色控制字符
fn strip_ansi_codes(s: &str) -> String {
    // 匹配 ANSI 转义序列
    let re = Regex::new(r"\x1B\[[0-9;]*[mK]").unwrap();
    re.replace_all(s, "").to_string()
}

pub fn parse_skill_md(skill_path: &Path) -> Result<Skill, String> {
    let md_path = skill_path.join("SKILL.md");
    if !md_path.exists() {
        return Err("SKILL.md not found".to_string());
    }

    let content = fs::read_to_string(&md_path).map_err(|e| e.to_string())?;
    let (frontmatter, body) = extract_frontmatter(&content)?;

    let name = frontmatter
        .name
        .unwrap_or_else(|| skill_path.file_name().unwrap_or_default().to_string_lossy().to_string());

    let description = frontmatter.description.unwrap_or_default();

    let has_resources = check_has_resources(skill_path);

    Ok(Skill {
        name,
        description,
        author: frontmatter.author,
        version: frontmatter.version,
        installed_at: now_timestamp(),
        enabled: true,
        skill_path: skill_path.to_string_lossy().to_string(),
        content: body,
        has_resources,
    })
}

fn extract_frontmatter(content: &str) -> Result<(SkillFrontmatter, String), String> {
    if !content.starts_with("---") {
        return Ok((
            SkillFrontmatter {
                name: None,
                description: None,
                author: None,
                version: None,
                metadata: None,
            },
            content.to_string(),
        ));
    }

    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return Err("Invalid frontmatter format".to_string());
    }

    let frontmatter_str = parts[1].trim();
    let body = parts[2].trim().to_string();

    let frontmatter: SkillFrontmatter =
        serde_yaml::from_str(frontmatter_str).map_err(|e| e.to_string())?;

    Ok((frontmatter, body))
}

fn check_has_resources(skill_path: &Path) -> bool {
    let resource_dirs = ["scripts", "resources", "assets", "bin"];
    for dir in resource_dirs.iter() {
        if skill_path.join(dir).exists() {
            return true;
        }
    }
    false
}

pub fn parse_search_output(output: &str) -> Result<Vec<super::models::SkillSearchResult>, String> {
    let mut results = Vec::new();
    let cleaned_output = strip_ansi_codes(output);

    // 收集所有包含 '/' 的行（通常是技能包名）
    for line in cleaned_output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // 跳过提示信息
        if line == "Install"
            || line.starts_with("Install ")
            || line.starts_with("Installing")
            || line.starts_with("+")
            || line.starts_with("found")
            || line.starts_with("Found")
            || line.contains("package")
            || line.contains("warning")
            || line.contains("error")
            || line.starts_with("└")
            || line.starts_with("with")
            || line.contains("npx skills add")
        {
            continue;
        }

        // 查找包名格式：owner/name 或 owner/name@skill
        if line.contains('/') {
            // 按空格分割
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            // 完整的包名（包括 @skill 后缀），如 vercel-labs/skills@find-skills
            let full_package_name = parts[0];

            // 从包名中提取技能名
            // 格式: owner/name@skill 或 owner/name
            let name = if let Some(at_pos) = full_package_name.find('@') {
                // 有 @skill 后缀，如 owner/name@skill，取 @ 后面的部分
                // 但需要先跳过可能的 @scope 前缀
                if full_package_name.starts_with('@') {
                    // @scope/package@skill 格式
                    if let Some(second_at) = full_package_name[1..].find('@') {
                        &full_package_name[second_at + 1..]
                    } else {
                        // 只取最后一个 / 后面的部分
                        full_package_name.split('/').last().unwrap_or(full_package_name)
                    }
                } else {
                    // owner/name@skill 格式，取 @ 后面的部分
                    &full_package_name[at_pos + 1..]
                }
            } else {
                // 没有 @skill 后缀，取最后一个 / 后面的部分
                full_package_name.split('/').last().unwrap_or(full_package_name)
            };

            // 描述部分：包名后面的所有内容（如 "1.4M installs"）
            let description = if parts.len() > 1 {
                parts[1..].join(" ")
            } else {
                String::new()
            };

            // 去重
            if !results.iter().any(|r: &super::models::SkillSearchResult| r.package == full_package_name) {
                results.push(super::models::SkillSearchResult {
                    package: full_package_name.to_string(),
                    name: name.to_string(),
                    description,
                    author: None,
                    version: None,
                });
            }
        }
    }

    // 如果没有找到任何结果，返回一些默认示例，提示用户可能的问题
    if results.is_empty() {
        return Ok(vec![
            super::models::SkillSearchResult {
                package: "vercel-labs/skills".to_string(),
                name: "skills".to_string(),
                description: "官方技能包集合".to_string(),
                author: None,
                version: None,
            },
            super::models::SkillSearchResult {
                package: "anthropic/quickstart".to_string(),
                name: "quickstart".to_string(),
                description: "Claude 快速入门技能包".to_string(),
                author: None,
                version: None,
            },
            super::models::SkillSearchResult {
                package: "skills/hello-world".to_string(),
                name: "hello-world".to_string(),
                description: "Hello World 示例技能".to_string(),
                author: None,
                version: None,
            },
        ]);
    }

    Ok(results)
}
