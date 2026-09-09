/// Windows 平台单实例检测模块
///
/// 在 Tauri 初始化之前通过 Windows 命名互斥量 (Named Mutex) 检测是否已有实例运行。
/// 若检测到重复实例，通过 FindWindowW 查找已有主窗口并激活/聚焦，然后退出当前进程。
/// 这样第二个进程不会初始化任何 Tauri 资源（包括系统托盘图标）。

#[cfg(windows)]
mod win {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    // Windows API FFI 声明（仅引入所需的最小 API 集合）
    extern "system" {
        fn CreateMutexW(
            lp_mutex_attributes: *const std::ffi::c_void,
            b_initial_owner: i32,
            lp_name: *const u16,
        ) -> isize;
        fn GetLastError() -> u32;
        fn FindWindowW(lp_class_name: *const u16, lp_window_name: *const u16) -> isize;
        fn ShowWindow(h_wnd: isize, n_cmd_show: i32) -> i32;
        fn SetForegroundWindow(h_wnd: isize) -> i32;
        fn IsIconic(h_wnd: isize) -> i32;
    }

    const ERROR_ALREADY_EXISTS: u32 = 183;
    const SW_SHOW: i32 = 5;
    const SW_RESTORE: i32 = 9;

    /// 将 Rust 字符串转为 null-terminated UTF-16 宽字符数组
    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    /// 尝试获取单实例锁。
    /// - 返回 true：当前是唯一实例，可继续运行
    /// - 返回 false：已有实例运行，已激活其窗口，当前进程应退出
    pub fn try_acquire_single_instance() -> bool {
        // 使用应用标识符作为全局互斥量名称，避免与其他应用冲突
        let mutex_name = to_wide("Global\\com.skillsmanager.app.single-instance");

        unsafe {
            let handle = CreateMutexW(std::ptr::null(), 0, mutex_name.as_ptr());
            if handle == 0 || GetLastError() == ERROR_ALREADY_EXISTS {
                // 已有实例运行 —— 尝试查找并激活已有主窗口
                activate_existing_window();
                return false;
            }
            // 互斥量句柄不能释放，需保持到进程结束以维持锁定
            // handle 在进程退出时由 OS 自动回收
            true
        }
    }

    /// 通过窗口标题查找已有的 Skills Manager 主窗口并激活到前台
    fn activate_existing_window() {
        let title = to_wide("Skills Manager");
        unsafe {
            let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
            if hwnd != 0 {
                // 如果窗口处于最小化状态则恢复，否则直接显示
                if IsIconic(hwnd) != 0 {
                    ShowWindow(hwnd, SW_RESTORE);
                } else {
                    ShowWindow(hwnd, SW_SHOW);
                }
                SetForegroundWindow(hwnd);
            }
        }
    }
}

/// 单实例守卫入口。在 Tauri 初始化之前调用。
/// 若已有实例运行，激活其窗口并终止当前进程。
pub fn enforce_single_instance() {
    #[cfg(windows)]
    {
        if !win::try_acquire_single_instance() {
            std::process::exit(0);
        }
    }
}
