use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::ProcessStatus::{
    GetModuleBaseNameW, GetModuleFileNameExW,
};
use windows::Win32::System::Threading::{GetCurrentProcessId, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::UI::Input::KeyboardAndMouse::GetLastInputInfo;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
};

/// LASTINPUTINFO 结构体，用于 GetLastInputInfo
#[repr(C)]
struct LastInputInfo {
    cb_size: u32,
    dw_time: u32,
}

/// 获取系统级空闲时长（秒），基于 GetLastInputInfo（键盘/鼠标最后活动时间）
/// 这是检测用户是否真正处于「闲置」状态的可靠信号
/// 不依赖窗口标题变化，避免在看 PDF、IDE、播放视频时误判
pub fn get_system_idle_secs() -> u64 {
    use std::time::Duration;
    // SAFETY: LASTINPUTINFO 字段都已正确初始化；GetLastInputInfo 写入 cb_size 和 dw_time
    let mut info = LastInputInfo { cb_size: std::mem::size_of::<LastInputInfo>() as u32, dw_time: 0 };
    let ok = unsafe {
        // 将结构体转换为 windows crate 要求的类型
        let raw = windows::Win32::UI::Input::KeyboardAndMouse::LASTINPUTINFO {
            cbSize: info.cb_size,
            dwTime: info.dw_time,
        };
        let mut raw = raw;
        let result = GetLastInputInfo(&mut raw);
        info.dw_time = raw.dwTime;
        result.as_bool()
    };
    if !ok {
        return 0;
    }
    // GetTickCount 返回毫秒；GetLastInputInfo 返回的是相对 tick 数
    // 空闲毫秒 = 当前 tick - 最后输入 tick
    let now_tick = unsafe { windows::Win32::System::SystemInformation::GetTickCount() };
    let diff_ms = now_tick.wrapping_sub(info.dw_time);
    Duration::from_millis(diff_ms as u64).as_secs()
}

/// 进程信息结构
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub exe_path: String,
    pub window_title: String,
}

impl ProcessInfo {
    /// 获取进程可执行文件名（不含路径）
    pub fn exe_name(&self) -> String {
        Path::new(&self.exe_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&self.name)
            .to_string()
    }
}

/// 长路径 buffer 大小（NTFS 实际最大值），用于避免 MAX_PATH=260 截断现代 Windows 路径
const LONG_PATH_BUFFER_SIZE: usize = 32768;

/// 进程句柄的 RAII 包装，确保即使发生 panic 也能正确 CloseHandle
struct ProcessHandle(windows::Win32::Foundation::HANDLE);

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        // SAFETY: self.0 是 OpenProcess 成功返回的合法句柄；CloseHandle 对无效句柄返回 FALSE，但不会 panic
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// 获取当前前台窗口的进程信息
pub fn get_foreground_process() -> Option<ProcessInfo> {
    // SAFETY: GetForegroundWindow 没有前置条件；无前台窗口时返回 NULL
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0 == 0 {
        return None;
    }

    // SAFETY: title_buffer 是栈分配的 [u16; 512]，GetWindowTextW 最多写入 512 WCHAR
    let mut title_buffer = [0u16; 512];
    let title_len = unsafe { GetWindowTextW(hwnd, &mut title_buffer) };
    let window_title = if title_len > 0 {
        OsString::from_wide(&title_buffer[..title_len as usize])
            .to_string_lossy()
            .to_string()
    } else {
        String::new()
    };

    // SAFETY: pid 是合法的 u32 指针，GetWindowThreadProcessId 写入一个 DWORD
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };

    if pid == 0 {
        return None;
    }

    // 排除自身进程
    let current_pid = unsafe { GetCurrentProcessId() };
    if pid == current_pid {
        return None;
    }

    // 打开进程获取详细信息
    // SAFETY: OpenProcess 的参数都是合法值；失败返回的错误通过 Result 传递
    let process_handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false,
            pid,
        )
    };

    let handle = match process_handle {
        Ok(h) => ProcessHandle(h),
        Err(_) => {
            // 无法打开进程（可能是系统进程或权限不足）
            return Some(ProcessInfo {
                pid,
                name: format!("PID_{}", pid),
                exe_path: String::new(),
                window_title,
            });
        }
    };

    // 获取进程模块路径（使用 32768 大小的 buffer，支持长路径）
    // SAFETY: path_buffer 是合法指针，长度足够；GetModuleFileNameExW 写入的实际长度由返回值给出
    let mut path_buffer = vec![0u16; LONG_PATH_BUFFER_SIZE];
    let path_len = unsafe { GetModuleFileNameExW(handle.0, None, &mut path_buffer) };

    let exe_path = if path_len > 0 {
        OsString::from_wide(&path_buffer[..path_len as usize])
            .to_string_lossy()
            .to_string()
    } else {
        String::new()
    };

    // 获取进程名
    // SAFETY: name_buffer 同 path_buffer，长度足够
    let mut name_buffer = [0u16; 512];
    let name_len = unsafe { GetModuleBaseNameW(handle.0, None, &mut name_buffer) };

    let name = if name_len > 0 {
        OsString::from_wide(&name_buffer[..name_len as usize])
            .to_string_lossy()
            .to_string()
    } else {
        exe_path
            .split('\\')
            .last()
            .unwrap_or("unknown")
            .to_string()
    };

    // handle 通过 Drop 自动 CloseHandle，无需显式调用

    Some(ProcessInfo {
        pid,
        name,
        exe_path,
        window_title,
    })
}

