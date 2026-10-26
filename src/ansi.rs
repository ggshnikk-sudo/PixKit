//! Цветной вывод, который работает везде:
//! - на Windows включает VT-режим через консольный API (по умолчанию там ANSI выключен);
//! - если stdout не терминал (редирект в файл/пайп) — цвета отключаются.
//! На Windows также сбрасываем кодировку вывода на UTF-8 (cmd по умолчанию cp866).

use std::io::IsTerminal;
use std::sync::OnceLock;

const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

static COLOR: OnceLock<bool> = OnceLock::new();

/// Включена ли поддержка ANSI-цветов (один раз на сессию).
pub fn color_enabled() -> bool {
    *COLOR.get_or_init(|| {
        if !std::io::stdout().is_terminal() {
            return false;
        }
        #[cfg(windows)]
        if !win::setup_console() {
            return false;
        }
        true
    })
}

fn wrap(color: &str, s: &str) -> String {
    if color_enabled() {
        format!("{color}{s}{RESET}")
    } else {
        s.to_string()
    }
}

pub fn ok(msg: &str) {
    println!("{}", wrap(GREEN, &format!("✔ {msg}")));
}

pub fn warn(msg: &str) {
    println!("{}", wrap(YELLOW, &format!("⚠ {msg}")));
}

pub fn err(msg: &str) {
    eprintln!("{}", wrap(RED, &format!("✖ {msg}")));
}

pub fn dim(msg: &str) -> String {
    wrap(DIM, msg)
}

pub fn bold(msg: &str) -> String {
    wrap(BOLD, msg)
}

#[cfg(windows)]
mod win {
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, SetConsoleMode, ENABLE_VIRTUAL_TERMINAL_PROCESSING,
        STD_OUTPUT_HANDLE,
    };

    pub fn setup_console() -> bool {
        // UTF-8 для русского текста (cmd по умолчанию cp866).
        unsafe {
            let _ = windows_sys::Win32::System::Console::SetConsoleOutputCP(65001);
        }
        unsafe {
            let h = GetStdHandle(STD_OUTPUT_HANDLE);
            if h.is_null() || h == -1isize as _ {
                return false;
            }
            let mut mode: u32 = 0;
            if GetConsoleMode(h, &mut mode) == 0 {
                return false;
            }
            SetConsoleMode(h, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) != 0
        }
    }
}