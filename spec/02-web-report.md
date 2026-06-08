# Specification: Interactive Webpage Report for Krebs Cycle Status

**Project:** krebslog
**Feature:** Webpage Report (`report web`)
**Status:** Draft v0.1
**Date:** 2026-06-07
**Author:** Grok (based on existing krebslog philosophy and spec)
**Mobile-first • Local-only • Fully static • Explorable**

---

## 1. Purpose & Goals

The webpage report provides a **rich, interactive, self-contained HTML experience** for exploring personal Krebs (TCA) cycle status, redox balance, energy flux, and related metabolic insights derived from `nutlog` + `repslog` data.

### Primary Goals
- Enable **deeper exploration** than static images or CLI text reports.
- Make the Krebs cycle **tangible and personal** through an interactive visual diagram.
- Maintain 100% alignment with krebslog philosophy: local-first, offline, transparent formulas, agent-friendly, minimal dependencies.
- Excellent **mobile experience** (primary form factor for many users after training/nutrition sessions).
- Serve as both a human-friendly dashboard **and** a machine-readable artifact (JSON embedded or adjacent).

### Secondary Goals
- Educational: Help users connect biochemistry concepts to their real data.
- Complement existing outputs: Can embed or link to generated `image krebs-cycle` PNG/SVG.
- Future-proof for Hermes agent consumption (the page or its data.json can be read by local tools).

---

## 2. Constraints & Design Principles

Must strictly follow krebslog core principles:

- **Local-only & offline**: No external network calls after generation. All data embedded or loaded from adjacent local files.
- **Static generation**: Output is plain HTML + CSS + JS + optional data.json + images. Runnable with `darkhttpd`, `python -m http.server`, or by opening `index.html` directly.
- **CLI is still primary**: The webpage is an *additional* output format (`--json` remains the contract for agents/scripts).
- **Transparency**: Every derived value (flux proxy, redox score, etc.) must expose its calculation method, inputs, assumptions, and limitations.
- **Mobile-first**: Design starts on small screens (320–640px). Desktop is an enhancement.
- **Touch-friendly**: Large tap targets (min 44×44px), bottom sheets instead of modals where possible, swipe gestures where natural.
- **Minimal dependencies**: Prefer vanilla JS + Tailwind via Play CDN (or fully embedded CSS). Avoid heavy frameworks.
- **Self-contained where practical**: Single-file HTML is ideal for simplicity; small folder structure acceptable if it improves maintainability.
- **Consistent with existing visuals**: Color palette, status indicators (Good/Moderate/Low), and Krebs cycle aesthetic should match `image krebs-cycle` output.

---

## 3. Target Users & Use Cases

**Primary users**
- The individual tracking their own metabolism (athlete, biohacker, or health-conscious person).
- Occasional coach or partner viewing on phone/tablet.

**Key use cases**
1. **Daily/weekly review** — Quickly see overall status + drill into specific cycle steps that are bottlenecks.
2. **Period comparison** — Understand how a training block or diet change affected flux/redox.
3. **Educational exploration** — Tap around the cycle to learn what each step means *in the context of their data*.
4. **Preparation for agent discussion** — Generate the page, then have Hermes or another agent analyze the embedded data.
5. **Sharing** — Export as PDF or send the folder to someone (still local).

---

## 4. Information Architecture

Organize information using **progressive disclosure** to avoid overwhelming the user on mobile:

### Level 0 — Global Context (always visible)
- Report title + selected period
- Prominent period selector (chips or dropdown)
- Overall status summary (1–2 sentences)

### Level 1 — Overview (first thing user sees)
- 4–6 high-level KPI cards in a responsive grid
- Mini sparklines or trend indicators
- Quick "What stands out" summary

### Level 2 — Visual Exploration (core value)
- **Interactive Krebs Cycle diagram** (hero element)
- Clicking nodes/arrows reveals personal data + explanations in a bottom sheet

### Level 3 — Details & Analysis
- Tabbed or accordion sections:
  - Inputs (Acetyl-CoA sources)
  - Outputs & Energy Carriers
  - Redox & Antioxidant Status
  - Training Demand on the Cycle
  - Anaplerosis / Cataplerosis support
- Clean data tables + small charts

### Level 4 — Trends, Correlations & Transparency
- Time-series charts
- Correlation views
- Full methodology + assumptions (expandable)

### Navigation Model (Mobile)
- **Sticky top bar** with title + period + menu
- **Bottom navigation** (5 tabs): Overview | Cycle | Metrics | Trends | Insights
- Or single long scroll with clear section headers + "jump to" links
- Bottom sheet for all detail views (preserves context on the diagram)

