//! 工具模块
//!
//! 所有工具都放在这个目录下，每个工具一个独立文件。
//!
//! # 新增工具步骤
//!
//! 1. 在本目录下新建文件，如 `my_tool.rs`
//! 2. 实现 `Tool` trait（参考 `system_time.rs`）
//! 3. 在本文件添加 `pub mod my_tool;`
//! 4. 在 `register_builtin_tools` 函数中注册

pub mod registry;
pub mod system_time;
pub mod system_status;
pub mod open_app;
pub mod run_command;
pub mod take_screenshot;
pub mod pet_control;

/// 导出公共类型
pub use registry::{ToolRegistry, ToolResult};

/// 注册所有内置工具
pub fn register_builtin_tools(registry: &mut ToolRegistry) {
    use open_app::OpenAppTool;
    use pet_control::PetControlTool;
    use run_command::RunCommandTool;
    use system_status::SystemStatusTool;
    use system_time::SystemTimeTool;
    use take_screenshot::TakeScreenshotTool;

    // 使用宏批量注册
    crate::register_tools!(
        registry,
        SystemTimeTool,      // 获取系统时间
        SystemStatusTool,    // 获取系统状态（CPU/内存/磁盘）
        OpenAppTool,         // 打开应用
        RunCommandTool,      // 执行命令（安全白名单）
        TakeScreenshotTool,  // 屏幕截图
        PetControlTool,      // 宠物行为控制
    );

    // 注册技能工具
    crate::skills::tool::register_skill_tools(registry);
}
