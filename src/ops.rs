//! PixKit — операции с изображениями: чтение кадров, конвертация, стек, инфо.
//!
//! Полностью на чистом Rust (image/png), без системных зависимостей,
//! так что работает одинаково на Linux/macOS/Windows.

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::time::Duration;

use image::codecs::gif::GifDecoder;
use image::codecs::png::PngDecoder;
use image::codecs::webp::WebPDecoder;
use image::{AnimationDecoder, DynamicImage, ExtendedColorType, ImageFormat, RgbaImage};

pub const ANIM_OUT: &[&str] = &["gif", "apng"];

/// Один кадр: RGBA-буфер + задержка до следующего (мс, 0/отсутствие — 100).
#[derive(Clone)]
pub struct FrameData {
    pub rgba: RgbaImage,
    pub delay_ms: u32,
}

// ────────────────────────── загрузка кадров ──────────────────────────

fn default_delay(d: Duration) -> u32 {
    let ms = d.as_millis().min(u32::MAX as u128) as u32;
    if ms == 0 { 100 } else { ms }
}

/// Читает файл как последовательность кадров (RGBA).
/// GIF/APNG/WebP-анимации дают все кадры; статика — один кадр.
pub fn load_frames(path: &str) -> Result<Vec<FrameData>, String> {
    let src_file = File::open(path).map_err(|e| format!("нет файла {path}: {e}"))?;
    let probe = BufReader::new(File::open(path).map_err(|e| format!("нет файла {path}: {e}"))?);
    let fmt = image::ImageReader::new(probe)
        .with_guessed_format()
        .map_err(|_| "неизвестный формат файла")?
        .format()
        .ok_or("неизвестный формат файла")?;

    match fmt {
        ImageFormat::Gif => frames_from_anim(GifDecoder::new(BufReader::new(src_file)).map_err(|e| e.to_string())?),
        ImageFormat::Png => png_frames(path, src_file),
        ImageFormat::WebP => webp_frames(src_file),
        _ => {
            // Статичный формат: decode первого кадра.
            let img = decode_static(path)?;
            Ok(vec![FrameData { rgba: img, delay_ms: 0 }])
        }
    }
}

fn frames_from_anim<'a, D: AnimationDecoder<'a>>(dec: D) -> Result<Vec<FrameData>, String> {
    let frames = dec
        .into_frames()
        .collect_frames()
        .map_err(|e| format!("не удалось прочитать кадры: {e}"))?;
    Ok(frames
        .into_iter()
        .map(|f| {
            let delay_ms = default_delay(Duration::from(f.delay()));
            FrameData { rgba: f.into_buffer(), delay_ms }
        })
        .collect())
}

fn png_frames(path: &str, file: File) -> Result<Vec<FrameData>, String> {
    let dec = PngDecoder::new(BufReader::new(file)).map_err(|e| e.to_string())?;
    if dec.is_apng().unwrap_or(false) {
        let apng = dec.apng().map_err(|e| e.to_string())?;
        return frames_from_anim(apng);
    }
    let img = decode_static(path)?;
    Ok(vec![FrameData { rgba: img, delay_ms: 0 }])
}

fn webp_frames(file: File) -> Result<Vec<FrameData>, String> {
    let dec = WebPDecoder::new(BufReader::new(file)).map_err(|e| e.to_string())?;
    // `into_frames` отдаёт все кадры анимированного webp и 1 кадр для статики.
    frames_from_anim(dec)
}

/// Статичный кадр (для превью) — только первый кадр, без декодирования всей анимации.
pub fn preview_frame(path: &str) -> Result<RgbaImage, String> {
    Ok(decode_static(path)?)
}

fn decode_static(path: &str) -> Result<RgbaImage, String> {
    let img = image::ImageReader::open(path)
        .map_err(|e| format!("нет файла {path}: {e}"))?
        .with_guessed_format()
        .map_err(|_| "неизвестный формат файла")?
        .decode()
        .map_err(|e| format!("не удалось декодировать {path}: {e}"))?
        .to_rgba8();
    Ok(img)
}

// ────────────────────────── ресайз ──────────────────────────

fn resize_dim(w: u32, h: u32, width: Option<u32>, height: Option<u32>) -> (u32, u32) {
    if let (Some(w2), Some(h2)) = (width, height) {
        return (w2.max(1), h2.max(1));
    }
    if let Some(w2) = width {
        let h2 = (u64::from(h) * u64::from(w2) / u64::from(w)).clamp(1, u32::MAX as u64) as u32;
        return (w2, h2);
    }
    if let Some(h2) = height {
        let w2 = (u64::from(w) * u64::from(h2) / u64::from(h)).clamp(1, u32::MAX as u64) as u32;
        return (w2, h2);
    }
    (w, h)
}

fn resize_rgba(img: &RgbaImage, width: Option<u32>, height: Option<u32>) -> RgbaImage {
    let (w, h) = resize_dim(img.width(), img.height(), width, height);
    if (w, h) == (img.width(), img.height()) {
        return img.clone();
    }
    DynamicImage::ImageRgba8(img.clone())
        .resize(w, h, image::imageops::FilterType::Lanczos3)
        .to_rgba8()
}

