# Платформенные бэкенды через imp/stub, не cfg в оркестраторах

Дата: 2026-06

Статус: Принято

## Контекст

OpenPolySphere изначально macOS-first (PolySphere Speech, Metal Whisper, PolySphere Translate). Параллельно идёт порт на Windows: CI на `windows-latest` с `cargo clippy -D warnings`.

Разработка в основном на macOS. На Mac host-clippy зелёный, на Windows — падает на типичных ошибках:

- `unused import` — символ импортирован на уровне модуля, но используется только в `#[cfg(unix)]` / `#[cfg(target_os = "macos")]`;
- `dead_code` и расхождения веток, если `#[cfg(target_os = …)]` размазан по оркестраторам (`stt/mod.rs`, `translation/mod.rs`, `main.rs`).

Нужна одна понятная граница «где живёт платформа», без сюрпризов в Windows job после каждого macOS PR.

## Решение

1. **`platform/`** — capabilities, пути, bundle env (`crates/audio-core/src/platform/`). Единственное место для compile-time флагов вида `cfg!(target_os = "macos")` на уровне продукта.

2. **imp/stub в бэкендах** — модули с двумя `mod imp`:
   - `#[cfg(target_os = "macos")] mod imp { … }` — реальная реализация;
   - `#[cfg(not(target_os = "macos"))] mod imp { … }` — stub с тем же публичным API.

   Примеры: `stt/apple.rs`, `translation/apple.rs`, `stt/local/metal.rs`.

3. **Оркестраторы без `#[cfg(target_os)]`** — `stt/mod.rs`, `translation/mod.rs`, `stt/local/mod.rs`, `translator/main.rs` вызывают единый API; проверка возможностей через `Capabilities::current()` и runtime `bail!` при выборе macOS-only бэкенда на чужой ОС.

4. **Статический guard** — `scripts/check-windows-lint.sh` (в `just prepush`) запрещает `cfg(target_os` в оркестраторах и требует imp/stub в платформенных бэкендах; ловит unix-only импорты в файлах с `#[cfg(windows)]`.

## Почему не альтернативы

- **`#[cfg]` везде в mod.rs** — Mac и Windows компилируют разные срезы одного файла; Mac dev не видит Windows warnings. Уже ломало CI (`port.rs`, Metal в `common.rs`).

- **Cargo features `macos` / `windows`** — не отменяют разный target при кросс-компиляции; всё равно нужны stub'ы. Плюс размножение feature-матриц в `Cargo.toml` для маленькой команды.

- **Отдельные крейты `polysphere-macos` / `polysphere-win`** — честная граница, но overhead: версии, CI, публикация. Рано для текущего размера проекта.

- **Runtime plugin loading (dylib)** — против **rust-first**: усложняет деплой, подпись, Windows artifact; не нужен для двух-трёх ОС.

- **Полный Windows clippy на Mac (zig / cross)** — не взлетает из-за нативных C-зависимостей (OpenBLAS, sentencepiece, MSVC/cmake). Статический lint + CI на нативной Windows — осознанный компромисс (см. `docs/windows.md`).

## Последствия

**Плюсы:**

- Оркестраторы читаются как обычный Rust без платформенного шума.
- Регрессии cfg в оркестраторах ловятся за секунды в `just prepush`, без Windows runner.
- Новый macOS-only бэкенд = новый файл imp/stub, а не правки в пяти mod.rs.

**Минусы:**

- Больше файлов; stub'ы нужно держать в синхроне с API imp (пустые `Ok(())` / `None` должны быть осмысленными).
- «Почему пустой stub?» без ADR неочевидно — этот документ как раз для этого.
- Windows dev всё ещё нужен для полного clippy и линковки; Mac не заменяет CI.

## Связанные файлы

- `crates/audio-core/src/platform/` — capabilities, paths
- `crates/audio-core/src/stt/apple.rs`, `translation/apple.rs`, `stt/local/metal.rs` — imp/stub
- `scripts/check-windows-lint.sh`, `Justfile` (`check-windows-static`, `prepush`)
- `docs/windows.md` — CI и локальные проверки по ОС
