#[cfg(windows)]
use windows::{
    core::*,
    Win32::Foundation::*,
    Win32::Graphics::Gdi::*,
    Win32::System::Console::*,
    Win32::System::Threading::*,
    Win32::UI::HiDpi::*,
    Win32::UI::Input::KeyboardAndMouse::*,
    Win32::UI::WindowsAndMessaging::*,
};

#[cfg(windows)]
use std::ffi::OsString;
#[cfg(windows)]
use std::os::windows::ffi::OsStringExt;
#[cfg(windows)]
use std::sync::atomic::{AtomicBool, Ordering};

/// A rectangle representing a window position and size
#[derive(Debug, Clone, PartialEq)]
struct Rect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

/// Calculate grid positions for `n` windows within a work area.
/// `work_x, work_y`: top-left of work area
/// `work_w, work_h`: dimensions of work area
/// `gap`: spacing between windows in pixels
/// Returns a Vec of Rect, one per window, in order (left-to-right, top-to-bottom).
fn calculate_grid(n: usize, work_x: i32, work_y: i32, work_w: i32, work_h: i32, gap: i32) -> Vec<Rect> {
    if n == 0 {
        return vec![];
    }

    let cols = (n as f64).sqrt().ceil() as usize;
    let rows = (n as f64 / cols as f64).ceil() as usize;

    // Number of windows in the last row
    let last_row_count = n - cols * (rows - 1);

    let row_h = (work_h - gap * (rows as i32 - 1)) / rows as i32;

    let mut rects = Vec::with_capacity(n);

    for row in 0..rows {
        let is_last_row = row == rows - 1;
        let row_cols = if is_last_row { last_row_count } else { cols };
        let col_w = (work_w - gap * (row_cols as i32 - 1)) / row_cols as i32;

        for col in 0..row_cols {
            let x = work_x + col as i32 * (col_w + gap);
            let y = work_y + row as i32 * (row_h + gap);
            rects.push(Rect { x, y, w: col_w, h: row_h });
        }
    }

    rects
}

#[cfg(windows)]
fn find_telegram_windows() -> Vec<HWND> {
    let mut windows: Vec<HWND> = Vec::new();

    unsafe {
        let _ = EnumWindows(
            Some(enum_window_proc),
            LPARAM(&mut windows as *mut Vec<HWND> as isize),
        );
    }

    windows
}

#[cfg(windows)]
unsafe extern "system" fn enum_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows = &mut *(lparam.0 as *mut Vec<HWND>);

    // Must be visible
    if !IsWindowVisible(hwnd).as_bool() {
        return TRUE;
    }

    // Check extended style: skip tool windows
    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
    if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
        return TRUE;
    }

    // Must have a non-empty title
    let title_len = GetWindowTextLengthW(hwnd);
    if title_len == 0 {
        return TRUE;
    }

    // Check if this window belongs to Telegram.exe
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));

    if pid == 0 {
        return TRUE;
    }

    if is_telegram_process(pid) {
        windows.push(hwnd);
    }

    TRUE
}

#[cfg(windows)]
fn is_telegram_process(pid: u32) -> bool {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);
        let Ok(handle) = handle else {
            return false;
        };

        let mut buf = [0u16; 260];
        let mut size = buf.len() as u32;
        let result = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut size);
        let _ = CloseHandle(handle);

        if result.is_err() {
            return false;
        }

        let name = OsString::from_wide(&buf[..size as usize]);
        let name = name.to_string_lossy().to_lowercase();
        name.ends_with("telegram.exe")
    }
}

#[cfg(windows)]
fn get_work_area() -> (i32, i32, i32, i32) {
    unsafe {
        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);

        let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTOPRIMARY);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = GetMonitorInfoW(monitor, &mut info);

        let rc = info.rcWork;
        (rc.left, rc.top, rc.right - rc.left, rc.bottom - rc.top)
    }
}

const GAP: i32 = 4;

