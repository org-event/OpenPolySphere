# CI: tiered platform jobs, cache, path filters

Дата: 2026-06

Статус: Принято

## Контекст

После добавления Windows CI (`cargo clippy` + release + vcpkg OpenBLAS) один прогон занимает **25–35 минут** wall time. macOS и Windows идут **параллельно**, но оба стартуют на каждый push/PR, включая:

- правки только в `docs/` или README;
- волну dependabot PR (10+ полных Windows-сборок);
- работу на ветке Windows, когда macOS уже проверен локально.

Security (RustSec, CodeQL, dependency-review) — отдельные workflow и должны оставаться на каждом PR. Тяжёлые **platform build** jobs — нет.

## Решение

### 1. Path filters (`dorny/paths-filter`)

| Output | Пути |
|--------|------|
| `rust` | `crates/**`, `Cargo.*`, `.cargo/**` |
| `js` | `web/**`, `bun.lock`, eslint config |
| `workflows` | `.github/workflows/**`, `Justfile`, `scripts/**` |

`rustfmt` / platform jobs — только если `rust` или `workflows`. Чисто документационные PR — без macOS/Windows.

### 2. Кэш

- **`swatinem/rust-cache`** на macOS и Windows (`save-if: main` — не раздувать кэш с каждой feature-ветки).
- **`actions/cache`** для `C:\vcpkg\installed\x64-windows-static` — OpenBLAS не собирать 15+ мин каждый раз.
- ONNX Runtime zip — скачивать только если нет в `ort/`.

### 3. Scope platform jobs

| Событие | macOS | Windows |
|---------|-------|---------|
| push / PR, rust или workflows изменились | ✅ | ✅ |
| PR с label `ci/windows-only` | ❌ | ✅ |
| `workflow_dispatch` | по выбору `all` / `macos` / `windows` | по выбору |
| только docs/README | ❌ | ❌ |

Label **`ci/windows-only`** — вешать на PR, пока активно портирование Windows (экономия ~10 мин macOS).

### 4. Что не трогаем

- `security-audit.yml`, `codeql.yml`, `dependency-review.yml` — на каждом PR без изменений.
- `just prepush` локально — по-прежнему главный быстрый gate на Mac.

### 5. Защита `main`

Прямой push в `main` запрещён: **require pull request before merging** (см. `scripts/apply-main-branch-protection.sh`). Linear history уже включён.

## Почему не альтернативы

- **Всегда полный matrix на каждый push** — очередь, dependabot-шторм, разработка Windows неподъёмна.
- **Отключить security на feature-ветках** — CVE пройдут незаметно; путают «долгий build» и «audit».
- **Только manual `workflow_dispatch` для Windows** — легко забыть перед merge; оставили dispatch как дополнение, не замену.
- **Один ubuntu cross-build вместо Windows runner** — не ловит MSVC/CRT (уже обожглись с `+crt-static`).

## Последствия

**Плюсы:**

- docs-only PR — секунды вместо получаса.
- Windows-ветка с label — без macOS.
- vcpkg + rust-cache — ожидаемо −10…15 мин на Windows после прогрева кэша на `main`.
- `workflow_dispatch` — ручной полный прогон одной ОС.

**Минусы:**

- Нужно помнить про label `ci/windows-only` и снять его перед merge в `main`.
- Первый прогон на ветке без кэша — по-прежнему долгий.
- Path filter может пропустить edge case (редкий путь вне списка) — тогда `workflow_dispatch` или поправить filters.

## Связанные файлы

- `.github/workflows/ci.yml`
- `CONTRIBUTING.md` — label и workflow
- `scripts/apply-main-branch-protection.sh`
- `docs/windows.md`