// ────────────────────────── конвертация ──────────────────────────

/// Конвертация: статика между форматами, анимации — в GIF/APNG.
/// Возвращает строку-описание результата (как в conv.py).
pub fn convert(
    src: &str,
    dst: &str,
    width: Option<u32>,
    height: Option<u32>,
    quality: u8,
) -> Result<String, String> {
    let frames = load_frames(src)?;
    let ext = Path::new(dst)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let ext = ext.as_str();

    if frames.len() > 1 && ANIM_OUT.contains(&ext) {
        let frames: Vec<FrameData> = frames
            .into_iter()
            .map(|f| FrameData { rgba: resize_rgba(&f.rgba, width, height), delay_ms: f.delay_ms })
            .collect();
        let (w, h) = (frames[0].rgba.width(), frames[0].rgba.height());
        if ext == "gif" {
            write_gif(&frames, dst)?;
        } else {
            write_apng(&frames, dst)?;
        }
        return Ok(format!("{w}×{h}, {} кадров", frames.len()));
    }

    // Статика: берём первый кадр (для анимаций в png/jpg/webp — первый кадр).
    let img = resize_rgba(&frames[0].rgba, width, height);
    save_still(&img, dst, ext, quality)?;
    Ok(format!("{}×{}", img.width(), img.height()))
}

