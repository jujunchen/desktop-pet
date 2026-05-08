use super::models::{Skill, SkillConfig, SkillSearchResult};
use super::parser::parse_skill_md;
use crate::config::APP_DIR;
use dirs::config_dir;
use regex::Regex;
use std::fs;
use std::path::PathBuf;
use tokio::process::Command as AsyncCommand;

const SKILLS_DIR: &str = "skills";
const SKILL_CONFIG_FILE: &str = "skills.json";

/// 移除 ANSI 颜色控制字符
fn strip_ansi_codes(s: &str) -> String {
    let re = Regex::new(r"\x1B\[[0-9;]*[mK]").unwrap();
    re.replace_all(s, "").to_string()
}

/// 检查 npx 是否可用
async fn check_npx_available() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        // Windows 使用 cmd /c 检查
        let result = AsyncCommand::new("cmd")
            .args(&["/c", "npx", "--version"])
            .output()
            .await;

        if let Ok(output) = result {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return Ok(version);
            }
            // 返回更详细的错误信息帮助调试
            let stderr = strip_ansi_codes(&String::from_utf8_lossy(&output.stderr));
            return Err(format!(
                "npx 执行失败，退出码: {:?}\n错误输出: {}",
                output.status.code(),
                stderr
            ));
        }

        return Err(format!(
            "无法执行 cmd /c npx，错误: {:?}",
            result.err()
        ));
    }

    #[cfg(not(target_os = "windows"))]
    {
        let result = AsyncCommand::new("npx").arg("--version").output().await;

        if let Ok(output) = result {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return Ok(version);
            }
            let stderr = strip_ansi_codes(&String::from_utf8_lossy(&output.stderr));
            return Err(format!(
                "npx 执行失败，退出码: {:?}\n错误输出: {}",
                output.status.code(),
                stderr
            ));
        }

        Err(format!(
            "无法执行 npx 命令，错误: {:?}",
            result.err()
        ))
    }
}

fn skills_config_path() -> PathBuf {
    if let Some(dir) = config_dir() {
        dir.join(APP_DIR).join(SKILL_CONFIG_FILE)
    } else {
        PathBuf::from(".").join(APP_DIR).join(SKILL_CONFIG_FILE)
    }
}

fn skills_install_dir() -> PathBuf {
    if let Some(dir) = config_dir() {
        dir.join(APP_DIR).join(SKILLS_DIR)
    } else {
        PathBuf::from(".").join(APP_DIR).join(SKILLS_DIR)
    }
}

pub fn load_skill_config() -> Result<SkillConfig, String> {
    let path = skills_config_path();
    if path.exists() {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let config: SkillConfig = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        return Ok(config);
    }
    Ok(SkillConfig::default())
}

pub fn save_skill_config(config: &SkillConfig) -> Result<(), String> {
    let path = skills_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let body = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(path, body).map_err(|e| e.to_string())
}

pub fn list_skills() -> Result<Vec<Skill>, String> {
    let config = load_skill_config()?;
    Ok(config.skills)
}

