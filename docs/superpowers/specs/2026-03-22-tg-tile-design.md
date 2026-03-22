# tg-tile: 텔레그램 창 자동 그리드 배열 유틸리티

## 목적

Windows에서 텔레그램 데스크톱의 팝아웃 채팅 창들을 핫키 한 번으로 그리드 배열하는 Rust CLI 유틸리티.

## 배경

텔레그램 봇을 여러 개 사용하면서 채팅창을 별도 창으로 팝아웃하면 창이 겹쳐서 관리가 어려움. 기존에 Tauri + TDLib로 패널형 클라이언트를 만들었으나 사용성이 떨어지고 고도화 비용이 높아, 네이티브 텔레그램 데스크톱 클라이언트를 그대로 활용하고 창 배치만 자동화하는 유틸리티로 전환.

## 핵심 동작 흐름

1. `tg-tile.exe` 실행 → 글로벌 핫키(`Win+Shift+G`) 등록 → 메시지 루프 대기
2. 핫키 입력 감지
3. `Telegram.exe` 프로세스의 보이는 최상위 윈도우 전부 탐색 (아래 "창 탐색 및 필터링" 참조)
4. 마우스 커서가 위치한 모니터의 작업 영역(work area, 태스크바 제외) 크기 획득 (`MonitorFromPoint` + `GetCursorPos`)
5. 창 개수에 맞는 최적 그리드 계산
6. 최소화된 창은 `ShowWindow(SW_RESTORE)`로 복원 후 `SetWindowPos`로 위치/크기 적용 (`SWP_NOZORDER | SWP_NOACTIVATE` 플래그 사용)
7. 콘솔에 결과 출력

## 그리드 계산 로직

창 개수(n)에 따라 자동 결정:

- `cols = ceil(sqrt(n))`
- `rows = ceil(n / cols)`
- 마지막 행에 빈 칸이 생기면 해당 행의 창들을 남은 공간에 균등 분배
- 창 간격(gap): 4px 고정

예시 (rows x cols 표기):

- n=1 → 1x1, 전체 화면
- n=2 → cols=2, rows=1 → 1x2 (좌/우 분할)
- n=3 → cols=2, rows=2 → 2x2 중 마지막 행 1개 → 하단 창이 전체 너비 차지
- n=4 → cols=2, rows=2 → 2x2
- n=5 → cols=3, rows=2 → 2x3 중 마지막 행 2개 → 하단 2개가 3칸 너비를 균등 분할
- n=6 → cols=3, rows=2 → 2x3
- n=7 → cols=3, rows=3 → 3x3 중 마지막 행 1개 → 하단 1개가 전체 너비 차지
- n=8 → cols=3, rows=3 → 3x3 중 마지막 행 2개 → 하단 2개가 3칸 너비를 균등 분할
- n=9 → cols=3, rows=3 → 3x3

## 창 탐색 및 필터링

`EnumWindows`로 최상위 윈도우를 순회하며 다음 조건을 모두 만족하는 창만 수집:

1. `IsWindowVisible` == true
2. `IsIconic` == false (최소화 상태는 `ShowWindow(SW_RESTORE)`로 복원 후 포함)
3. `GetWindowThreadProcessId`로 얻은 PID가 `Telegram.exe` 프로세스에 해당
4. `GetWindowLong(GWL_EXSTYLE)`에 `WS_EX_TOOLWINDOW` 플래그가 없음
5. `GetWindowTextW`로 얻은 타이틀이 비어있지 않음

창은 `EnumWindows`가 반환하는 Z-order 순서대로 그리드의 왼쪽 위부터 채워 배치.

## 실행 모드

- `tg-tile.exe` → 핫키 등록 + 메시지 루프 대기 (Ctrl+C로 종료)
- `tg-tile.exe --once` → 즉시 배열 1회 후 종료 (스크립트/숏컷 연동용)

## 핫키

- 기본값: `Win+Shift+G` (G = Grid)
- `RegisterHotKey` Win32 API 사용
- 핫키 충돌 시 에러 메시지 출력 후 종료

## 에러 처리

- 텔레그램 창 0개 → "열린 텔레그램 창이 없습니다" 출력, 아무 동작 안 함
- 텔레그램 창 1개 → 작업 영역 전체로 배치
- 이미 tg-tile 실행 중 → `CreateMutexW` 후 `GetLastError() == ERROR_ALREADY_EXISTS`로 판단, "이미 실행 중입니다" 출력 후 종료

## 콘솔 출력

```
[tg-tile] 핫키 등록: Win+Shift+G
[tg-tile] 대기 중... (Ctrl+C로 종료)
[tg-tile] 4개 창 배열 완료 (2x2)
```

## 기술 스택

- **언어:** Rust
- **의존성:** `windows-rs` (Win32 API 바인딩)
- **타겟:** x86_64-pc-windows-msvc

## 프로젝트 구조

```
tg-tile/
├── Cargo.toml
└── src/
    └── main.rs
```

단일 파일(`main.rs`)로 구현. 핫키 등록, 창 탐색, 그리드 계산, 배치를 각 함수로 분리하되 파일은 하나로 유지.

## 사용하는 Win32 API

- `RegisterHotKey` / `UnregisterHotKey` — 글로벌 핫키
- `GetMessage` — 메시지 루프
- `EnumWindows` — 모든 최상위 윈도우 순회
- `IsWindowVisible` / `IsIconic` — 보이는 창 필터링 / 최소화 상태 확인
- `GetWindowLong(GWL_EXSTYLE)` — WS_EX_TOOLWINDOW 등 확장 스타일 확인
- `GetWindowTextW` — 윈도우 타이틀 확인
- `ShowWindow(SW_RESTORE)` — 최소화된 창 복원
- `GetWindowThreadProcessId` — 프로세스 ID 확인
- `OpenProcess` / `QueryFullProcessImageNameW` — 프로세스명 확인 (Telegram.exe)
- `SetWindowPos` (`SWP_NOZORDER | SWP_NOACTIVATE`) — 창 위치/크기 설정
- `GetCursorPos` / `MonitorFromPoint` / `GetMonitorInfoW` — 마우스 커서 기준 모니터 작업 영역 획득
- `CreateMutexW` / `GetLastError` — 중복 실행 방지
- `SetConsoleCtrlHandler` — Ctrl+C 시 `UnregisterHotKey` 정리
- `SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)` — 고해상도 DPI 대응

## 스코프 외 (YAGNI)

- 설정 파일 / 핫키 커스터마이징
- 시스템 트레이 아이콘
- 멀티 모니터 분산 배치
- 레이아웃 프리셋
- GUI 설정 화면
