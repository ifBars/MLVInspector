# MLVInspector.Dioxus Design

## Purpose

This document keeps new UI work aligned with the inspector that already exists in this repo. It
is not a prompt to redesign the product. Treat it as the grounding document for extending the
current Windows desktop UI with minimal stylistic drift.

## Source Of Truth

When this guidance and implementation differ, prefer the implementation and update this file.

- `src/components/theme.rs`
  - canonical colors, fonts, shared classes, overlay styling, tab styling, empty states, scrollbars,
    and common interaction timing
- `src/components/title_bar.rs`
  - Windows-native title bar, menu bar, and caption controls
- `src/components/explorer_panel.rs`
  - left-side explorer structure and header treatment
- `src/components/il_view_panel.rs`
  - center-pane tab system, code/data presentation, and dual IL/C# viewing modes
- `src/components/findings_panel.rs`
  - findings list density, muted severity presentation, and detail-pane treatment
- `src/components/explorer_metadata.rs` and `src/components/explorer_tree.rs`
  - dense metadata cards, tree rows, and drill-down patterns

## Product Feel

The app is a dense desktop inspector, not a marketing site and not a generic web dashboard.

- Prefer a quiet, technical, Windows-native feel.
- Keep the UI compact, legible, and information-first.
- Use restrained contrast and muted accenting instead of bright brand colors.
- Favor stability and clarity over novelty.
- Preserve the current three-panel workspace mental model.

## Color System

Use the existing tokens from `src/components/theme.rs` before introducing new literal values.

```css
--bg-base: #101113;
--bg-surface: #17191c;
--bg-elevated: #1e2126;
--border-base: #2d3138;
--border-accent: #3b4048;
--border-strong: #5a606a;
--border-highlight: #8f96a2;
--text-primary: #f5f5f5;
--text-secondary: #b4b8c0;
--text-muted: #7d828d;
--text-dim: #6f7580;
--accent-blue: #a1a1aa;
--accent-green: #d4d4d8;
--accent-amber: #b8b8b0;
--accent-danger: #caa0a6;
```

- `C_BG_BASE` is for the app frame and primary workspace background.
- `C_BG_SURFACE` is for secondary panels and list containers.
- `C_BG_ELEVATED` is for cards, selected containers, and inset surfaced content.
- `C_BORDER` is for default separation.
- `C_BORDER_ACCENT` is for slightly stronger framed detail blocks.
- `C_TEXT_PRIMARY`, `C_TEXT_SECONDARY`, and `C_TEXT_MUTED` map to active content, secondary
  descriptions, and captions or empty-state support copy.
- `C_ACCENT_GREEN`, `C_ACCENT_BLUE`, and `C_ACCENT_AMBER` are intentionally desaturated. Treat
  them as restrained UI accents, not saturated brand colors.

Do not add vivid blues, purples, neon greens, rainbow severity palettes, or bright gradients unless
the user explicitly asks for a visual redesign.

## Typography

The current typography is part of the product identity.

```css
--font-sans: 'IBM Plex Sans', 'Segoe UI', system-ui, sans-serif;
--font-mono: 'JetBrains Mono', 'Cascadia Code', 'Consolas', monospace;
```

- Use `FONT_SANS` for standard UI text.
- Use `FONT_MONO` for code, IL, signatures, shortcuts, badges with technical content, and paths.
- Preserve the existing size hierarchy:
  - panel labels and chrome labels sit around `10px`
  - dense list/detail content sits around `10px` to `12px`
  - key item titles sit around `11px` to `13px`
- Uppercase labels are used sparingly for panel headers, metadata labels, and compact badges.
- Avoid oversized headings or spacious marketing-style typography.

## Spacing And Radius

```css
--space-xxs: 3px;
--space-xs: 4px;
--space-sm: 6px;
--space-md: 8px;
--space-lg: 10px;
--space-xl: 12px;
--space-2xl: 14px;
--space-3xl: 16px;

--radius-sm: 4px;
--radius-md: 6px;
--radius-lg: 8px;
--radius-xl: 10px;
--radius-2xl: 12px;
--radius-full: 999px;
```

- Panel padding is usually `14px`.
- Card padding is usually `9px` to `10px`.
- Button padding is usually `6px 14px`.
- Icon gaps are usually `6px` to `10px`.
- Section gaps are usually around `16px`.
- Keep radii modest: small controls use `4px` to `6px`, buttons and tabs use about `8px`, and
  overlays may use `16px`.

