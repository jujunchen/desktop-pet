# 工具开发指南

本目录是桌面宠物的LLM工具模块，采用 **模块化 + Trait-based** 架构设计。

## 📁 目录结构

```
tools/
├── README.md              # 本文档
├── mod.rs                 # 模块导出 + 工具注册入口
├── registry.rs            # 工具Trait定义 + 注册表核心
├── system_time.rs         # 示例：系统时间工具
│
│  === 新增工具放在这里 === ↓
│
├── system_status.rs       # 示例：系统状态查询（CPU/内存/磁盘）
├── open_app.rs            # 示例：打开应用
├── pet_control.rs         # 示例：宠物行为控制
└── ...                    # 更多工具...
```

---

## 🚀 3步开发一个新工具

### 第1步：新建工具文件

创建 `your_tool.rs` 文件，参考模板实现 `Tool` Trait：

```rust
//! 工具功能描述

use super::registry::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

/// 工具结构体（可以包含内部状态）
pub struct YourTool;

#[async_trait]
impl Tool for YourTool {
    /// 【必填】工具唯一名称
    /// 给LLM调用时用，用下划线命名法
    fn name(&self) -> &str {
        "your_tool_name"
    }

    /// 【必填】工具描述
    /// 告诉LLM这个工具是做什么的，什么场景下使用
    fn description(&self) -> &str {
        "工具的详细描述，让LLM知道什么时候该调用这个工具"
    }

    /// 【必填】参数定义（JSON Schema格式）
    /// 定义工具需要接收什么参数
    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "param1": {
                    "type": "string",
                    "description": "参数1的说明"
                },
                "param2": {
                    "type": "number",
                    "description": "参数2的说明"
                }
            },
            "required": ["param1"]  // 必填参数
        })
    }

    /// 【必填】执行工具
    /// 实现具体的业务逻辑
    async fn execute(&self, args: Value) -> ToolResult {
        // 1. 解析参数
        let param1 = args
            .get("param1")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "缺少参数 param1".to_string())?;

        // 2. 具体实现
        // ...你的代码...

        // 3. 返回结果
        // Ok(成功结果字符串) 或 Err(错误信息字符串)
        Ok(format!("执行成功，参数：{}", param1))
    }
}
```

---

### 第2步：导出模块

在 `mod.rs` 中添加模块声明：

```rust
// src-tauri/src/llm/tools/mod.rs

pub mod registry;
pub mod system_time;
pub mod your_tool;      // ← 新增这一行
```

---

### 第3步：注册工具

在 `mod.rs` 的 `register_builtin_tools` 函数中注册：

```rust
// src-tauri/src/llm/tools/mod.rs

pub fn register_builtin_tools(registry: &mut ToolRegistry) {
    use system_time::SystemTimeTool;
    use your_tool::YourTool;      // ← 新增导入

    crate::register_tools!(
        registry,
        SystemTimeTool,
        YourTool,                  // ← 新增注册
    );
}
```

**完成！** 工具会自动被ReAct引擎发现和使用。

---

## 🎯 完整示例：「打开应用」工具

```rust
//! 打开系统应用工具

use super::registry::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::process::Command;

pub struct OpenAppTool;

#[async_trait]
impl Tool for OpenAppTool {
    fn name(&self) -> &str {
        "open_app"
    }

    fn description(&self) -> &str {
        "打开电脑上的应用程序，如记事本、计算器、浏览器等。
        当用户说'打开XXX'、'启动XXX'、'运行XXX'时使用。"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "app_name": {
                    "type": "string",
                    "description": "应用名称，如：notepad, calc, chrome, explorer"
                }
            },
            "required": ["app_name"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let app_name = args
            .get("app_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "缺少 app_name 参数".to_string())?;

        #[cfg(target_os = "windows")]
        {
            let result = match app_name.to_lowercase().as_str() {
                "notepad" | "记事本" => Command::new("notepad.exe").spawn(),
                "calc" | "计算器" => Command::new("calc.exe").spawn(),
                "explorer" | "资源管理器" => Command::new("explorer.exe").spawn(),
                "chrome" | "浏览器" => Command::new("cmd").args(["/c", "start", "chrome.exe"]).spawn(),
                _ => return Err(format!("不支持的应用: {}", app_name)),
            };

            result.map_err(|e| format!("启动失败: {}", e))?;
        }

        #[cfg(not(target_os = "windows"))]
        {
            return Err("仅支持Windows系统".to_string());
        }

        Ok(format!("已成功打开: {}", app_name))
    }
}
```

---

## 💡 开发最佳实践