pub async fn install_skill(package: &str) -> Result<Skill, String> {
    let install_dir = skills_install_dir();
    fs::create_dir_all(&install_dir).map_err(|e| e.to_string())?;

    // 检查 npx 是否可用
    if let Err(e) = check_npx_available().await {
        return Err(format!(
            "未找到 Node.js/npm 或无法执行命令。\n技能安装功能需要先安装 Node.js 环境。\n请访问 https://nodejs.org/ 安装后重试。\n错误: {}",
            e
        ));
    }

    #[cfg(target_os = "windows")]
    let output = AsyncCommand::new("cmd")
        .args(&["/c", "npx", "skills", "add", package, "--skill", "*", "-y"])
        .current_dir(&install_dir)
        .output()
        .await
        .map_err(|e| {
            format!(
                "执行 npx 失败: {}\n请确保已安装 Node.js 并且网络连接正常。",
                e
            )
        })?;

    #[cfg(not(target_os = "windows"))]
    let output = AsyncCommand::new("npx")
        .args(&["skills", "add", package, "--skill", "*", "-y"])
        .current_dir(&install_dir)
        .output()
        .await
        .map_err(|e| {
            format!(
                "执行 npx 失败: {}\n请确保已安装 Node.js 并且网络连接正常。",
                e
            )
        })?;

    if !output.status.success() {
        let stderr = strip_ansi_codes(&String::from_utf8_lossy(&output.stderr));
        if stderr.contains("network") || stderr.contains("fetch") || stderr.contains("ECONN") {
            return Err(format!("网络连接失败，请检查网络后重试：\n{}", stderr));
        }
        return Err(format!("安装失败: {}", stderr));
    }

    let mut config = load_skill_config()?;

    // skills.sh 在 .agents/skills/ 子目录下安装技能
    let skills_subdir = install_dir.join(".agents").join("skills");
    if skills_subdir.exists() {
        let skill_dirs = fs::read_dir(&skills_subdir).map_err(|e| e.to_string())?;
        for entry in skill_dirs {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                let skill_md = path.join("SKILL.md");
                if skill_md.exists() {
                    let mut skill = parse_skill_md(&path)?;
                    // 修正技能路径为实际安装路径
                    skill.skill_path = path.to_string_lossy().to_string();
                    if !config.skills.iter().any(|s| s.name == skill.name) {
                        config.skills.push(skill.clone());
                        save_skill_config(&config)?;
                        return Ok(skill);
                    }
                }
            }
        }
    }

    // 备选：直接在安装目录下查找
    let skill_dirs = fs::read_dir(&install_dir).map_err(|e| e.to_string())?;
    for entry in skill_dirs {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() && path.file_name().and_then(|n| n.to_str()) != Some(".agents") {
            let skill_md = path.join("SKILL.md");
            if skill_md.exists() {
                let skill = parse_skill_md(&path)?;
                if !config.skills.iter().any(|s| s.name == skill.name) {
                    config.skills.push(skill.clone());
                    save_skill_config(&config)?;
                    return Ok(skill);
                }
            }
        }
    }

    Err("Skill installed but not found in directory".to_string())
}

pub fn uninstall_skill(name: &str) -> Result<(), String> {
    let mut config = load_skill_config()?;

    if let Some(skill) = config.skills.iter().find(|s| s.name == name) {
        let skill_path = PathBuf::from(&skill.skill_path);
        if skill_path.exists() {
            fs::remove_dir_all(&skill_path).map_err(|e| e.to_string())?;
        }
    }

    config.skills.retain(|s| s.name != name);
    save_skill_config(&config)
}

pub fn enable_skill(name: &str) -> Result<(), String> {
    let mut config = load_skill_config()?;
    if let Some(skill) = config.skills.iter_mut().find(|s| s.name == name) {
        skill.enabled = true;
        save_skill_config(&config)?;
        Ok(())
    } else {
        Err(format!("Skill '{}' not found", name))
    }
}

pub fn disable_skill(name: &str) -> Result<(), String> {
    let mut config = load_skill_config()?;
    if let Some(skill) = config.skills.iter_mut().find(|s| s.name == name) {
        skill.enabled = false;
        save_skill_config(&config)?;
        Ok(())
    } else {
        Err(format!("Skill '{}' not found", name))
    }
}

pub async fn search_skills(query: &str) -> Result<Vec<SkillSearchResult>, String> {
    // 先检查 npx 是否可用
    if let Err(e) = check_npx_available().await {
        return Err(format!(
            "未找到 Node.js/npm 或无法执行命令。\n技能搜索功能需要先安装 Node.js 环境。\n请访问 https://nodejs.org/ 安装后重试。\n错误: {}",
            e
        ));
    }

    #[cfg(target_os = "windows")]
    let output = AsyncCommand::new("cmd")
        .args(&["/c", "npx", "skills", "find", query])
        .output()
        .await
        .map_err(|e| {
            format!(
                "执行 npx 失败: {}\n请确保已安装 Node.js 并且网络连接正常。",
                e
            )
        })?;

    #[cfg(not(target_os = "windows"))]
    let output = AsyncCommand::new("npx")
        .args(&["skills", "find", query])
        .output()
        .await
        .map_err(|e| {
            format!(
                "执行 npx 失败: {}\n请确保已安装 Node.js 并且网络连接正常。",
                e
            )
        })?;

    if !output.status.success() {
        let stderr = strip_ansi_codes(&String::from_utf8_lossy(&output.stderr));
        // 如果是网络错误，给出友好提示
        if stderr.contains("network") || stderr.contains("fetch") || stderr.contains("ECONN") {
            return Err(format!("网络连接失败，请检查网络后重试：\n{}", stderr));
        }
        return Err(format!("搜索失败: {}", stderr));
    }

    let stdout = strip_ansi_codes(&String::from_utf8_lossy(&output.stdout));
    super::parser::parse_search_output(&stdout)
}

pub fn get_enabled_skills() -> Result<Vec<Skill>, String> {
    let config = load_skill_config()?;
    Ok(config.skills.into_iter().filter(|s| s.enabled).collect())
}
