//! PixKit CLI — консольный режим (совместим с conv.py).
//!
//!   pixkit-cli convert in.png out.webp [--width N] [--height N] [--quality N]
//!   pixkit-cli stack   anim.gif out.png [--up|--down]
//!   pixkit-cli info    file.gif
//!
//! Цвета работают даже на Windows (включается VT-режим), без TTY — без цветов.

use std::process::ExitCode;

use pixkit::{ansi, ops};

struct Opts {
    width: Option<u32>,
    height: Option<u32>,
    quality: u8,
    order: ops::StackOrder,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            quality: 90,
            order: ops::StackOrder::Down,
        }
    }
}

fn parse_opts(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts::default();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        let next = || args.get(i + 1).cloned();
        match a {
            "--width" => {
                o.width = Some(next().ok_or("--width: нужен аргумент")?.parse().map_err(|_| "--width: число")?);
                i += 2;
            }
            "--height" => {
                o.height = Some(next().ok_or("--height: нужен аргумент")?.parse().map_err(|_| "--height: число")?);
                i += 2;
            }
            "--quality" => {
                let v: u8 = next().ok_or("--quality: нужен аргумент")?.parse().map_err(|_| "--quality: 1-100")?;
                o.quality = v.clamp(1, 100);
                i += 2;
            }
            "--up" => {
                o.order = ops::StackOrder::Up;
                i += 1;
            }
            "--down" => {
                o.order = ops::StackOrder::Down;
                i += 1;
            }
            other => return Err(format!("неизвестная опция: {other}")),
        }
    }
    Ok(o)
}

fn usage() {
    ansi::err("использование: pixkit-cli [convert <in> <out> [--width N] [--height N] [--quality N] | stack <in> <out> [--up|--down] | info <file>]");
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        usage();
        return ExitCode::from(1);
    }

    let cmd = args[0].as_str();
    let rest = &args[1..];

    match cmd {
        "convert" => {
            if rest.len() < 2 {
                usage();
                return ExitCode::from(1);
            }
            let (src, dst) = (&rest[0], &rest[1]);
            let opts = match parse_opts(&rest[2..]) {
                Ok(o) => o,
                Err(e) => {
                    ansi::err(&e);
                    return ExitCode::from(1);
                }
            };
            match ops::convert(src, dst, opts.width, opts.height, opts.quality) {
                Ok(res) => {
                    ansi::ok(&format!("→ {dst} ({res})"));
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    ansi::err(&e);
                    ExitCode::from(1)
                }
            }
        }
        "stack" => {
            if rest.len() < 2 {
                usage();
                return ExitCode::from(1);
            }
            let (src, dst) = (&rest[0], &rest[1]);
            let opts = match parse_opts(&rest[2..]) {
                Ok(o) => o,
                Err(e) => {
                    ansi::err(&e);
                    return ExitCode::from(1);
                }
            };
            match ops::stack(src, dst, opts.order) {
                Ok((n, w, h)) => {
                    let tag = match opts.order {
                        ops::StackOrder::Down => "down",
                        ops::StackOrder::Up => "up",
                    };
                    ansi::ok(&format!("→ {dst} ({n} кадров, кадр {w}×{h}, стек {w}×{}, order={tag})", h * n as u32));
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    ansi::err(&e);
                    ExitCode::from(1)
                }
            }
        }
        "info" => {
            if rest.is_empty() {
                usage();
                return ExitCode::from(1);
            }
            match ops::file_info(&rest[0]) {
                Ok(i) => {
                    if i.animated {
                        println!("{} {}x{} frames={} ~{:.1}fps", i.format, i.width, i.height, i.frames, i.fps);
                    } else {
                        println!("{} {}x{} frames={}", i.format, i.width, i.height, i.frames);
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    ansi::err(&e);
                    ExitCode::from(1)
                }
            }
        }
        _ => {
            usage();
            ExitCode::from(1)
        }
    }
}