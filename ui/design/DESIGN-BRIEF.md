# OpenPolySphere — Design Brief (Granola-inspired)

> **Статус:** черновик v0 · **Цель:** greenfield UI-концепт, не клон текущего web UI  
> **Референс по настроению:** [Granola](https://www.granola.ai) — спокойный, вымеренный, современный  
> **Исполнение:** OpenPencil → `ui/design/openpolysphere.fig`  
> **Правила рендера:** `.cursor/rules/openpencil-design.mdc`  
> **Правила агента (MCP/CLI):** `.cursor/rules/openpencil-agent.mdc`

---

## Прогресс (отмечать по мере готовности)

| Фаза | Что | Статус |
|------|-----|--------|
| **0** | Brief согласован (этот файл) | ✅ |
| **1** | Foundations — принципы, типографика, сетка, радиусы | ✅ |
| **2** | Palettes — light + dark токены | ✅ |
| **3** | Elements — атомы (цвет, текст, border, dot, icon-slot) | ✅ |
| **4** | Patterns — крупные куски (панели, строки, карточки) | ✅ |
| **5** | Components — кнопки, chips, bubbles, metrics, inputs | ✅ |
| **6** | Screens — экраны/окна из компонентов | ✅ |
| **7** | OpenPencil file + PNG self-review | ✅ (ожидает проверку в GUI) |
| **8** | Product — полный инвентарь фич из кода | ✅ (ожидает проверку в GUI) |

**Текущий `.fig`:** 6 страниц, 667 нод + PNG-превью в `ui/design/review/`.

```bash
bun ui/design/apply-tokens.mjs      # фазы 1–2
bun ui/design/build-elements.mjs    # фазы 1–3
bun ui/design/build-patterns.mjs    # фазы 1–4
bun ui/design/build-components.mjs  # фазы 1–5
bun ui/design/build-screens.mjs     # фазы 1–6 (концепт v1)
bun ui/design/build-product.mjs     # фазы 1–8 (актуальная полная пересборка)

# PNG self-review (фазы 7–8)
mkdir -p ui/design/review
for p in Foundations Tokens Elements Patterns Components Screens; do
  bun "$(command -v openpencil)" export ui/design/openpolysphere.fig --page "$p" -f png -s 2 -o "ui/design/review/$p.png"
done
bun "$(command -v openpencil)" export ui/design/openpolysphere.fig --page Screens -f png -s 1 -o ui/design/review/Screens-product.png
bun "$(command -v openpencil)" export ui/design/openpolysphere.fig --page Components -f png -s 2 -o ui/design/review/Components-product.png
```

**Логотип в `.fig`:** `ui/design/openpolysphere-tree-icon.png` (56px из `site/img/openpolysphere-tree-icon.svg` через `createImage`). Перегенерация PNG:

```bash
bunx @resvg/resvg-js-cli --fit-width 56 site/img/openpolysphere-tree-icon.svg ui/design/openpolysphere-tree-icon.png
```

---

## 0. Brand book — как меняется всё разом

**Файл:** `ui/design/tokens.json` — единственный источник правды (цвета light/dark, spacing, radius, typography).

| Что меняешь | Что происходит |
|-------------|----------------|
| `tokens.json` → `brand/open`, `accent/calm`, … | Пересборка страниц из токенов |
| Spacing / radius / type scale | То же |
| Будущие Components / Screens | Строятся **только** из `tokens.json`, не хардкод hex |

**Пересборка фаз 1–2:**

```bash
bun ui/design/apply-tokens.mjs
```

**Честно про OpenPencil:** `figma.variables` **пока нет** (ни в CLI, ни в app). Нет Figma-style live binding «поменял переменную — всё обновилось в GUI».  
Каскад у нас такой:

1. Правишь **brand book** (`tokens.json`)
2. Запускаешь **apply-tokens** (фазы 1–2) или **build-elements** (фазы 1–3)
3. В коде (React/CSS) — те же токены → CSS variables / theme (фаза 7+)

Имена слоёв в `.fig`: `token / light / brand/open` — чтобы найти и пересобрать.  
Когда OpenPencil добавит variables — мигрируем `tokens.json` → native collections.

**Проверка:** поменяй `brand/open` в `tokens.json` → `bun ui/design/apply-tokens.mjs` → swatches и Foundations обновятся.

---

## 1. Что такое «стиль Granola» (разбор, не копипаст)

Granola — не «ещё один dark SaaS». Это осознанный **indie-premium / warm editorial productivity**.

### 1.1 К какому типу дизайна относится

| Ось | Granola | Антипример (не делаем) |
|-----|---------|-------------------------|
| Тон | Calm presence — спокойный фон, контент главный | Dashboard с 20 кнопками |
| Теплота | Warm cream, оливковый/лаймовый акцент, «бумага» | Холодный `#0a0a0a` + неон |
| Типографика | **Display serif** (Quadrant) + **нейтральный UI sans** (Melange) | Один Inter везде |
| Плотность | Воздух, 40–64px между секциями, узкая колонка чтения | Всё в одну полосу toolbar |
| Границы | Hairline (≈0.5–1px), едва заметные | Толстые рамки, glow |
| Формы | Pills, мягкие radius 10–16 | Острые прямоугольники |
| Акцент | Один органический зелёный + редкий highlight | Радуга метрик |
| Характер | «Блокнот, который оказался софтом»; чуть неровный, human | Корпоративный шаблон |

### 1.2 Ключевая идея бренда (из их ребренда 2026)

> Быстрая жизнь пользователя, но **спокойный инструмент внутри неё**.  
> Энергия снизу, тишина сверху.

Для OpenPolySphere это переводится так:

- **Звонок и речь** — хаос снаружи.
- **Перевод на экране** — тихая, уверенная опора: читаемо, без суеты, без «пульта Чернобыля».

### 1.3 Что берём от Granola (принципы)

1. **Content-first** — центр экрана = диалог / перевод, не chrome.
2. **Сдержанная палитра** — 1 brand accent + нейтрали + семантика (success / danger).
3. **Типографическая иерархия** — крупный перевод, мелкий оригинал, mono для latency.
4. **Мягкие поверхности** — в light: cream + frosted cards; в dark: warm charcoal, не pure black.
5. **Минимум постоянных controls** — основное в одной нижней или верхней зоне, остальное в overflow / settings.
6. **Доверие через тишину** — latency показываем деликатно, не пятью неоновыми чипами.

### 1.4 Что НЕ копируем буквально

- Оливковый Granola как единственный brand color (у OpenPolySphere свой зелёный/синий бренд).
- Только light theme (нужны **оба**: light как Granola, dark как calm warm night).
- Meeting-notes layout (у нас **live bilingual call**, не пост-митинг).
- Шрифты Quadrant/Melange в OpenPencil могут быть недоступны → см. fallbacks в фазе 1.

---

## 2. Продукт: что проектируем

**OpenPolySphere** — desktop real-time speech translator во время звонка.

### Must-have на главном экране (Call)

- [ ] Статус сессии (live / stopped / connecting)
- [ ] Пара языков (например EN ↔ RU) — заметно, но спокойно
- [ ] Лента: оригинал + перевод (outgoing / incoming визуально различимы)
- [ ] Индикатор «слушаем…» / typing
- [ ] Start / Stop (одна главная CTA)
- [ ] Mic out / Mic in (mute) — с понятным состоянием
- [ ] Latency / качество — **одной строкой или тонким индикатором**, не dashboard

### Nice-to-have (отдельные экраны / панели)

- [ ] Settings (slide-over)
- [ ] History
- [ ] Export transcript
- [ ] Logs / errors
- [ ] Monitor (TTS в браузере)

### Не тащим на главный экран

- 12 кнопок toolbar как сейчас в web
- Settings form на полный экран
- Метрики как пять цветных коробок

---

## 3. Ограничения desktop-приложения

| Ограничение | Влияние на дизайн |
|-------------|-------------------|
| **Shell:** `tao` + `wry` (не Tauri) | Один embedded WebView, без нативных NSWindow controls в макете |
| **Типичный размер окна** | 1100–1400 × 700–900; artboard **1280 × 860** |
| **Platform** | macOS приоритет, Windows/Linux parity |
| **Шрифты** | Системные / Inter / SF; display-serif — опционально, с fallback |
| **Иконки** | SVG в коде; в OpenPencil — placeholder-рамки 16–20px |
| **Тема** | `data-theme` light/dark — **оба** варианта в brief |
| **Производительность** | Без тяжёлых blur в MVP CSS (в OpenPencil blur может ломать рендер) |
| **Реальный функционал** | Дизайн не добавляет фич — только подаёт существующие |

---

## 4. Жёсткие границы (что не трогаем)

```
⛔ НЕ открывать и НЕ копировать web/static/index.html и style.css как источник layout
⛔ НЕ «подровнять текущий UI» — это другая задача
⛔ НЕ коммитить без явной просьбы
✅ Можно смотреть: список фич (must-have выше), бренд-цвета Open/PolySphere
✅ Референс настроения: Granola, не OpenPolySphere web
```

---

## 5. Ограничения агента (честно — чтобы не накосячить)

Агент (Cursor) **умеет** вести этот brief по фазам. Склонности, за которыми нужен контроль:

| Склонность | Что будет | Противоядие |
|----------|-----------|-------------|
| Якорь на репо | Снова скопирует header/toolbar из web | Фаза 0: явный запрет; чеклист «не похоже на index.html» |
| Одним махом | «Великолепие» за один eval-скрипт | Только одна фаза за итерацию |
| OpenPencil bugs | Пустой canvas, NaN fills, nested coords | `.cursor/rules/openpencil-design.mdc` |
| Fake design | Прямоугольники без иерархии | Сначала токены и типографика, потом экран |
| Переусложнение | Components page, scripts, Justfile | Только `DESIGN-BRIEF.md` + `openpolysphere.fig` |
| Ложный success | PNG ок, в GUI пусто | PNG + tree + напомнить reload |

**Ожидание:** первые версии — **стильно и цельно**, не pixel-perfect. Итерации по чеклисту.

---

## 6. Критерии успеха

Дизайн считается удачным, если:

1. **Не узнаётся** как текущий web UI с первого взгляда.
2. **Ощущается Granola-like:** спокойно, тепло, вымеренно, современно.
3. **Иерархия ясна:** перевод > оригинал > мета > chrome.
4. **Главный экран** читается за 3 секунды: кто говорит, что переведено, live или нет.
5. **Light и dark** согласованы одной системой токенов.
6. **Реализуемо** в vanilla CSS / будущем React без магии.
7. **OpenPencil:** один screen frame, flat children, видно в GUI после reload.

---

## 7. Фазы работ (подробно)

### Фаза 1 — Foundations ✅

- [x] **Spacing:** база 4px, шаг 8px; секции 32 / 48 / 64
- [x] **Radius:** sm 8, md 12, lg 16, pill 999
- [x] **Typography roles:**
  - Display — serif или semibold sans для редких заголовков
  - Body — 15–16px, line-height 1.5
  - UI — 13–14px medium
  - Meta — 11–12px mono, muted
- [x] **Шрифты (OpenPencil):** Inter Regular/Medium/Semi Bold; display: Inter Bold или позже serif
- [x] **Сетка artboard:** margins 24–32px, max text width ~680px для ленты

### Фаза 2 — Palettes ✅

#### Light (Granola-adjacent)

| Token | Значение | Назначение |
|-------|----------|------------|
| `bg/base` | `#f7f7f2` | тёплый cream фон |
| `bg/elevated` | `#ffffff` @ 70% | карточки |
| `text/primary` | `#1a1814` | основной текст |
| `text/secondary` | `#6a5f52` | оригинал, подписи |
| `text/muted` | `#9a8f82` | meta, latency |
| `border/subtle` | `rgba(26,24,20,0.08)` | hairline |
| `brand/open` | `#2ecf7a` | Open (сохраняем узнаваемость) |
| `brand/poly` | `#3d9be8` | PolySphere |
| `accent/calm` | `#5b6f00` | спокойный olive CTA (Granola-like) |
| `semantic/live` | `#3d9a5a` | live, connected |
| `semantic/danger` | `#c44d4d` | stop, muted mic |

#### Dark (warm night, не cold SaaS)

| Token | Значение | Назначение |
|-------|----------|------------|
| `bg/base` | `#141310` | тёплый charcoal |
| `bg/elevated` | `#1e1c18` | карточки |
| `text/primary` | `#f5f2eb` | основной |
| `text/secondary` | `#a69e92` | оригинал |
| `border/subtle` | `rgba(245,242,235,0.08)` | hairline |
| `brand/*` | как в light | узнаваемость |
| `accent/calm` | `#8fa83a` | приглушённый olive на тёмном |

- [x] Таблицы выше утверждены
- [x] Страница **Tokens** в `.fig` (цветовые swatches)

### Фаза 3 — Elements ✅

- [x] Hairline divider (1px)
- [x] Dot / status LED (6–8px)
- [x] Text styles: display, body, ui, meta-mono
- [x] Icon slot 16 / 20 / 24 (пустая рамка + label)
- [x] Focus ring spec (для CSS; в OpenPencil опционально)

### Фаза 4 — Patterns ✅

- [x] **Top bar** — низкая, воздушная (44px, не монолит)
- [x] **Floating bottom bar** — pill dock (Granola command bar)
- [x] **Message stack** — вертикальная лента с ритмом
- [x] **Language chip** — compact pill EN ↔ RU
- [x] **Session strip** — live dot + timer + connection одной линией

### Фаза 5 — Components ✅

Страница **Components** в `.fig`:

- [x] Button / Primary (olive calm)
- [x] Button / Secondary (frosted / hairline)
- [x] Button / Danger (stop)
- [x] Chip / Language
- [x] Chip / Status (live, connected)
- [x] Toggle / Mic
- [x] Bubble / Outgoing
- [x] Bubble / Incoming
- [x] Row / Metric (одна строка, не пять коробок)
- [x] Typing indicator
- [x] List item / Settings (заготовка)

### Фаза 6 — Screens ✅

Страница **Screens** в `.fig`:

- [x] **Call / Light** — главный, greenfield
- [x] **Call / Dark**
- [x] **Settings / Slide-over** (опционально v2)
- [x] **Empty / Waiting** — до старта сессии

### Фаза 7 — Delivery ✅ (GUI — на тебе)

- [x] `openpolysphere.fig` обновлён (6 страниц, 418 нод — концепт v1)
- [x] PNG export каждого ключевого экрана → `ui/design/review/*.png`
- [x] Tree check (один artboard per page, flat children)
- [ ] Пользователь подтвердил: видно в OpenPencil GUI

### Фаза 8 — Product ✅ (GUI — на тебе)

**Скрипт:** `ui/design/build-product.mjs` — полная пересборка фаз 1–8.

**Инвентарь из кода** (`web/static/index.html`, `history.html`, `locales/en.json`), **не** копия layout:

- [x] **Call / Light + Dark** — dock с иконками (Stop, Mic Out/In, Monitor, More)
- [x] **Call / Overflow** — меню: Logs, Tab Audio, Compact, Saved, Export, Clear
- [x] **Empty / Waiting** — до старта сессии
- [x] **Settings / Full** — slide-over 480px, 5 accordion (API Keys expanded), Save & Restart
- [x] **History / List** — карточки звонков
- [x] **History / Detail** — summary, utterances, Back / Summary / Delete
- [x] **Components / Icon slots** — 11 toolbar placeholders (16px stroke)
- [x] `openpolysphere.fig` обновлён (667 нод)
- [x] PNG → `ui/design/review/Screens-product.png`, `Components-product.png`
- [ ] Пользователь подтвердил: видно в OpenPencil GUI

---

## 8. Композиция главного экрана (концепт v1)

```
┌─────────────────────────────────────────────────────────┐
│  [logo]  OpenPolySphere          ● Live  02:34  Connected │  ← тихий top bar
│                          [ EN ● — ● RU ]        [ ⚙ ]   │
├─────────────────────────────────────────────────────────┤
│                                                         │
│                    (много воздуха)                      │
│         ┌─────────────────────────────┐               │
│         │  Перевод (крупно)            │  ← outgoing   │
│         │  original italic, muted       │               │
│         └─────────────────────────────┘               │
│   ┌─────────────────────────────┐                       │
│   │  Перевод                     │  ← incoming         │
│   │  original                    │                       │
│   └─────────────────────────────┘                       │
│              ··· listening                              │
│                                                         │
├─────────────────────────────────────────────────────────┤
│      ╭─────────────────────────────────────────────╮   │
│      │  ■ Stop   🎤 out   🎤 in   Monitor    ···   │   │  ← pill dock
│      ╰─────────────────────────────────────────────╯   │
│      STT 142ms · Translate 198ms · Total 627ms         │  ← одна строка meta
└─────────────────────────────────────────────────────────┘
```

**Отличия от текущего web:** нет плотного toolbar из 10 кнопок; перевод крупнее; метрики — строка; dock — одна pill.

---

## 9. Промпт для агента (копировать в чат)

```
Читаем ui/design/DESIGN-BRIEF.md.
Делаем только фазу [N] из чеклиста.
Стиль: Granola-like (calm, warm, editorial, content-first).
НЕ открывать web/static/index.html.
OpenPencil: один screen frame, flat children, file-mode eval.
По завершении: PNG self-review + отметить фазу в DESIGN-BRIEF.md.
```

---

## 10. Ссылки

- [Granola — new look (blog)](https://www.granola.ai/blog/a-new-look-for-granola)
- [Granola on Refero Styles](https://styles.refero.design/style/d6c2a911-45ed-4860-a992-43df22793c2a)
- [Ragged Edge — Granola identity](https://abduzeedo.com/granola-brand-identity-design-ragged-edge/)
- OpenPencil render rules: `.cursor/rules/openpencil-design.mdc`
- OpenPencil agent playbook: `.cursor/rules/openpencil-agent.mdc`
- [OpenPencil MCP tools](https://open-pencil-open-pencil.mintlify.app/mcp/tools-reference)
- [OpenPencil CLI](https://openpencil.dev/reference/cli)

---

*Делаем для души хорошо — по одной фазе, с проверкой, без спешки.*
