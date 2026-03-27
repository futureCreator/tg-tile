#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
use windows::{
    core::*,
    Win32::Foundation::*,
    Win32::Graphics::Gdi::*,
    Win32::System::Threading::*,
    Win32::UI::HiDpi::*,
    Win32::UI::Input::KeyboardAndMouse::*,
    Win32::UI::Shell::*,
    Win32::UI::WindowsAndMessaging::*,
};

#[cfg(windows)]
use std::ffi::OsString;
#[cfg(windows)]
use std::os::windows::ffi::OsStringExt;

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

    // Special case: 5 windows = 4 equal columns, 4th column split into 2 rows
    if n == 5 {
        let col_w = (work_w - gap * 3) / 4;
        let full_h = work_h;
        let half_h = (work_h - gap) / 2;

        return vec![
            Rect { x: work_x, y: work_y, w: col_w, h: full_h },
            Rect { x: work_x + (col_w + gap), y: work_y, w: col_w, h: full_h },
            Rect { x: work_x + (col_w + gap) * 2, y: work_y, w: col_w, h: full_h },
            Rect { x: work_x + (col_w + gap) * 3, y: work_y, w: col_w, h: half_h },
            Rect { x: work_x + (col_w + gap) * 3, y: work_y + half_h + gap, w: col_w, h: half_h },
        ];
    }

    let cols = if n <= 4 { n } else { (n as f64).sqrt().ceil() as usize };
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

    if !IsWindowVisible(hwnd).as_bool() {
        return TRUE;
    }

    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
    if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
        return TRUE;
    }

    let title_len = GetWindowTextLengthW(hwnd);
    if title_len == 0 {
        return TRUE;
    }

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
fn get_window_title(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len == 0 {
            return String::new();
        }
        let mut buf = vec![0u16; (len + 1) as usize];
        let copied = GetWindowTextW(hwnd, &mut buf);
        String::from_utf16_lossy(&buf[..copied as usize])
    }
}

#[cfg(windows)]
fn sort_main_window_last(hwnds: &mut Vec<HWND>) {
    if let Some(pos) = hwnds.iter().position(|&hwnd| get_window_title(hwnd) == "Telegram") {
        let main_hwnd = hwnds.remove(pos);
        hwnds.push(main_hwnd);
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
const WM_TRAY: u32 = WM_APP + 1;
#[cfg(windows)]
const HOTKEY_ID: i32 = 1;
#[cfg(windows)]
const IDM_TILE: usize = 1001;
#[cfg(windows)]
const IDM_EXIT: usize = 1002;

#[cfg(windows)]
fn tile_windows() {
    let hwnds = find_telegram_windows();

    if hwnds.is_empty() {
        return;
    }

    unsafe {
        for &hwnd in &hwnds {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
        }
    }

    let (work_x, work_y, work_w, work_h) = get_work_area();
    let rects = calculate_grid(hwnds.len(), work_x, work_y, work_w, work_h, GAP);

    unsafe {
        // First, gain foreground rights by activating any Telegram window
        if let Some(&first) = hwnds.first() {
            let _ = SetForegroundWindow(first);
        }

        // Now position and bring each window to top (reverse order so first ends up on top)
        for (hwnd, rect) in hwnds.iter().zip(rects.iter()).rev() {
            let _ = SetWindowPos(
                *hwnd,
                Some(HWND_TOP),
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                SWP_SHOWWINDOW,
            );
            let _ = BringWindowToTop(*hwnd);
        }
    }
}

#[cfg(windows)]
fn init_dpi() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
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
                    let _ = CloseHandle(h);
                    None
                } else {
                    Some(h)
                }
            }
            Err(_) => None,
        }
    }
}

#[cfg(windows)]
fn make_tip(s: &str) -> [u16; 128] {
    let mut tip = [0u16; 128];
    for (i, c) in s.encode_utf16().take(127).enumerate() {
        tip[i] = c;
    }
    tip
}

#[cfg(windows)]
const ICON_ICO: &[u8] = include_bytes!("../resources/tg-tile.ico");

#[cfg(windows)]
unsafe fn load_embedded_icon() -> HICON {
    // Parse ICO: offset at bytes 18..22, size at bytes 14..18 of first entry
    let offset = u32::from_le_bytes(ICON_ICO[18..22].try_into().unwrap()) as usize;
    let size = u32::from_le_bytes(ICON_ICO[14..18].try_into().unwrap()) as usize;
    CreateIconFromResourceEx(
        &ICON_ICO[offset..offset + size],
        true,
        0x00030000,
        16,
        16,
        LR_DEFAULTCOLOR,
    )
    .unwrap_or_else(|_| LoadIconW(None, IDI_APPLICATION).unwrap())
}

#[cfg(windows)]
unsafe fn add_tray_icon(hwnd: HWND) {
    let icon = load_embedded_icon();
    let nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
        uCallbackMessage: WM_TRAY,
        hIcon: icon,
        szTip: make_tip("tg-tile (Win+Shift+G)"),
        ..Default::default()
    };
    let _ = Shell_NotifyIconW(NIM_ADD, &nid);
}

#[cfg(windows)]
unsafe fn remove_tray_icon(hwnd: HWND) {
    let nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        ..Default::default()
    };
    let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
}

