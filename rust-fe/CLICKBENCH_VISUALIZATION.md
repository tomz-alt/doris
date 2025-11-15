# ClickBench-Style Visualization

This benchmark suite uses **exact ClickBench visualization** for presenting TPC-H and TPC-DS results.

## Overview

Inspired by [ClickBench](https://benchmark.clickhouse.com/), the world's premier database benchmark comparison tool, our visualization provides:

✅ **Clean, minimalist design** - Focus on data, not decoration
✅ **Bar-based visualizations** - Instant visual comparison
✅ **Theme support** - Light/dark mode toggle
✅ **Responsive layout** - Works on all devices
✅ **Zero dependencies** - No Chart.js or external libraries

## Features

### 1. ClickBench-Style Summary Table

```
Query Execution Times (Lower is Better)
────────────────────────────────────────────────
Query    Java FE                    Rust FE
────────────────────────────────────────────────
Q1       ████████████████  2.450s   █████  0.825s   2.97x
Q2       ██████████        1.234s   ████   0.456s   2.70x
Q3       █████████████     1.567s   █████  0.534s   2.93x
...
────────────────────────────────────────────────
```

**Features:**
- Color-coded bars (orange for Java FE, green for Rust FE)
- Proportional bar widths
- Monospace numbers for alignment
- Hover highlighting

### 2. Theme Toggle

Click 🌓 button to switch between:
- **Light mode**: Clean white background
- **Dark mode**: ClickBench's signature dark blue-green (#04293A)

Themes persist across page loads using localStorage.

### 3. Metadata Dashboard

```
┌──────────────┬────────┬──────────┬────────────────────────┐
│ Benchmark    │  SF1   │ 5 Rounds │ Geom Mean Speedup: 2.87x│
│ TPC-H        │        │          │                         │
├──────────────┼────────┼──────────┼─────────────────────────┤
│ Java FE      │ Rust FE│          │                         │
│ 127.0.0.1:9030│127.0.0.1:9031│   │                         │
└──────────────┴────────┴──────────┴─────────────────────────┘
```

### 4. Detailed Results Table

- Per-query statistics (mean, stddev, speedup)
- Median and best speedup calculations
- Color-coded speedup indicators

### 5. Overall Statistics

- Geometric mean speedup (primary metric)
- Arithmetic mean, median, best, worst speedup
- Clean table layout

## Visual Design

### Color Scheme

**Light Theme:**
- Background: `white`
- Text: `black`
- Java FE bars: `#FFA500` (orange)
- Rust FE bars: `#4CAF50` (green)
- Good speedup: `#4CAF50` (green)
- Bad speedup: `#FF5252` (red)

**Dark Theme:**
- Background: `#04293A` (ClickBench dark)
- Text: `#CCC`
- Bars: Adjusted for dark background
- Highlight: `#064663`

### Typography

- Font: **Inter** (same as ClickBench)
- Numbers: Monospace for alignment
- Headers: Bold, clear hierarchy

### Layout

- Sticky theme toggle (top right)
- Full-width tables
- Responsive grid for metadata
- Clean spacing and padding

## Usage

### Generate TPC-H Report

```bash
python3 scripts/benchmark_tpch.py --scale 1 --rounds 5
open tpch_results.html
```

### Generate TPC-DS Report

```bash
python3 scripts/benchmark_tpcds.py --scale 1 --rounds 5
open tpcds_results.html
```

### Output Files

- **HTML**: `tpch_results.html` / `tpcds_results.html`
- **JSON**: `tpch_results.json` / `tpcds_results.json`

## Comparison with ClickBench

| Feature | ClickBench | Our Implementation |
|---------|-----------|-------------------|
| **Theme Toggle** | ✅ Yes | ✅ Yes |
| **Bar Visualizations** | ✅ Yes | ✅ Yes |
| **Monospace Numbers** | ✅ Yes | ✅ Yes |
| **Inter Font** | ✅ Yes | ✅ Yes |
| **Dark Mode** | ✅ Yes (#04293A) | ✅ Yes (same color) |
| **Sticky Elements** | ✅ Yes | ✅ Yes |
| **Hover Effects** | ✅ Yes | ✅ Yes |
| **Zero Dependencies** | ✅ Yes | ✅ Yes |
| **Responsive** | ✅ Yes | ✅ Yes |

## Screenshots

### Light Mode
```
┌─────────────────────────────────────────────┐
│  🌓 Toggle Theme                             │
│                                              │
│  TPC-H Benchmark Results                    │
│  Java FE vs Rust FE Performance Comparison  │
│                                              │
│  ┌──────────┬────┬─────────┬──────────────┐│
│  │Benchmark │ SF1│ 5 Rounds│ Geom: 2.87x  ││
│  └──────────┴────┴─────────┴──────────────┘│
│                                              │
│  Query Execution Times (Lower is Better)    │
│  ┌──────┬──────────────┬──────────────┬───┐│
│  │Query │ Java FE      │ Rust FE      │ x ││
│  ├──────┼──────────────┼──────────────┼───┤│
│  │Q1    │████████ 2.45s│███ 0.82s     │2.9││
│  │Q2    │██████ 1.23s  │██ 0.46s      │2.7││
│  └──────┴──────────────┴──────────────┴───┘│
└─────────────────────────────────────────────┘
```

### Dark Mode
```
┌─────────────────────────────────────────────┐
│░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  🌓 Toggle   │
│                                              │
│  TPC-H Benchmark Results                    │
│  [ClickBench signature dark blue-green]     │
│                                              │
│  [Same layout with adjusted colors]         │
│  - Light text on dark background            │
│  - Adjusted bar colors for visibility       │
│  - Smooth hover transitions                 │
└─────────────────────────────────────────────┘
```

## Technical Implementation

### Zero Dependencies

Unlike the original implementation which used Chart.js, this version:
- ✅ **No JavaScript libraries** - Pure vanilla JS
- ✅ **No CSS frameworks** - Custom CSS with CSS variables
- ✅ **No build step** - Direct HTML generation
- ✅ **Fast loading** - Only Google Fonts CDN

### CSS Variables

All colors and styles use CSS variables for easy theming:

```css
:root {
    --color: black;
    --background-color: white;
    --bar-java-color: #FFA500;
    --bar-rust-color: #4CAF50;
}

[data-theme="dark"] {
    --color: #CCC;
    --background-color: #04293A;
    /* ... */
}
```

### Responsive Design

- Grid layout for metadata cards
- Full-width tables
- Mobile-friendly touch targets
- Readable on all screen sizes

## ClickBench References

- **Official Site**: https://benchmark.clickhouse.com/
- **GitHub Repo**: https://github.com/ClickHouse/ClickBench
- **Design Philosophy**: Minimal, data-focused, fast

## Why ClickBench Style?

1. **Industry Standard**: ClickBench is the de-facto standard for database benchmarking
2. **Clean & Simple**: No visual clutter, focus on results
3. **Instant Comparison**: Bar charts show relative performance at a glance
4. **Professional**: Used by major database vendors for comparisons
5. **Accessible**: Light/dark themes, responsive, readable

## Customization

To adjust visualization styles, edit `benchmark_clickbench.py`:

```python
# Change bar colors
--bar-java-color: #YOUR_COLOR;
--bar-rust-color: #YOUR_COLOR;

# Change theme colors
--background-color: #YOUR_COLOR;
--color: #YOUR_COLOR;
```

## Future Enhancements

Potential additions inspired by ClickBench:

- [ ] Multi-system comparison (>2 systems)
- [ ] Query filtering/selection
- [ ] URL hash state preservation
- [ ] Logarithmic scale bars
- [ ] Combined metrics (hot/cold/load/size)

## Conclusion

This implementation provides a **professional, ClickBench-style visualization** perfect for comparing Java FE vs Rust FE performance across TPC-H and TPC-DS benchmarks.

**Key Benefits:**
- ✅ Industry-standard design
- ✅ Zero dependencies
- ✅ Fast and responsive
- ✅ Light/dark themes
- ✅ Clean, data-focused presentation

Perfect for presentations, reports, and performance analysis! 📊
