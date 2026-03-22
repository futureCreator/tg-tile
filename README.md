# tg-tile

Telegram Desktop 멀티 계정 창을 자동으로 그리드 배열하는 Windows 유틸리티.

## Features

- Telegram Desktop 창 자동 탐지 (프로세스명 기반)
- N개 창을 모니터 작업 영역에 맞춰 그리드 배열
- `Win+Shift+G` 글로벌 핫키로 즉시 타일링
- 시스템 트레이 상주 (우클릭 메뉴 / 더블클릭 타일링)
- Per-Monitor DPI 인식 지원
- 단일 인스턴스 보장 (Mutex)

## Usage

```
cargo build --release
tg-tile.exe
```

핫키 `Win+Shift+G`를 누르면 열려있는 Telegram Desktop 창들이 자동 그리드 배열됩니다.

## Changelog

### v0.1.2 - 2026-03-22
- feat: 시스템 트레이 지원 (콘솔 창 없이 백그라운드 실행)
- feat: 3개 창 배열 시 2x2 대신 1행 3열 사용
- fix: `TrackPopupMenu` API `Option<i32>` 타입 수정

### v0.1.1 - 2026-03-22
- fix: `SetWindowPos` API 호출 시 `Option<HWND>` 타입 불일치 수정
- fix: `CreateMutexW` 사용을 위한 `Win32_Security` feature 추가

### v0.1.0 - 2026-03-22
- feat: Win32 창 열거, 타일링, 핫키, DPI 지원
- feat: 그리드 계산 로직 및 테스트
- chore: tg-tile Rust 프로젝트 스캐폴딩
- docs: 설계 문서 및 구현 계획
