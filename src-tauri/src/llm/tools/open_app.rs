//! 打开系统应用工具（支持搜索任意应用，ReAct 智能候选模式）

use super::registry::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

/// 匹配候选
#[derive(Debug, Clone)]
struct AppCandidate {
    path: PathBuf,
    display_name: String,
    source: String, // "开始菜单" | "桌面" | "Program Files"
    score: i32,     // 匹配质量分数，用于排序
}

/// 打开应用工具
pub struct OpenAppTool;

#[async_trait]
impl Tool for OpenAppTool {
    fn name(&self) -> &str {
        "open_app"
    }

    fn description(&self) -> &str {
        "打开电脑上的任意应用程序。
当用户说'打开XXX'、'启动XXX'、'运行XXX'、'卸载XXX'时使用。
如果搜索到多个匹配，会返回候选列表让你选择。"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "app_name": {
                    "type": "string",
                    "description": "应用名称，如：微信、QQ、Chrome、VSCode、Word、Photoshop、Steam 等"
                },
                "select_index": {
                    "type": "integer",
                    "description": "可选：从候选列表中选择第几个（从1开始）。当返回了多个候选时，用这个参数来指定启动哪个。"
                }
            },
            "required": ["app_name"]
        })
    }

    async fn execute(&self, _app: tauri::AppHandle, args: Value) -> ToolResult {
        let app_name = args
            .get("app_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "缺少 app_name 参数".to_string())?;

        #[cfg(target_os = "windows")]
        {
            let app_name_lower = app_name.to_lowercase();

            // 检查是否有 select_index 参数（从候选列表中选择）
            if let Some(Some(index)) = args.get("select_index").map(|v| v.as_i64()) {
                return launch_by_index(&app_name_lower, index as usize).await;
            }

            // 方案1：先尝试常用应用的直接启动（精确匹配，无歧义）
            if let Ok(result) = try_known_apps(&app_name_lower) {
                return Ok(result);
            }

            // 方案2：收集所有候选，让 LLM 决策
            let mut candidates = Vec::new();

            // 搜索开始菜单
            if let Ok(mut lnks) = search_lnk_candidates(&get_start_menu_paths(), &app_name_lower).await {
                candidates.append(&mut lnks);
            }

            // 搜索桌面
            let desktop_path = format!("C:\\Users\\{}\\Desktop", get_username());
            if let Ok(mut lnks) = search_lnk_candidates(&[desktop_path], &app_name_lower).await {
                candidates.append(&mut lnks);
            }

            // 搜索安装目录
            if let Ok(mut exes) = search_exe_candidates(&get_common_install_dirs(), &app_name_lower).await {
                candidates.append(&mut exes);
            }

            // 按分数排序（高质量在前）
            candidates.sort_by(|a, b| b.score.cmp(&a.score));

            // 去重（相同路径只保留一个）
            candidates.dedup_by(|a, b| a.path == b.path);

            // 根据候选数量决策
            match candidates.len() {
                0 => {
                    // 没找到，最后尝试直接启动命令
                    if let Ok(result) = try_start_command(&app_name_lower) {
                        return Ok(result);
                    }
                    Ok(format!(
                        "汪呜...找不到'{}'这个应用呢😢\n\
                        试试说更准确的名字？比如完整的应用名称～",
                        app_name
                    ))
                }
                1 => {
                    // 只有1个匹配，直接启动
                    let candidate = &candidates[0];
                    if launch_path(&candidate.path) {
                        Ok(format!("汪汪！已成功打开: {}", candidate.display_name))
                    } else {
                        Ok(format!("汪呜...打开失败了: {}", candidate.display_name))
                    }
                }
                n => {
                    // 多个匹配，返回候选列表给 LLM 选择
                    let mut result = format!("找到了 {} 个匹配的应用，请选择：\n\n", n);
                    for (i, c) in candidates.iter().take(5).enumerate() {
                        result.push_str(&format!("{}. {} ({})\n", i + 1, c.display_name, c.source));
                    }
                    if n > 5 {
                        result.push_str(&format!("\n...还有 {} 个更多结果", n - 5));
                    }
                    result.push_str("\n请用 select_index 参数选择序号，比如：{\"app_name\": \"微信\", \"select_index\": 1}");

                    Ok(result)
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            Err("打开应用功能仅支持Windows系统".to_string())
        }
    }
}

/// 根据序号启动（存储最近一次搜索结果的简化方案：重新搜索再选）
#[cfg(target_os = "windows")]
async fn launch_by_index(app_name: &str, index: usize) -> ToolResult {
    let mut candidates = Vec::new();

    if let Ok(mut lnks) = search_lnk_candidates(&get_start_menu_paths(), app_name).await {
        candidates.append(&mut lnks);
    }

    let desktop_path = format!("C:\\Users\\{}\\Desktop", get_username());
    if let Ok(mut lnks) = search_lnk_candidates(&[desktop_path], app_name).await {
        candidates.append(&mut lnks);
    }

    if let Ok(mut exes) = search_exe_candidates(&get_common_install_dirs(), app_name).await {
        candidates.append(&mut exes);
    }

    candidates.sort_by(|a, b| b.score.cmp(&a.score));
    candidates.dedup_by(|a, b| a.path == b.path);

    if index < 1 || index > candidates.len() {
        return Ok(format!("汪呜...序号 {} 无效，只有 {} 个候选", index, candidates.len()));
    }

    let candidate = &candidates[index - 1];
    if launch_path(&candidate.path) {
        Ok(format!("汪汪！已成功打开: {}", candidate.display_name))
    } else {
        Ok(format!("汪呜...打开失败了: {}", candidate.display_name))
    }
}

/// 尝试常用应用的直接启动
#[cfg(target_os = "windows")]
fn try_known_apps(app_name: &str) -> Result<String, ()> {
    // 预设常用应用的启动命令
    let known_apps = [
        ("notepad", "notepad.exe"),
        ("记事本", "notepad.exe"),
        ("calc", "calc.exe"),
        ("计算器", "calc.exe"),
        ("explorer", "explorer.exe"),
        ("资源管理器", "explorer.exe"),
        ("文件管理器", "explorer.exe"),
        ("cmd", "cmd.exe"),
        ("命令行", "cmd.exe"),
        ("终端", "cmd.exe"),
        ("powershell", "powershell.exe"),
        ("chrome", "chrome.exe"),
        ("谷歌", "chrome.exe"),
        ("谷歌浏览器", "chrome.exe"),
        ("浏览器", "chrome.exe"),
        ("edge", "msedge.exe"),
        ("edge浏览器", "msedge.exe"),
        ("vscode", "code"),
        ("code", "code"),
        ("vs code", "code"),
        ("word", "winword.exe"),
        ("文档", "winword.exe"),
        ("excel", "excel.exe"),
        ("表格", "excel.exe"),
        ("powerpoint", "powerpnt.exe"),
        ("ppt", "powerpnt.exe"),
        ("幻灯片", "powerpnt.exe"),
        ("paint", "mspaint.exe"),
        ("画图", "mspaint.exe"),
        ("mspaint", "mspaint.exe"),
        ("画笔", "mspaint.exe"),
        ("taskmgr", "taskmgr.exe"),
        ("任务管理器", "taskmgr.exe"),
        ("control", "control.exe"),
        ("控制面板", "control.exe"),
        ("设置", "ms-settings:"),
    ];

    for (keyword, exe_name) in known_apps {
        // 只有精确匹配才直接启动，避免歧义
        if app_name == keyword {
            if let Ok(_) = Command::new("cmd").args(["/c", "start", "", exe_name]).spawn() {
                return Ok(format!("已成功打开: {}", keyword));
            }
        }
    }

    Err(())
}

/// 使用 Windows start 命令直接尝试启动（仅用于纯英文的可执行文件）
#[cfg(target_os = "windows")]
fn try_start_command(app_name: &str) -> Result<String, ()> {
    let is_pure_english = app_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !is_pure_english {
        return Err(());
    }

    let found = Command::new("cmd")
        .args(["/c", "where", app_name])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !found {
        return Err(());
    }

    if Command::new("cmd")
        .args(["/c", "start", "", app_name])
        .spawn()
        .is_ok()
    {
        return Ok(format!("已成功打开: {}", app_name));
    }

    Err(())
}

/// 获取开始菜单路径
#[cfg(target_os = "windows")]
fn get_start_menu_paths() -> Vec<String> {
    vec![
        format!("C:\\Users\\{}\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs", get_username()),
        "C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs".to_string(),
    ]
}

/// 获取常见安装目录
#[cfg(target_os = "windows")]
fn get_common_install_dirs() -> Vec<String> {
    let username = get_username();
    vec![
        "C:\\Program Files".to_string(),
        "C:\\Program Files (x86)".to_string(),
        format!("C:\\Users\\{}\\AppData\\Local\\Programs", username),
        format!("C:\\Users\\{}\\AppData\\Roaming", username),
        "C:\\Program Files\\Microsoft Office\\root\\Office16".to_string(),
    ]
}

/// 搜索快捷方式候选
#[cfg(target_os = "windows")]
async fn search_lnk_candidates(dirs: &[String], app_name: &str) -> Result<Vec<AppCandidate>, ()> {
    let mut candidates = Vec::new();
    let patterns = build_search_patterns(app_name);

    for dir in dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let filename = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_lowercase();

                    if filename.ends_with(".lnk") && patterns.iter().any(|p| filename.contains(p)) {
                        let score = calculate_match_score(&filename, app_name, &patterns, true);
                        candidates.push(AppCandidate {
                            display_name: path.file_stem().and_then(|s| s.to_str()).unwrap_or("应用").to_string(),
                            source: if dir.contains("Start Menu") { "开始菜单".into() } else { "桌面".into() },
                            path,
                            score,
                        });
                    }
                } else if path.is_dir() {
                    // 递归搜索一级子目录
                    if let Ok(sub_entries) = std::fs::read_dir(&path) {
                        for sub_entry in sub_entries.flatten() {
                            let sub_path = sub_entry.path();
                            if sub_path.is_file() {
                                let filename = sub_path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("")
                                    .to_lowercase();

                                if filename.ends_with(".lnk") && patterns.iter().any(|p| filename.contains(p)) {
                                    let score = calculate_match_score(&filename, app_name, &patterns, true);
                                    candidates.push(AppCandidate {
                                        display_name: sub_path.file_stem().and_then(|s| s.to_str()).unwrap_or("应用").to_string(),
                                        source: if dir.contains("Start Menu") { "开始菜单".into() } else { "桌面".into() },
                                        path: sub_path,
                                        score,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(candidates)
}

/// 搜索 exe 候选
#[cfg(target_os = "windows")]
async fn search_exe_candidates(dirs: &[String], app_name: &str) -> Result<Vec<AppCandidate>, ()> {
    let mut candidates = Vec::new();
    let patterns = build_search_patterns(app_name);

    for dir in dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let filename = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_lowercase();

                    if (filename.ends_with(".exe") || filename.ends_with(".lnk"))
                        && patterns.iter().any(|p| filename.contains(p))
                    {
                        let score = calculate_match_score(&filename, app_name, &patterns, false);
                        candidates.push(AppCandidate {
                            display_name: path.file_stem().and_then(|s| s.to_str()).unwrap_or("应用").to_string(),
                            source: "Program Files".into(),
                            path,
                            score,
                        });
                    }
                } else if path.is_dir() {
                    // 目录名匹配，进入搜索
                    let dirname = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_lowercase();

                    if patterns.iter().any(|p| dirname.contains(p)) {
                        if let Ok(sub_entries) = std::fs::read_dir(&path) {
                            for sub_entry in sub_entries.flatten() {
                                let sub_path = sub_entry.path();
                                if sub_path.is_file() {
                                    let filename = sub_path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("")
                                        .to_lowercase();

                                    if (filename.ends_with(".exe") || filename.ends_with(".lnk"))
                                        && patterns.iter().any(|p| filename.contains(p))
                                    {
                                        let score = calculate_match_score(&filename, app_name, &patterns, false);
                                        candidates.push(AppCandidate {
                                            display_name: sub_path.file_stem().and_then(|s| s.to_str()).unwrap_or("应用").to_string(),
                                            source: "Program Files".into(),
                                            path: sub_path,
                                            score,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(candidates)
}

/// 计算匹配质量分数
#[cfg(target_os = "windows")]
fn calculate_match_score(filename: &str, app_name: &str, patterns: &[String], is_lnk: bool) -> i32 {
    let mut score = 0;

    // 快捷方式优先（用户通常点击的就是快捷方式）
    if is_lnk {
        score += 50;
    }

    // 精确匹配加分
    let name_no_ext = filename.trim_end_matches(".lnk").trim_end_matches(".exe");
    if name_no_ext == app_name {
        score += 100;
    }

    // 文件名开头匹配
    if name_no_ext.starts_with(app_name) {
        score += 50;
    }

    // 包含用户搜索的关键词（不通过别名映射的原始匹配）
    if filename.contains(app_name) {
        score += 30;
    }

    // 别名匹配也加分
    for pattern in patterns {
        if filename.contains(pattern) {
            score += 20;
        }
    }

    score
}

/// 构建搜索关键词列表（别名映射）
#[cfg(target_os = "windows")]
fn build_search_patterns(app_name: &str) -> Vec<String> {
    let mut patterns = vec![app_name.to_string()];

    let alias_map = [
        ("微信", &["wechat", "weixin"][..]),
        ("qq", &["qq", "tim", "腾讯qq", "tencent qq", "qq.exe"][..]),
        ("腾讯", &["qq", "tim", "tencent", "腾讯qq"][..]),
        ("wechat", &["微信"][..]),
        ("vscode", &["code", "vscode", "visual studio code"][..]),
        ("visual studio", &["devenv", "visual studio", "vs"][..]),
        ("idea", &["idea64", "idea", "intellij"][..]),
        ("intellij", &["idea64", "idea"][..]),
        ("photoshop", &["photoshop", "ps"][..]),
        ("ps", &["photoshop"][..]),
        ("网易云", &["cloudmusic", "网易云音乐", "netease"][..]),
        ("钉钉", &["dingtalk", "ding"][..]),
        ("飞书", &["feishu", "lark"][..]),
        ("企业微信", &["wxwork", "wecom"][..]),
        ("steam", &["steam"][..]),
        ("discord", &["discord"][..]),
        ("spotify", &["spotify", "声田"][..]),
    ];

    for (keyword, aliases) in alias_map {
        if app_name.contains(keyword) {
            patterns.extend(aliases.iter().map(|s| s.to_string()));
        }
    }

    patterns
}

/// 启动路径（支持 exe 或 lnk）
#[cfg(target_os = "windows")]
fn launch_path(path: &PathBuf) -> bool {
    // 直接用 path 启动，不要转字符串再拼接
    Command::new("cmd")
        .arg("/c")
        .arg("start")
        .arg("")  // 窗口标题（空，避免路径被误解析）
        .arg(path)  // 直接传 Path，让 Rust 处理引号和空格
        .spawn()
        .is_ok()
}

/// 获取当前用户名
#[cfg(target_os = "windows")]
fn get_username() -> String {
    std::env::var("USERNAME").unwrap_or_else(|_| "Public".to_string())
}