fn write_gif(frames: &[FrameData], dst: &str) -> Result<(), String> {
    let file = File::create(dst).map_err(|e| format!("не могу создать {dst}: {e}"))?;
    let mut enc = image::codecs::gif::GifEncoder::new(BufWriter::new(file));
    enc.set_repeat(image::codecs::gif::Repeat::Infinite)
        .map_err(|e| e.to_string())?;
    for f in frames {
        let delay = Duration::from_millis(f.delay_ms.clamp(1, u32::MAX as u32) as u64);
        let frame = image::Frame::from_parts(
            f.rgba.clone(),
            0,
            0,
            image::Delay::from_saturating_duration(delay),
        );
        enc.encode_frame(frame).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn write_apng(frames: &[FrameData], dst: &str) -> Result<(), String> {
    let (w, h) = (frames[0].rgba.width(), frames[0].rgba.height());
    let file = File::create(dst).map_err(|e| format!("не могу создать {dst}: {e}"))?;
    let mut enc = png::Encoder::new(file, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_animated(frames.len() as u32, 0)
        .map_err(|e| e.to_string())?;
    // Первый кадр — часть анимации (как default_image=True в Pillow).
    enc.set_sep_def_img(false).map_err(|e| e.to_string())?;
    let mut wtr = enc.write_header().map_err(|e| e.to_string())?;
    for f in frames {
        let num = f.delay_ms.clamp(1, u16::MAX as u32) as u16;
        // задержка = num/1000 секунды; заодно и на кадр пишем.
        wtr.set_frame_delay(num, 1000).map_err(|e| e.to_string())?;
        wtr.write_image_data(&f.rgba).map_err(|e| e.to_string())?;
    }
    wtr.finish().map_err(|e| e.to_string())?;
    Ok(())
}

fn save_still(img: &RgbaImage, dst: &str, ext: &str, quality: u8) -> Result<(), String> {
    if !Path::new(dst).exists() {
        if let Some(parent) = Path::new(dst).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("не могу создать папку: {e}"))?;
            }
        }
    }

    match ext {
        "jpg" | "jpeg" => {
            let rgb = flatten_white(img);
            let file = File::create(dst).map_err(|e| format!("не могу создать {dst}: {e}"))?;
            let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(BufWriter::new(file), quality);
            enc.encode_image(&rgb).map_err(|e| e.to_string())?;
        }
        "webp" => {
            let file = File::create(dst).map_err(|e| format!("не могу создать {dst}: {e}"))?;
            let enc = image::codecs::webp::WebPEncoder::new_lossless(BufWriter::new(file));
            enc.encode(img.as_raw(), img.width(), img.height(), ExtendedColorType::Rgba8)
                .map_err(|e| e.to_string())?;
        }
        "png" => {
            let file = File::create(dst).map_err(|e| format!("не могу создать {dst}: {e}"))?;
            let mut enc = png::Encoder::new(BufWriter::new(file), img.width(), img.height());
            enc.set_color(png::ColorType::Rgba);
            let mut wtr = enc.write_header().map_err(|e| e.to_string())?;
            wtr.write_image_data(img).map_err(|e| e.to_string())?;
            wtr.finish().map_err(|e| e.to_string())?;
        }
        other => {
            // например bmp/tiff — пусть image сам решит
            let dyn_img = DynamicImage::ImageRgba8(img.clone());
            dyn_img
                .save_with_format(dst, ext_to_format(other)?)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn ext_to_format(ext: &str) -> Result<ImageFormat, String> {
    let f = match ext {
        "bmp" => ImageFormat::Bmp,
        "tiff" | "tif" => ImageFormat::Tiff,
        "ico" => ImageFormat::Ico,
        "pnm" | "ppm" | "pgm" | "pbm" => ImageFormat::Pnm,
        "tga" => ImageFormat::Tga,
        "farbfeld" | "ff" => ImageFormat::Farbfeld,
        "avif" => return Err("AVIF не поддерживается в этой сборке".into()),
        _ => return Err(format!("неизвестный формат вывода: {ext}")),
    };
    Ok(f)
}

/// Подложка белого, если есть прозрачность (для JPEG).
fn flatten_white(img: &RgbaImage) -> image::RgbImage {
    if !img.pixels().any(|p| p.0[3] < 255) {
        return image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
    }
    let (w, h) = (img.width(), img.height());
    let white = RgbaImage::from_pixel(w, h, image::Rgba([255, 255, 255, 255]));
    let mut base = RgbaImage::new(w, h);
    image::imageops::overlay(&mut base, &white, 0, 0);
    image::imageops::overlay(&mut base, img, 0, 0);
    image::DynamicImage::ImageRgba8(base).to_rgb8()
}

// ────────────────────────── стек (капы модов) ──────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum StackOrder {
    Down,
    Up,
}

/// Склейка анимации в Stacked PNG (кадры вертикально).
pub fn stack(src: &str, dst: &str, order: StackOrder) -> Result<(usize, u32, u32), String> {
    let frames = load_frames(src)?;
    let n = frames.len();

    let w = frames.iter().map(|f| f.rgba.width()).max().unwrap_or(1);
    let h = frames.iter().map(|f| f.rgba.height()).max().unwrap_or(1);

    // Нормализуем кадры к общему размеру (макс.) — как в conv.py.
    let mut normalized: Vec<RgbaImage> = Vec::with_capacity(n);
    for f in &frames {
        if (f.rgba.width(), f.rgba.height()) == (w, h) {
            normalized.push(f.rgba.clone());
        } else {
            normalized.push(DynamicImage::ImageRgba8(f.rgba.clone())
                .resize_exact(w, h, image::imageops::FilterType::Lanczos3)
                .to_rgba8());
        }
    }

    let mut canvas = RgbaImage::new(w, h * n as u32);
    for (i, fr) in normalized.iter().enumerate() {
        let y = match order {
            StackOrder::Down => i as u32 * h,
            StackOrder::Up => (n - 1 - i) as u32 * h,
        };
        image::imageops::overlay(&mut canvas, fr, 0, y as i64);
    }

    let has_alpha = normalized.iter().any(|f| f.pixels().any(|p| p.0[3] != 255));
    let color = if has_alpha { png::ColorType::Rgba } else { png::ColorType::Rgb };
    let bytes: Vec<u8> = if has_alpha {
        canvas.to_vec()
    } else {
        image::DynamicImage::ImageRgba8(canvas).to_rgb8().into_raw()
    };

    let file = File::create(dst).map_err(|e| format!("не могу создать {dst}: {e}"))?;
    let mut enc = png::Encoder::new(BufWriter::new(file), w, h * n as u32);
    enc.set_color(color);
    enc.add_text_chunk(
        "Description".into(),
        format!("Stacked PNG: {n} frames {w}x{h}, order={}", order_tag(order)),
    )
    .map_err(|e| e.to_string())?;
    let mut wtr = enc.write_header().map_err(|e| e.to_string())?;
    wtr.write_image_data(&bytes).map_err(|e| e.to_string())?;
    wtr.finish().map_err(|e| e.to_string())?;

    Ok((n, w, h))
}

fn order_tag(o: StackOrder) -> &'static str {
    match o {
        StackOrder::Down => "down",
        StackOrder::Up => "up",
    }
}

// ────────────────────────── инфо о файле ──────────────────────────

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub frames: usize,
    pub fps: f64,
    pub animated: bool,
}

pub fn file_info(src: &str) -> Result<FileInfo, String> {
    let frames = load_frames(src)?;
    let fmt_name = format_name(src);
    let (width, height) = (frames[0].rgba.width(), frames[0].rgba.height());
    let fps = if frames.len() > 1 {
        let valid: Vec<f64> = frames.iter().map(|f| f.delay_ms as f64).filter(|d| *d > 0.0).collect();
        let avg = if valid.is_empty() { 100.0 } else { valid.iter().sum::<f64>() / valid.len() as f64 };
        1000.0 / avg
    } else {
        0.0
    };
    Ok(FileInfo {
        format: fmt_name,
        width,
        height,
        frames: frames.len(),
        fps,
        animated: frames.len() > 1,
    })
}

fn format_name(path: &str) -> String {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return "?".to_string(),
    };
    let probe = BufReader::new(file);
    let r = image::ImageReader::new(probe).with_guessed_format();
    match r.map(|r| r.format()) {
        Ok(Some(f)) => format!("{f:?}").to_uppercase(),
        _ => "?".to_string(),
    }
}