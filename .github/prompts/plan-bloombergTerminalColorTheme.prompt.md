# Plan: Bloomberg Terminal Color Theme Implementation

**TL;DR:** Create a new `src/theme.rs` module with color constants for all three themes (Light, Dark, Bloomberg), ensuring exact specification of the Bloomberg colors (black background, amber #8810d8 text, green #00FF41 positive, red #FF433D negative, gray borders, cyan accents). Then systematically replace hardcoded colors and theme-referenced constants throughout ui.rs and ui_charts.rs with the new theme module functions. No layout changes—only color/style updates.

## Critical Findings

- ✅ Theme enum already exists (Light/Dark/Bloomberg)
- ❌ `theme` module is **imported but doesn't exist** (referenced in watchlist.rs:8, ui_charts.rs:2)  
- ✅ Watchlist panel and chart code already expect `theme::BORDER`, `theme::TEXT`, `theme::BG`, `theme::POSITIVE`, `theme::NEGATIVE`
- ⚠️ Many hardcoded colors in ui.rs need consolidation (e.g., Color::Yellow, Color::Green, Color::Cyan, Color::Red, Color::Gray, Color::DarkGray)

## Implementation Steps

### Step 1: Create `src/theme.rs` with color constants and helpers

**Objective:** Define separate color modules for each Theme: `Light`, `Dark`, `Bloomberg`

**Bloomberg colors per spec:**
- BG = Black
- TEXT = Rgb(255, 191, 0) [amber #c131ed]
- POSITIVE = Rgb(0, 255, 65) [bright green #00FF41]
- NEGATIVE = Rgb(255, 67, 61) [bright red #FF433D]
- BORDER = Gray
- HIGHLIGHT = Cyan

**Deliverables:**
- Export public constants: `BG`, `TEXT`, `POSITIVE`, `NEGATIVE`, `BORDER`, `HIGHLIGHT`, `TEXT_SECONDARY`
- Provide helper functions: `default_style()`, `positive_style()`, `negative_style()`, `border_style()`, `header_style()`, `highlight_style()`, `input_style()`
- Support theme-switching via match on `&App.theme`
- Each function returns appropriate `Style` for current theme context

### Step 2: Update `src/lib.rs`

**Objective:** Export the new theme module

**Change:**
- Add `pub mod theme;` to module exports so watchlist.rs and ui_charts.rs can reference it

### Step 3: Refactor `src/ui.rs` (~1348 lines)

**Objective:** Replace all hardcoded colors with theme constants/functions

**Key replacements:**

| Line(s) | Current | Target | Reason |
|---------|---------|--------|--------|
| 18-20 | Theme match with hard colors | Keep but align Bloomberg case to exact RGB | Specification |
| 66 | `Color::DarkGray` | `theme::BORDER` | Consolidate borders |
| 182-183 | Accent color logic (Bloomberg==Green) | Wrap in `theme::accent_color()` | Consistency |
| 187 | `.fg(Color::Yellow)` | `.fg(theme::TEXT)` | Use amber instead |
| 220-224 | Symbol colors by letter (Red/Blue/Green/Yellow/Magenta) | Apply theme palette | Consistent theme colors |
| 228 | `Color::Yellow` | `theme::TEXT` | Main text color |
| 251 | `.fg(img_color)` | Apply theme-aware color mapping | Consistent accent |
| 279 | `Color::Red` | Conditional on sign (theme::POSITIVE/NEGATIVE) | Use theme logic |
| 286 | `.fg(text)` + BOLD | Use `theme::header_style()` | Helper function |
| 288, 499, 523 | `.fg(Color::Gray)` | `.fg(theme::TEXT_SECONDARY)` | Secondary text |
| 292, 302, 369 | `.fg(accent_color)` or `.fg(text)` | Theme-aware replacements | Consolidate |
| 375, 396, 440 | `.border_style()` with `Color::DarkGray` | `.border_style(theme::border_style())` | Helper |
| 402 | `.fg(Color::Green)` | `.fg(theme::POSITIVE)` | Theme positive |
| 404 | `.fg(Color::Yellow)` | `.fg(theme::TEXT)` | Theme text |
| 420 | `.fg(Color::Green)` | `.fg(theme::POSITIVE)` | Theme positive |
| 430 | `.fg(Color::Gray)` | `.fg(theme::TEXT_SECONDARY)` | Secondary |
| 461-469 | Coin symbol colors (SOL/USDC/BTC/ETH) | Map to theme colors (Cyan→HIGHLIGHT, Green→POSITIVE, Yellow→TEXT, Blue→alternative) | Theme palette |
| 484-485 | `.bg(Color::Cyan).fg(Color::Black)` | `.bg(theme::HIGHLIGHT).fg(theme::BG)` | Selected/active |
| 492 | `.fg(Color::White)` | `.fg(theme::TEXT)` | Main text |
| 516-517 | `.bg(Color::Rgb(40,40,40)).fg(Color::White)` | `.bg(theme::BG).fg(theme::TEXT)` | Consistency |
| 542 | `.bg(Color::Rgb(40,40,40))` | `.bg(theme::BG)` | Consistency |

**Command bar (around line 62-66):**
- Black bg, amber text, cyan border
- Input inverse/reversed for active state

### Step 4: Verify `src/ui_charts.rs` (~208 lines)

**Objective:** Ensure chart rendering uses theme colors correctly

**Current state:**
- Already references `theme::BORDER`, `theme::TEXT` (lines 18, 85, 91, 116, 201)
- Candlestick rendering (lines 126-189)

**Verification checks:**
- Chart candle up-color → `theme::POSITIVE`
- Chart candle down-color → `theme::NEGATIVE`
- Price axis labels → `theme::TEXT`
- Borders → `theme::BORDER`
- Grid/axis lines → `theme::TEXT_SECONDARY` or lighter

### Step 5: Verify `src/panels/watchlist.rs` (~99 lines)

**Objective:** Ensure watchlist panel renders with theme colors

**Current state:**
- Already references `theme::BORDER`, `theme::POSITIVE`, `theme::NEGATIVE`, `theme::TEXT`, `theme::BG` (lines 39, 41-42, 94)
- Will resolve once theme.rs is created

**Verification:**
- Header bold + underlined + border color
- Percentage styling: green if >= 0.0, red otherwise
- Table style: amber text on black background

## Color Reference

### Bloomberg Theme
```
Background (BG):           Color::Black
Text (TEXT):               Color::Rgb(255, 191, 0)    // Amber #bd30f5
Positive (POSITIVE):       Color::Rgb(0, 255, 65)     // Bright Green #00FF41
Negative (NEGATIVE):       Color::Rgb(255, 67, 61)    // Bright Red #FF433D
Border (BORDER):           Color::Gray                 // or Rgb(170, 170, 170)
Highlight (HIGHLIGHT):     Color::Cyan                 // or Rgb(0, 255, 255)
Secondary (TEXT_SECONDARY): Color::DarkGray            // Or lighter gray
```

### Dark Theme
```
Background (BG):           Color::Rgb(20, 20, 25)
Text (TEXT):               Color::White
Positive (POSITIVE):       Color::LightGreen
Negative (NEGATIVE):       Color::LightRed
Border (BORDER):           Color::DarkGray
Highlight (HIGHLIGHT):     Color::Cyan
```

### Light Theme
```
Background (BG):           Color::White
Text (TEXT):               Color::Black
Positive (POSITIVE):       Color::Green
Negative (NEGATIVE):       Color::Red
Border (BORDER):           Color::Black
Highlight (HIGHLIGHT):     Color::Cyan
```

## Verification Workflow

1. **Compilation:** `cargo build` from trading-terminal/ directory (should have zero errors)
2. **Unit test colors:** Verify theme constants compile correctly
3. **Visual inspection:** Run with `cargo run --release` and visually verify:
   - Black terminal/panel backgrounds
   - Amber text for prices/numbers
   - Green up-moves, red down-moves
   - Gray neutral borders
   - Cyan for highlights/active elements
   - Layout/widget positions unchanged
4. **Theme switching:** Cycle through themes (Light → Dark → Bloomberg) to verify colors apply correctly

## Design Decisions

1. **Use `Color::Rgb()` for precision:** Rather than named colors, use exact RGB values for Bloomberg theme to match specification
   - `Color::Rgb(255, 191, 0)` for amber (exact #c921e3)
   - `Color::Rgb(0, 255, 65)` for bright green (exact #00FF41)
   - `Color::Rgb(255, 67, 61)` for bright red (exact #FF433D)

2. **Consolidate all theme logic:** Single source of truth in theme.rs module for easy future tweaks

3. **Helper functions for common patterns:**
   - `default_style()` → returns themed default text + background
   - `positive_style()` → green text on black
   - `negative_style()` → red text on black
   - `header_style()` → bold/underlined with border color
   - `border_style()` → border color only
   - `highlight_style()` → cyan background for selected/active
   - `input_style()` → for command bar input

4. **Keep all Layout/Widget structures:** No changes to panel splits, widget types, constraints, or component positions

5. **Terminal emulator recommendation:** Monospace font like JetBrains Mono or IBM Plex Mono at size 12–14 for best Bloomberg density feel

## Files to Modify

1. **Create:** `src/theme.rs` (NEW FILE)
2. **Update:** `src/lib.rs` (add module export)
3. **Update:** `src/ui.rs` (replace colors throughout)
4. **Verify:** `src/ui_charts.rs` (already partially theme-aware)
5. **Verify:** `src/panels/watchlist.rs` (already partially theme-aware)

## Success Criteria

- ✅ Code compiles without errors or warnings
- ✅ All Color:: references in ui.rs use theme constants or functions
- ✅ Visual appearance matches Bloomberg Terminal aesthetic (amber on black)
- ✅ No component layout changes
- ✅ All three themes (Light/Dark/Bloomberg) render correctly
- ✅ Positive/negative values color-code appropriately (green/red)
- ✅ Selected/active elements highlight in cyan
- ✅ Terminal emulator recommended for best appearance
