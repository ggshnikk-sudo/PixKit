# PixKit

Кросс-платформенный конвертер изображений (Rust, без внешних зависимостей).
Работает на Windows / Linux. Один статический бинарник, ничего ставить не нужно.

## Скачать

Готовые сборки — в [Releases](../../releases) для `windows-x64`, `linux-x64` и `linux-arm64`.

| Файл | Что это |
|---|---|
| `pixkit-windows-x64.exe` | GUI (склейка капов GIF/APNG → стек PNG). Без консоли. |
| `pixkit-cli-windows-x64.exe` | Консольная версия (все операции). |
| `pixkit-linux-x64` / `pixkit-cli-linux-x64` | То же для Linux x64. |
| `pixkit-linux-arm64` / `pixkit-cli-linux-arm64` | То же для Linux arm64. |

## GUI (двойной клик)

- «Откуда» — входной GIF/APNG (можно перетащить файл в окно).
- «Куда» — куда сохранить стек (по умолчанию рядом с исходником: `имя-stack.png`).
- «Порядок кадров»: первый сверху (down) / первый снизу (up).
- Кнопка «▶ Склеить». Превью и инфо покажутся сразу.

## CLI

```
pixkit-cli convert  in.png  out.webp [--width N] [--height N] [--quality N]
pixkit-cli convert  anim.gif out.apng
pixkit-cli stack    anim.gif out.png        [--up | --down]
pixkit-cli stack    anim.apng out.png
pixkit-cli info     file.gif
```

Форматы: чтение — GIF, APNG/PNG, WebP (в т.ч. анимации), JPEG, BMP, TIFF.
Запись — PNG, JPEG, WebP, GIF (анимации), APNG (анимации).

На Windows CLI сам включает поддержку ANSI-цветов и UTF-8 (cmd по умолчанию их не умеет);
если вывод идёт не в терминал — цвета отключаются автоматически.

## Сборка

Нужен Rust (stable):

```
cargo build --release --bins
```

Бинарники появятся в `target/release/`: `pixkit` (GUI) и `pixkit-cli`.

## Лицензия

MIT — см. [LICENSE](LICENSE).