/// 判断是否为系统进程（需要排除）
pub fn is_system_process(exe_path: &str) -> bool {
    if exe_path.is_empty() {
        return false;
    }

    let exe_lower = exe_path.to_lowercase();

    // 首先检查是否是已知的系统可执行文件（无论路径）
    if is_system_exe(&exe_lower) {
        return true;
    }

    // 动态获取 Windows 系统目录，避免硬编码 C 盘
    // SystemRoot 环境变量在 Windows 上始终存在且指向 Windows 安装目录
    // （如 C:\Windows、D:\Windows 等）。同时包含 System32 和 SysWOW64。
    let system_root = std::env::var("SystemRoot")
        .unwrap_or_else(|_| "C:\\Windows".to_string())
        .to_lowercase();

    let system_dirs = [
        format!("{}\\system32", system_root),
        format!("{}\\syswow64", system_root),
    ];

    // 检查是否在系统目录
    // 注意：使用目录分隔符 '\\' 拼接，避免误匹配 C:\Windows\SystemTools\xxx 这种用户路径
    for dir in &system_dirs {
        let prefix = format!("{}\\", dir);
        if exe_lower.starts_with(&prefix) {
            // 在系统目录下，检查是否是允许的白名单程序
            return !is_whitelisted_system_exe(&exe_lower);
        }
    }

    false
}

/// 系统白名单程序（允许显示）
fn is_whitelisted_system_exe(exe_lower: &str) -> bool {
    let whitelist = [
        "notepad.exe",
        "mspaint.exe",
        "calc.exe",
        "wordpad.exe",
    ];
    whitelist.iter().any(|exe| exe_lower.ends_with(exe))
}

/// 判断是否为特定的系统可执行文件
fn is_system_exe(exe_lower: &str) -> bool {
    let system_exes = [
        "svchost.exe",
        "services.exe",
        "lsass.exe",
        "csrss.exe",
        "smss.exe",
        "winlogon.exe",
        "wininit.exe",
        "dwm.exe",
        "taskhostw.exe",
        "sihost.exe",
        "explorer.exe", // 资源管理器虽然常用，但作为系统进程排除
        "searchindexer.exe",
        "searchui.exe",
        "runtimebroker.exe",
        "dllhost.exe",
        "backgroundtaskhost.exe",
        "conhost.exe",
        "cmd.exe", // 命令行工具排除
        "powershell.exe",
        "pwsh.exe",
    ];

    system_exes.iter().any(|exe| exe_lower.ends_with(exe))
}

/// 判断是否为应该被监控的用户应用程序
pub fn should_monitor(info: &ProcessInfo) -> bool {
    // 空路径不监控
    if info.exe_path.is_empty() {
        return false;
    }

    // 排除系统进程
    if is_system_process(&info.exe_path) {
        return false;
    }

    true
}

/// 获取所有用户进程的列表（用于初始化或统计）
pub fn enumerate_user_processes() -> Vec<ProcessInfo> {
    use sysinfo::{ProcessRefreshKind, RefreshKind, System};

    println!("[enumerate_user_processes] 开始获取进程列表...");
    let s = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::new()),
    );

    let all_count = s.processes().len();
    println!("[enumerate_user_processes] 系统总进程数: {}", all_count);

    let result: Vec<ProcessInfo> = s.processes()
        .iter()
        .filter_map(|(pid, process)| {
            let exe_path = process.exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            let name = process.name().to_string();

            // 排除空名称的进程
            if name.is_empty() {
                return None;
            }

            // 排除系统进程（检查路径和进程名）
            let path_to_check = if exe_path.is_empty() { &name } else { &exe_path };
            if is_system_process(path_to_check) {
                return None;
            }

            Some(ProcessInfo {
                pid: pid.as_u32(),
                name: name.clone(),
                exe_path: if exe_path.is_empty() { name.clone() } else { exe_path },
                window_title: String::new(), // sysinfo 无法直接获取窗口标题
            })
        })
        .collect();

    println!("[enumerate_user_processes] 过滤后进程数: {}", result.len());
    if !result.is_empty() {
        println!("[enumerate_user_processes] 前3个进程: {:?}", &result[..3.min(result.len())]);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_system_process() {
        assert!(is_system_process("C:\\Windows\\System32\\svchost.exe"));
        assert!(is_system_process("C:\\Windows\\explorer.exe"));
        assert!(!is_system_process("C:\\Program Files\\Chrome\\chrome.exe"));
        assert!(!is_system_process("C:\\Users\\Test\\AppData\\Local\\Discord\\app.exe"));
    }

    #[test]
    fn test_get_foreground_process() {
        // 运行时需要Windows环境
        let result = get_foreground_process();
        println!("Foreground process: {:?}", result);
    }
}