---

## 5. User Interface & Interaction Design

### Visual Style
- **Theme**: Dark mode by default (scientific, low eye strain). Optional light mode toggle.
- **Color palette** (consistent with existing images):
  - Good / High flux: `#22c55e` (green)
  - Moderate: `#eab308` (amber)
  - Low / Bottleneck: `#ef4444` (red)
  - Neutral / Background accents: cool grays + teal/cyan for cycle highlights
- **Typography**: Clean sans-serif. Large numbers for KPIs. Readable body text.
- **Cards**: Subtle shadows or borders, rounded corners, good padding.
- **Icons**: Simple line icons (SVG inline) for nutrition, training, redox, etc.

### Responsive Behavior
- **Mobile (base)**: Single column, bottom nav or scrollspy, bottom sheets for details, diagram sized to viewport with pinch-zoom if possible.
- **Tablet/Desktop (>768px)**: Optional two-column layout (diagram + details side-by-side), larger interactive elements, persistent sidebar for legend/filters.

### Key Interactive Elements
1. **Period Selector**: Preset chips (Today, Last 7 days, Last 30 days, This month) + date range picker (simple native or lightweight calendar).
2. **Krebs Cycle Diagram**:
   - SVG-based (hand-crafted for zero deps).
   - 8 main metabolites + key enzymes/arrows.
   - Color + stroke-width encoding of personal status/flux.
   - Click/tap → Bottom sheet with rich content.
   - Controls: "Highlight bottlenecks", "Show classic labels", "Reset view", Legend.
3. **Bottom Sheet** (for node details):
   - Draggable / swipe-to-dismiss.
   - Header with step name + status badge.
   - Your personal proxy value + trend.
   - Contribution breakdown (e.g., from carbs / fats / protein).
   - Short biochemical explanation + "Why it matters for you".
   - Small sparkline or mini chart.
   - Link to related full metrics section.
4. **Data Tables**: Sortable, filterable (client-side JS). Responsive (horizontal scroll on mobile or card conversion).
5. **Charts**: Use Chart.js (via CDN or local bundle) or pure SVG/Canvas for minimalism. Focus on line, bar, and radar charts.

---

## 6. Krebs Cycle Explorer — Core Feature

This is the **differentiating element** of the webpage report.

### Diagram Requirements
- Accurate high-level representation of the TCA cycle (citrate → isocitrate → α-ketoglutarate → succinyl-CoA → succinate → fumarate → malate → oxaloacetate → back to citrate).
- Visual encoding:
  - Node color = status of that step/intermediate for the user.
  - Arrow thickness + color = estimated relative flux through the reaction.
  - Optional overlay: "Antioxidant shield" layer or ROS indicators.
- Interaction:
  - Tap node or arrow → detail bottom sheet.
  - Long-press or info icon → educational popover.
- Two modes:
  - **Personalized Flux View** (default): Data-driven colors and thicknesses.
  - **Classic Educational View**: Standard textbook labels + user's current values as annotations.

### Detail Bottom Sheet Content (per node)
- Title + status
- Your estimated contribution / proxy value for the selected period
- Trend (last 7/30 days)
- Main inputs affecting this step (from nutrition data)
- Main outputs / downstream effects
- Training interaction (e.g., high aerobic demand increases flux here)
- Transparent calculation note (short version + "See full methodology")
- "Related metrics" quick links

---

## 7. Data Model & What the Page Needs

The generation command will produce a structured JSON payload (embedded in HTML or as `data.json`).

**Core data needed** (extending existing aggregates):

```json
{
  "meta": {
    "generated_at": "...",
    "period": { "start": "2026-05-08", "end": "2026-06-07", "label": "Last 30 days" },
    "sources": ["nutlog", "repslog"]
  },
  "kpis": {
    "krebs_flux_proxy": { "value": 78, "unit": "%", "status": "good", "trend": "up" },
    "redox_balance": { "value": 0.82, "status": "moderate", ... },
    ...
  },
  "krebs_cycle_steps": [
    {
      "id": "citrate_synthase",
      "name": "Citrate Synthase",
      "status": "good",
      "flux_proxy": 82,
      "personal_contribution": { "acetyl_coa_from_carbs": 45, ... },
      "explanation_short": "...",
      "calculation_note": "Weighted from acetyl-CoA availability + oxaloacetate regeneration"
    },
    ...
  ],
  "inputs": { ... },
  "outputs": { ... },
  "redox": { ... },
  "trends": {
    "daily": [ { "date": "...", "flux": 75, ... }, ... ]
  },
  "correlations": [...],
  "methodology": {
    "flux_model": "Description + formula summary",
    "assumptions": [...],
    "limitations": [...]
  }
}
```