#[cfg(windows)]
fn tile_windows() {
    let hwnds = find_telegram_windows();

    if hwnds.is_empty() {
        println!("[tg-tile] 열린 텔레그램 창이 없습니다");
        return;
    }

    // Restore minimized windows
    unsafe {
        for &hwnd in &hwnds {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
        }
    }

    let (work_x, work_y, work_w, work_h) = get_work_area();
    let rects = calculate_grid(hwnds.len(), work_x, work_y, work_w, work_h, GAP);

    let cols = (hwnds.len() as f64).sqrt().ceil() as usize;
    let rows = (hwnds.len() as f64 / cols as f64).ceil() as usize;

    unsafe {
        for (hwnd, rect) in hwnds.iter().zip(rects.iter()) {
            let _ = SetWindowPos(
                *hwnd,
                HWND::default(),
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }

    println!(
        "[tg-tile] {}개 창 배열 완료 ({}x{})",
        hwnds.len(),
        rows,
        cols,
    );
}

#[cfg(windows)]
fn init_dpi() {
    unsafe {
        if SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2).is_err() {
            eprintln!("[tg-tile] 경고: DPI 인식 설정 실패 (Windows 10 1703 이상 필요)");
        }
    }
}

#[cfg(windows)]
fn ensure_single_instance() -> Option<HANDLE> {
    unsafe {
        let mutex_name = w!("Global\\TgTileMutex");
        let handle = CreateMutexW(None, false, mutex_name);

        match handle {
            Ok(h) => {
                if GetLastError() == ERROR_ALREADY_EXISTS {
                    println!("[tg-tile] 이미 실행 중입니다");
                    let _ = CloseHandle(h);
                    None
                } else {
                    Some(h)
                }
            }
            Err(_) => {
                println!("[tg-tile] Mutex 생성 실패");
                None
            }
        }
    }
}

#[cfg(windows)]
static RUNNING: AtomicBool = AtomicBool::new(true);

#[cfg(windows)]
unsafe extern "system" fn ctrl_handler(ctrl_type: u32) -> BOOL {
    if ctrl_type == CTRL_C_EVENT || ctrl_type == CTRL_CLOSE_EVENT {
        RUNNING.store(false, Ordering::SeqCst);
        PostQuitMessage(0);
        TRUE
    } else {
        FALSE
    }
}

#[cfg(windows)]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let once_mode = args.iter().any(|a| a == "--once");

    init_dpi();

    if once_mode {
        tile_windows();
        return;
    }

    // Single instance check (only for daemon mode)
    let _mutex = match ensure_single_instance() {
        Some(h) => h,
        None => return,
    };

    unsafe {
        // Register Ctrl+C handler
        let _ = SetConsoleCtrlHandler(Some(ctrl_handler), true);

        // Register hotkey: Win+Shift+G
        let hotkey_id = 1;
        let result = RegisterHotKey(
            None,
            hotkey_id,
            MOD_WIN | MOD_SHIFT | MOD_NOREPEAT,
            0x47, // 'G' virtual key code
        );

        if result.is_err() {
            println!("[tg-tile] 핫키 충돌: Win+Shift+G가 이미 사용 중입니다");
            return;
        }

        println!("[tg-tile] 핫키 등록: Win+Shift+G");
        println!("[tg-tile] 대기 중... (Ctrl+C로 종료)");

        // Message loop
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            if msg.message == WM_HOTKEY && msg.wParam.0 == hotkey_id as usize {
                tile_windows();
            }
        }

        // Cleanup
        let _ = UnregisterHotKey(None, hotkey_id);
        let _ = CloseHandle(_mutex);
        println!("\n[tg-tile] 종료");
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("[tg-tile] 이 프로그램은 Windows에서만 실행 가능합니다.");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_0_windows() {
        let rects = calculate_grid(0, 0, 0, 1920, 1080, 4);
        assert_eq!(rects.len(), 0);
    }

    #[test]
    fn test_grid_1_window() {
        let rects = calculate_grid(1, 0, 0, 1920, 1080, 4);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0], Rect { x: 0, y: 0, w: 1920, h: 1080 });
    }

    #[test]
    fn test_grid_2_windows() {
        // 1 row x 2 cols
        let rects = calculate_grid(2, 0, 0, 1920, 1080, 4);
        assert_eq!(rects.len(), 2);
        // Left half: 0..958, right half: 962..1920
        assert_eq!(rects[0], Rect { x: 0, y: 0, w: 958, h: 1080 });
        assert_eq!(rects[1], Rect { x: 962, y: 0, w: 958, h: 1080 });
    }

    #[test]
    fn test_grid_4_windows() {
        // 2x2 grid
        let rects = calculate_grid(4, 0, 0, 1920, 1080, 4);
        assert_eq!(rects.len(), 4);
        assert_eq!(rects[0], Rect { x: 0, y: 0, w: 958, h: 538 });
        assert_eq!(rects[1], Rect { x: 962, y: 0, w: 958, h: 538 });
        assert_eq!(rects[2], Rect { x: 0, y: 542, w: 958, h: 538 });
        assert_eq!(rects[3], Rect { x: 962, y: 542, w: 958, h: 538 });
    }

    #[test]
    fn test_grid_3_windows_last_row_fills() {
        // 2 rows x 2 cols, last row has 1 window → takes full width
        let rects = calculate_grid(3, 0, 0, 1920, 1080, 4);
        assert_eq!(rects.len(), 3);
        // Top row: 2 windows, each 958 wide
        assert_eq!(rects[0], Rect { x: 0, y: 0, w: 958, h: 538 });
        assert_eq!(rects[1], Rect { x: 962, y: 0, w: 958, h: 538 });
        // Bottom row: 1 window, full width
        assert_eq!(rects[2], Rect { x: 0, y: 542, w: 1920, h: 538 });
    }

    #[test]
    fn test_grid_5_windows_last_row_2_of_3() {
        // 2 rows x 3 cols, last row has 2 windows splitting 3 cols worth of space
        let rects = calculate_grid(5, 0, 0, 1920, 1080, 4);
        assert_eq!(rects.len(), 5);
        // Top row: 3 windows
        let col_w = (1920 - 4 * 2) / 3; // (1920 - 8) / 3 = 637
        assert_eq!(rects[0].w, col_w);
        assert_eq!(rects[1].w, col_w);
        assert_eq!(rects[2].w, col_w);
        // Bottom row: 2 windows splitting full width
        let bottom_w = (1920 - 4) / 2; // 958
        assert_eq!(rects[3].w, bottom_w);
        assert_eq!(rects[4].w, bottom_w);
    }

    #[test]
    fn test_grid_with_offset() {
        // Work area starting at (100, 50)
        let rects = calculate_grid(1, 100, 50, 1820, 1030, 4);
        assert_eq!(rects[0], Rect { x: 100, y: 50, w: 1820, h: 1030 });
    }
}
