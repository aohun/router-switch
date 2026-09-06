// Release 构建隐藏 Windows 控制台窗口；Debug 构建保留以便在终端查看日志输出
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let _guard = session::tokio_runtime().enter();
    ui::run();
}