The HTML/JS will consume this object for all dynamic content.

---

## 8. Technical Implementation Notes

- **HTML Generation**: New module or command handler in Rust (similar to existing report/image generators). Use a template engine (minijinja or simple string templating) or write HTML directly with `format!`.
- **Styling**: Tailwind CSS Play CDN (`https://tailwindcss.com/docs/installation/play-cdn`) — perfect for generated static reports. Include the script tag.
- **Interactivity**: Vanilla JavaScript only. No build step required.
- **Diagram**: Custom SVG in a `<div>` or standalone `.svg` file. JS attaches event listeners to `<g>` or `<path>` elements.
- **Charts** (optional but recommended): Include Chart.js via CDN for trends and correlations. Provide a graceful fallback (tables + sparklines) if user prefers zero external resources.
- **Single-file vs Folder**:
  - **Preferred for simplicity**: Single `krebs-report-YYYY-MM-DD.html` with everything embedded (data as JS object, small images as data URIs if needed).
  - **Alternative**: Small folder with `index.html` + `data.json` + `assets/` (images, css if not using CDN). Easier to update data without regenerating HTML.
- **Darkhttpd compatibility**: Works perfectly — just point it at the output directory or file.
- **Print / PDF**: Add `@media print` styles for clean output (hide nav, expand all sections, good typography).

---

## 9. Proposed CLI Command

```bash
# Generate interactive webpage report
krebslog report web --period "last 30 days" \
                    --output ~/reports/krebslog/ \
                    --include-images \
                    --theme dark

# Or more concise
krebslog web --date today --output ./my-report.html
```

**Flags** (in addition to global ones):
- `--period`, `--since`, `--until` (flexible date parsing, same as other commands)
- `--output` (file or directory)
- `--single-file` / `--folder` (output style)
- `--include-images` (embed or copy existing krebs-cycle images)
- `--theme` (dark|light|auto)
- `--embed-data` (default true for single-file)

Output example:
```
~/reports/krebslog/
└── krebs-status-2026-06-07/
    ├── index.html
    ├── data.json
    └── images/
        ├── krebs-cycle.png
        └── training-load.png
```

---

## 10. Serving the Report

```bash
# Simple options
darkhttpd ~/reports/krebslog/krebs-status-2026-06-07/ --port 8080
# or
python -m http.server 8080 --directory ~/reports/krebslog/krebs-status-2026-06-07/

# Then open http://localhost:8080 in any browser (phone or desktop on same network)
```

The page must work fully offline once loaded (all assets local).

---

## 11. Accessibility, Performance & Polish

- **Accessibility**: Semantic HTML, ARIA labels on interactive elements, sufficient color contrast (WCAG AA), keyboard navigation for diagram controls.
- **Performance**: Target < 500KB total for single-file version. Lazy-load non-critical sections if using multiple files. Optimize SVG.
- **Error states**: Graceful handling if data is incomplete for a period.
- **Empty states**: Helpful messaging when no data for a step or period.
- **Versioning**: Include generation timestamp and krebslog version in the footer.

---

## 12. Future Extensions (Out of Scope for v1)

- PWA capabilities (installable, offline caching of multiple reports).
- Side-by-side period comparison mode.
- Export to PDF with high-quality embedded diagram.
- Integration with `telegram` command (generate web report + post link/image).
- Agent-specific view (more raw data + JSON blocks visible).
- User-configurable weights in the flux model (via config, reflected in the page).
- More advanced visualizations (Sankey for energy flow, radar charts for balance).

---

## 13. Success Criteria

- A user can open the report on their phone and within 30 seconds understand their overall Krebs status and identify at least one actionable insight by exploring the cycle diagram.
- The page works completely locally via `darkhttpd` or direct file open.
- All derived numbers link back to transparent methodology.
- The generated artifact is also useful for LLM agents (clean data structure + explanations).

---

**This specification is ready to be implemented** as a new `report web` subcommand in krebslog, following the same patterns as `report daily` and `image krebs-cycle`.

It extends the existing visual capabilities into an **explorable, educational, mobile-friendly format** while perfectly preserving the project's local-first, transparent, and agent-centric philosophy.