#[cfg(windows)]
unsafe fn show_context_menu(hwnd: HWND) {
    let Ok(menu) = CreatePopupMenu() else { return };
    let _ = AppendMenuW(menu, MF_STRING, IDM_TILE, w!("타일 정렬\tWin+Shift+G"));
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(menu, MF_STRING, IDM_EXIT, w!("종료"));

    let _ = SetForegroundWindow(hwnd);

    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    let _ = TrackPopupMenu(menu, TPM_RIGHTBUTTON, pt.x, pt.y, Some(0), hwnd, None);
    let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
    let _ = DestroyMenu(menu);
}

#[cfg(windows)]
unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_TRAY => {
            match lparam.0 as u32 {
                WM_RBUTTONUP => show_context_menu(hwnd),
                WM_LBUTTONDBLCLK => tile_windows(),
                _ => {}
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            match wparam.0 & 0xFFFF {
                IDM_TILE => tile_windows(),
                IDM_EXIT => {
                    remove_tray_icon(hwnd);
                    PostQuitMessage(0);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_HOTKEY => {
            tile_windows();
            LRESULT(0)
        }
        WM_DESTROY => {
            remove_tray_icon(hwnd);
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
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

    let _mutex = match ensure_single_instance() {
        Some(h) => h,
        None => return,
    };

    unsafe {
        let class_name = w!("TgTileClass");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassW(&wc);

        let Ok(hwnd) = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("tg-tile"),
            WINDOW_STYLE::default(),
            0, 0, 0, 0,
            None,
            None,
            None,
            None,
        ) else { return };

        add_tray_icon(hwnd);

        if RegisterHotKey(Some(hwnd), HOTKEY_ID, MOD_WIN | MOD_SHIFT | MOD_NOREPEAT, 0x47).is_err() {
            remove_tray_icon(hwnd);
            return;
        }

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let _ = UnregisterHotKey(Some(hwnd), HOTKEY_ID);
        remove_tray_icon(hwnd);
        let _ = CloseHandle(_mutex);
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
        let rects = calculate_grid(2, 0, 0, 1920, 1080, 4);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0], Rect { x: 0, y: 0, w: 958, h: 1080 });
        assert_eq!(rects[1], Rect { x: 962, y: 0, w: 958, h: 1080 });
    }

    #[test]
    fn test_grid_4_windows() {
        // 1 row x 4 cols
        let rects = calculate_grid(4, 0, 0, 1920, 1080, 4);
        assert_eq!(rects.len(), 4);
        let col_w = (1920 - 4 * 3) / 4; // 477
        assert_eq!(rects[0], Rect { x: 0, y: 0, w: col_w, h: 1080 });
        assert_eq!(rects[1], Rect { x: col_w + 4, y: 0, w: col_w, h: 1080 });
        assert_eq!(rects[2], Rect { x: (col_w + 4) * 2, y: 0, w: col_w, h: 1080 });
        assert_eq!(rects[3], Rect { x: (col_w + 4) * 3, y: 0, w: col_w, h: 1080 });
    }

    #[test]
    fn test_grid_3_windows_single_row() {
        // 1 row x 3 cols
        let rects = calculate_grid(3, 0, 0, 1920, 1080, 4);
        assert_eq!(rects.len(), 3);
        let col_w = (1920 - 4 * 2) / 3; // 637
        assert_eq!(rects[0], Rect { x: 0, y: 0, w: col_w, h: 1080 });
        assert_eq!(rects[1], Rect { x: col_w + 4, y: 0, w: col_w, h: 1080 });
        assert_eq!(rects[2], Rect { x: (col_w + 4) * 2, y: 0, w: col_w, h: 1080 });
    }

    #[test]
    fn test_grid_5_windows_4col_split() {
        // 4 equal columns, 4th column split into 2 rows
        // ┌────┬────┬────┬────┐
        // │    │    │    │ W4 │
        // │ W1 │ W2 │ W3 ├────┤
        // │    │    │    │ W5 │
        // └────┴────┴────┴────┘
        let rects = calculate_grid(5, 0, 0, 1920, 1080, 4);
        assert_eq!(rects.len(), 5);

        let col_w = (1920 - 4 * 3) / 4; // 477
        let full_h = 1080;
        let half_h = (1080 - 4) / 2; // 538

        // W1-W3: full height columns
        assert_eq!(rects[0], Rect { x: 0, y: 0, w: col_w, h: full_h });
        assert_eq!(rects[1], Rect { x: col_w + 4, y: 0, w: col_w, h: full_h });
        assert_eq!(rects[2], Rect { x: (col_w + 4) * 2, y: 0, w: col_w, h: full_h });

        // W4-W5: 4th column, split vertically
        assert_eq!(rects[3], Rect { x: (col_w + 4) * 3, y: 0, w: col_w, h: half_h });
        assert_eq!(rects[4], Rect { x: (col_w + 4) * 3, y: half_h + 4, w: col_w, h: half_h });
    }

    #[test]
    fn test_grid_with_offset() {
        let rects = calculate_grid(1, 100, 50, 1820, 1030, 4);
        assert_eq!(rects[0], Rect { x: 100, y: 50, w: 1820, h: 1030 });
    }
}