### 1. 参数定义要详细

JSON Schema 写得越详细，LLM调用越准确：

```rust
// ✅ 好的写法：详细描述参数和枚举值
serde_json::json!({
    "type": "object",
    "properties": {
        "format": {
            "type": "string",
            "enum": ["full", "date", "time"],
            "default": "full",
            "description": "返回格式：full=完整日期时间，date=仅日期，time=仅时间"
        }
    }
})
```

### 2. 描述要包含触发场景

告诉LLM **什么时候** 应该调用这个工具：

```rust
// ✅ 好的写法
fn description(&self) -> &str {
    "获取当前系统时间。当用户问'现在几点'、'今天几号'、'什么时间'时使用。"
}

// ❌ 不好的写法
fn description(&self) -> &str {
    "获取系统时间"
}
```

### 3. 错误信息要友好

```rust
// ✅ 好的错误信息
return Err(format!(
    "无法打开应用 '{}'，支持的应用有: notepad, calc, chrome",
    app_name
));

// ❌ 不好的错误信息
return Err("错误".to_string());
```

### 4. 跨平台兼容

使用 `#[cfg]` 处理不同操作系统：

```rust
#[cfg(target_os = "windows")]
{
    // Windows 实现
}

#[cfg(target_os = "macos")]
{
    // macOS 实现
}

#[cfg(target_os = "linux")]
{
    // Linux 实现
}
```

### 5. 异步操作

`execute` 方法是 async 的，可以直接调用异步API：

```rust
async fn execute(&self, args: Value) -> ToolResult {
    // 直接 await 异步函数
    let response = reqwest::get("https://api.example.com").await?;
    Ok(response.text().await?)
}
```

---

## 🔧 工具注册表 API

### `ToolRegistry` 公共方法

```rust
// 创建注册表
let mut registry = ToolRegistry::new();

// 注册工具
registry.register(Box::new(MyTool));

// 注销工具
registry.unregister("my_tool");

// 获取工具
let tool = registry.get("my_tool");

// 获取所有工具描述（给LLM用）
let tool_list = registry.list();  // 返回 Vec<ToolInfo>

// 检查工具是否存在
if registry.has("my_tool") { ... }

// 获取工具数量
let count = registry.len();
```

### `register_tools!` 宏

批量注册工具：

```rust
register_tools!(
    registry,
    SystemTimeTool,
    SystemStatusTool,
    OpenAppTool,
    PetControlTool,
);
```

---

## 🧪 测试工具

### 单独测试工具

```rust
#[tokio::test]
async fn test_system_time_tool() {
    let tool = SystemTimeTool;
    let args = serde_json::json!({"format": "time"});
    let result = tool.execute(args).await;
    assert!(result.is_ok());
}
```

### 完整对话测试

运行应用后测试：

```
用户: 现在几点了？
小白: （调用 get_system_time 工具）
小白: 现在是 14:30:25 哦～汪汪！
```

---

## 📋 常用工具开发清单

### 系统类
- [x] `get_system_time` - 系统时间
- [x] `get_system_status` - CPU/内存/磁盘使用率
- [x] `take_screenshot` - 屏幕截图

### 应用类
- [x] `open_app` - 打开应用程序（记事本、浏览器等）
- [x] `run_command` - 执行命令（安全白名单）

### 宠物控制类
- [x] `pet_control` - 宠物行为控制（说话、表情、移动、跳舞等）
- [ ] `pet_animation` - 切换宠物动画
- [ ] `pet_move` - 控制宠物位置移动（接入Tauri窗口API）

---

## ❓ 常见问题

### Q: 为什么用 async-trait？
A: Rust原生不支持trait中的async fn，`async-trait` 宏让我们可以在trait中定义异步方法。

### Q: 工具可以有内部状态吗？
A: 可以！工具结构体可以包含字段。但要注意需要线程安全（Send + Sync）。

```rust
pub struct StatefulTool {
    counter: Mutex<i32>,
}

impl StatefulTool {
    pub fn new() -> Self {
        Self { counter: Mutex::new(0) }
    }
}
```

### Q: 工具可以访问Tauri状态吗？
A: 目前需要通过全局变量或channel，后续会集成Tauri状态注入。

### Q: 新增工具需要重新编译吗？
A: 是的，Rust是编译型语言，修改代码后需要重新编译。

---

## 📚 参考链接

- [JSON Schema 参考](https://json-schema.org/learn/getting-started-step-by-step)
- [async-trait 文档](https://docs.rs/async-trait)
- [serde_json 文档](https://docs.rs/serde_json)