## Layout Model

The main app structure should remain recognizable.

- Title bar at the top with native-feel menus and caption buttons
- Three-panel workspace:
  - explorer on the left
  - IL/C# center pane in the middle
  - findings on the right when enabled
- Status bar anchored at the bottom
- Overlays layered over the workspace instead of navigating away from it

New features should usually fit into this model before inventing a new shell or navigation system.

## Shared Patterns

### Panel Headers

- Reuse the existing `panel-header` and `badge` language whenever possible.
- Panel headers are compact, uppercase, muted, and separated with a bottom border.
- Keep header actions small and quiet.

### Lists And Cards

- Existing list items use subtle borders, dark surfaced backgrounds, and small radii.
- Hover states slightly raise contrast through border/background changes.
- Selected states use modest border emphasis and light surface brightening.
- Density matters: prefer compact rows and cards over tall, airy layouts.

### Buttons And Toolbars

- Buttons are compact, text-first commands with restrained hover states.
- Toolbar buttons should keep stable dimensions, quiet borders, and clear active/disabled states.
- Destructive actions use muted danger coloring and should not dominate the workspace.

### Tabs

- Reuse the tab styling in `theme.rs` and `il_view_panel.rs`.
- Tabs are compact, technical, and text-first.
- Active tabs brighten slightly; they do not become loud or heavily animated.

### Overlays

- Command palette and settings overlays are the model for modal work.
- Use a blurred dark scrim, centered surfaced container, and compact internal spacing.
- Avoid full-screen wizard styling unless the user asks for it.

### Empty States

- Keep empty states concise and quiet.
- Use a simple line icon or understated visual marker.
- Limit supporting copy to short, practical guidance.

### Code And Data Views

- IL, C#, signatures, metadata values, offsets, and shortcuts should stay monospaced.
- Highlighting should remain muted and readable against dark surfaces.
- Prefer framed code/data blocks over decorative containers.

## Interaction Rules

- Keep transitions fast and subtle, typically around `100ms` to `150ms`.
- Favor border, background, and text-color changes over large movement or scale effects.
- Focus states should remain visible and explicit.
- Drag/drop affordances should look infrastructural, not playful.
- Avoid flashy animation, bouncing, or decorative motion.
- Respect reduced-motion preferences when adding new animation.

## Iconography

- Use simple inline SVG icons with thin strokes that match the current title bar and empty states.
- Keep icons functional and understated.
- Do not use emoji as icons.

## Severity And Semantic States

- Findings severity colors should stay muted and technical.
- Reuse `severity_color` in `src/components/helpers.rs` for finding severities.
- If a new semantic state is needed, keep it low-saturation and document it in `theme.rs`.

## Implementation Rules For Agents

- Start from existing tokens, helpers, and shared classes before inventing new styling.
- If a new reusable visual pattern emerges, add it to `src/components/theme.rs` instead of
  scattering one-off inline styles across multiple files.
- Match the density, spacing, and typography of neighboring components.
- Keep `src/app.rs` as wiring only; place new UI regions in focused component modules.
- Extend existing workspace patterns before adding new sidebars, top-level shells, or floating UI.
- Prefer subtle gradients and layered neutrals already used in the title bar and overlays over new
  decorative treatments.
- When adding a new panel or card, make it feel like it belongs beside Explorer, IL View, and
  Findings without needing explanation.

## Avoid

- Bright brand-color accents or rainbow semantic palettes
- Rounded, soft consumer-SaaS styling that fights the current inspector feel
- Large empty paddings, oversized cards, or landing-page composition
- Heavy shadows, glassmorphism, or animated chrome
- Multiple competing navigation systems
- Replacing monospaced technical presentation with overly stylized UI text
- Marketing-page guidance inside this desktop inspector repo

## Agent Checklist

Before shipping UI changes, verify:

1. The work uses existing theme tokens or extends them deliberately in `src/components/theme.rs`.
2. The new UI still feels like part of the same Windows desktop inspector.
3. Spacing and typography match nearby components.
4. Hover, selected, and focus states are present but restrained.
5. Any new semantic color is muted and justified by actual product meaning.
