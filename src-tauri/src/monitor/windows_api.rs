use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use windows::Win32::Foundation::{CloseHandle, HWND, MAX_PATH};
use windows::Win32::System::ProcessStatus::{
    GetModuleBaseNameW, GetModuleFileNameExW,
};
use windows::Win32::System::Threading::{GetCurrentProcessId, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
};

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

/// 获取当前前台窗口的进程信息
pub fn get_foreground_process() -> Option<ProcessInfo> {
    unsafe {
        // 获取前台窗口句柄
        let hwnd = GetForegroundWindow();
        if hwnd.0 == 0 {
            return None;
        }

        // 获取窗口标题
        let mut title_buffer = [0u16; 512];
        let title_len = GetWindowTextW(hwnd, &mut title_buffer);
        let window_title = if title_len > 0 {
            OsString::from_wide(&title_buffer[..title_len as usize])
                .to_string_lossy()
                .to_string()
        } else {
            String::new()
        };

        // 获取窗口所属的进程ID
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));

        if pid == 0 {
            return None;
        }

        // 排除自身进程
        if pid == GetCurrentProcessId() {
            return None;
        }

        // 打开进程获取详细信息
        let process_handle = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false,
            pid,
        );

        if process_handle.is_err() {
            // 无法打开进程（可能是系统进程或权限不足）
            return Some(ProcessInfo {
                pid,
                name: format!("PID_{}", pid),
                exe_path: String::new(),
                window_title,
            });
        }

        let handle = process_handle.unwrap();

        // 获取进程模块路径
        let mut path_buffer = [0u16; MAX_PATH as usize];
        let path_len = GetModuleFileNameExW(handle, None, &mut path_buffer);

        let exe_path = if path_len > 0 {
            OsString::from_wide(&path_buffer[..path_len as usize])
                .to_string_lossy()
                .to_string()
        } else {
            String::new()
        };

        // 获取进程名
        let mut name_buffer = [0u16; 512];
        let name_len = GetModuleBaseNameW(handle, None, &mut name_buffer);

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

        let _ = CloseHandle(handle);

        Some(ProcessInfo {
            pid,
            name,
            exe_path,
            window_title,
        })
    }
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

    // 系统目录列表 - 这些目录下的非白名单程序也排除
    let system_dirs = [
        "c:\\windows\\system32",
        "c:\\windows\\syswow64",
    ];

    // 检查是否在系统目录
    for dir in &system_dirs {
        if exe_lower.starts_with(dir) {
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
