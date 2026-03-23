//! Design tokens - single source of truth for all colours, fonts, and spacing.

pub const C_BG_BASE: &str = "#101113";
pub const C_BG_SURFACE: &str = "#17191c";
pub const C_BG_ELEVATED: &str = "#1e2126";
pub const C_BORDER: &str = "#2d3138";
pub const C_BORDER_ACCENT: &str = "#3b4048";
pub const C_ACCENT_GREEN: &str = "#d4d4d8";
pub const C_ACCENT_BLUE: &str = "#a1a1aa";
pub const C_ACCENT_AMBER: &str = "#b8b8b0";
pub const C_TEXT_PRIMARY: &str = "#f5f5f5";
pub const C_TEXT_SECONDARY: &str = "#b4b8c0";
pub const C_TEXT_MUTED: &str = "#7d828d";
pub const FONT_SANS: &str = "'IBM Plex Sans', 'Segoe UI', system-ui, sans-serif";
pub const FONT_MONO: &str = "'JetBrains Mono', 'Cascadia Code', 'Consolas', monospace";

/// Global CSS injected once at the root of the application.
pub fn global_css() -> String {
    format!(
        r#"
        @import url('https://fonts.googleapis.com/css2?family=IBM+Plex+Sans:wght@300;400;500;600;700&family=JetBrains+Mono:wght@400;500;600&display=swap');

        * {{ box-sizing: border-box; margin: 0; padding: 0; }}

        ::-webkit-scrollbar {{ width: 5px; height: 5px; }}
        ::-webkit-scrollbar-track {{ background: transparent; }}
        ::-webkit-scrollbar-thumb {{ background: #3b4048; border-radius: 3px; }}
        ::-webkit-scrollbar-thumb:hover {{ background: #5a606a; }}

        .btn {{
            display: inline-flex;
            align-items: center;
            gap: 6px;
            border-radius: 8px;
            padding: 6px 14px;
            font-size: 12px;
            font-weight: 500;
            font-family: inherit;
            cursor: pointer;
            transition: all 150ms ease;
            outline: none;
            text-decoration: none;
            white-space: nowrap;
        }}
        .btn-ghost {{
            border: 1px solid #2d3138;
            background: transparent;
            color: #b4b8c0;
        }}
        .btn-ghost:hover {{
            border-color: #5a606a;
            background: rgba(245,245,245,0.05);
            color: #f5f5f5;
        }}
        .btn-primary {{
            border: 1px solid #5a606a;
            background: rgba(245,245,245,0.08);
            color: #eceef2;
        }}
        .btn-primary:hover {{
            background: rgba(245,245,245,0.14);
            color: #ffffff;
        }}
        .btn-danger {{
            border: 1px solid #5a3f43;
            background: transparent;
            color: #caa0a6;
        }}
        .btn-danger:hover {{
            background: rgba(168,97,107,0.2);
            color: #e5c4c8;
        }}

        .title-shell {{
            position: relative;
            z-index: 1100;
            overflow: visible;
            display: flex;
            flex-direction: column;
            flex-shrink: 0;
            background: linear-gradient(180deg, #1a1c20 0%, #16181b 100%);
            box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.03);
        }}
        .caption-row {{
            min-height: 32px;
            display: flex;
            align-items: stretch;
            justify-content: space-between;
            gap: 12px;
            padding-left: 10px;
            background: linear-gradient(180deg, #1f2226 0%, #191b1f 100%);
            font-family: "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif;
        }}
        .caption-left {{
            min-width: 0;
            display: flex;
            align-items: center;
            gap: 2px;
        }}
        .title-identity {{
            min-width: 0;
            display: inline-flex;
            align-items: center;
            gap: 8px;
            padding-right: 12px;
            margin-right: 2px;
            border-right: 1px solid #2a2d33;
        }}
        .title-product {{
            font-size: 12px;
            font-weight: 600;
            color: #eef0f3;
            letter-spacing: 0.1px;
        }}
        .title-live-indicator {{
            font-size: 10px;
            font-weight: 700;
            letter-spacing: 0.08em;
            text-transform: uppercase;
        }}
        .menu-bar {{
            position: relative;
            display: inline-flex;
            align-items: stretch;
            gap: 0;
            z-index: 1;
        }}
        .menu-column {{
            position: relative;
            display: inline-flex;
            align-items: stretch;
        }}
        .menu-trigger {{
            min-height: 32px;
            display: inline-flex;
            align-items: center;
            justify-content: center;
            padding: 0 11px;
            border: 0;
            border-radius: 0;
            background: transparent;
            color: #d3d7de;
            font-size: 12px;
            font-weight: 400;
            cursor: pointer;
            transition: background 100ms ease, color 100ms ease;
        }}
        .menu-trigger:hover {{
            background: #25292f;
            color: #f5f5f5;
        }}
        .menu-trigger.active {{
            background: #2e333b;
            color: #f5f5f5;
            box-shadow: inset 1px 0 0 #434953, inset -1px 0 0 #434953, inset 0 1px 0 #505764;
        }}
        .menu-popover {{
            position: absolute;
            top: calc(100% - 1px);
            left: 0;
            min-width: 240px;
            padding: 4px 0;
            border: 1px solid #4a5059;
            border-radius: 0;
            background: #23262b;
            box-shadow: 0 16px 36px rgba(0, 0, 0, 0.42), inset 0 1px 0 rgba(255, 255, 255, 0.04);
            z-index: 1150;
            display: flex;
            flex-direction: column;
            gap: 0;
        }}
        .menu-entry {{
            width: 100%;
            min-height: 29px;
            display: grid;
            grid-template-columns: minmax(0, 1fr) auto;
            align-items: center;
            gap: 18px;
            padding: 5px 14px;
            border: 0;
            border-radius: 0;
            background: #23262b;
            color: #edf0f4;
            text-align: left;
            font-size: 12px;
            cursor: pointer;
            transition: background 100ms ease, color 100ms ease;
        }}
        .menu-entry:hover {{
            background: #39414a;
        }}
        .menu-entry-label {{
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }}
        .menu-shortcut {{
            color: #a8afba;
            font-size: 10px;
            font-family: {font_mono};
            letter-spacing: 0.01em;
        }}
        .menu-entry.disabled {{
            background: #23262b;
            color: #6f7681;
            cursor: not-allowed;
        }}
        .menu-entry.disabled .menu-shortcut {{
            color: #606772;
        }}
        .menu-entry.disabled:hover {{
            background: #23262b;
        }}
        .menu-trigger:focus-visible,
        .menu-entry:focus-visible,
        .caption-btn:focus-visible {{
            outline: 1px solid #7a8290;
            outline-offset: -1px;
        }}
        .caption-controls {{
            display: inline-flex;
            align-items: stretch;
            margin-left: auto;
        }}
        .caption-btn {{
            width: 46px;
            min-height: 32px;
            border: 0;
            border-radius: 0;
            background: transparent;
            color: #c7ccd5;
            display: inline-flex;
            align-items: center;
            justify-content: center;
            cursor: pointer;
            transition: background 100ms ease, color 100ms ease;
        }}
        .caption-btn:hover {{
            background: #2a2d33;
            color: #f5f5f5;
        }}
        .caption-btn.close:hover {{
            background: #c42b1c;
            color: #ffffff;
        }}

        .command-palette-overlay {{
            position: absolute;
            inset: 0;
            z-index: 1200;
            display: flex;
            align-items: flex-start;
            justify-content: center;
            padding: 72px 20px 20px;
            background: rgba(10, 11, 13, 0.58);
            backdrop-filter: blur(6px);
        }}
        .command-palette {{
            width: min(760px, 100%);
            max-height: min(78vh, 760px);
            display: flex;
            flex-direction: column;
            overflow: hidden;
            border: 1px solid #3b4048;
            border-radius: 16px;
            background: linear-gradient(180deg, #1a1c20 0%, #131518 100%);
            box-shadow: 0 28px 80px rgba(0, 0, 0, 0.45);
        }}
        .command-palette-search {{
            display: flex;
            align-items: center;
            gap: 10px;
            padding: 14px 16px;
            border-bottom: 1px solid #2d3138;
            background: rgba(255,255,255,0.02);
        }}
        .command-palette-close {{
            border: 1px solid #2d3138;
            border-radius: 6px;
            background: #101113;
            color: #7d828d;
            padding: 3px 7px;
            font-size: 10px;
            font-family: {font_mono};
            cursor: pointer;
        }}
        .command-palette-results {{
            flex: 1;
            overflow-y: auto;
            padding: 10px;
            display: grid;
            gap: 10px;
        }}
        .command-palette-group {{
            display: grid;
            gap: 6px;
        }}
        .command-palette-group-label {{
            padding: 4px 8px;
            font-size: 10px;
            font-weight: 700;
            letter-spacing: 0.08em;
            text-transform: uppercase;
            color: #7d828d;
        }}
        .command-palette-item {{
            width: 100%;
            display: flex;
            align-items: center;
            justify-content: space-between;
            gap: 10px;
            padding: 10px 12px;
            border: 1px solid #2d3138;
            border-radius: 10px;
            background: #17191c;
            color: #f5f5f5;
            text-align: left;
            cursor: pointer;
            transition: all 120ms ease;
        }}
        .command-palette-item:hover {{
            border-color: #5a606a;
            background: #1e2126;
        }}
        .command-palette-footer {{
            display: flex;
            align-items: center;
            justify-content: space-between;
            gap: 12px;
            padding: 10px 14px 12px;
            border-top: 1px solid #2d3138;
            color: #7d828d;
            font-size: 10px;
            font-family: {font_mono};
        }}
        .shortcut-badge {{
            flex-shrink: 0;
            border: 1px solid #3b4048;
            border-radius: 999px;
            padding: 4px 8px;
            background: #101113;
            color: #b4b8c0;
            font-size: 10px;
            font-family: {font_mono};
            white-space: nowrap;
        }}
        .shortcut-badge-live {{
            border-color: #5a606a;
            background: rgba(245,245,245,0.08);
            color: #f5f5f5;
        }}
        .settings-overlay {{
            width: min(860px, 100%);
        }}
        .settings-header {{
            justify-content: space-between;
        }}
        .settings-results {{
            gap: 8px;
        }}
        .settings-row {{
            display: flex;
            align-items: center;
            justify-content: space-between;
            gap: 16px;
            padding: 12px;
            border: 1px solid #2d3138;
            border-radius: 12px;
            background: rgba(255,255,255,0.02);
        }}
        .settings-row-actions {{
            display: flex;
            align-items: center;
            justify-content: flex-end;
            gap: 8px;
            flex-wrap: wrap;
        }}
        .shortcut-capture {{
            min-width: 76px;
            text-align: center;
        }}
        .shortcut-capture.active {{
            border-color: #5a606a;
            background: rgba(245,245,245,0.08);
            color: #f5f5f5;
        }}

        @media (max-width: 920px) {{
            .settings-header,
            .settings-row {{
                flex-direction: column;
                align-items: stretch;
            }}

            .settings-row-actions {{
                justify-content: flex-start;
            }}

            .command-palette-footer {{
                flex-direction: column;
                align-items: flex-start;
            }}
        }}

        .il-tabs {{
            display: flex;
            align-items: stretch;
            gap: 4px;
            overflow-x: auto;
            padding: 8px 10px 7px;
            background: #141619;
        }}
        .il-tab {{
            min-width: 140px;
            max-width: 220px;
            display: inline-flex;
            align-items: center;
            gap: 8px;
            padding: 6px 8px 6px 10px;
            border-radius: 8px;
            border: 1px solid #2d3138;
            background: #17191c;
            color: #b4b8c0;
            cursor: pointer;
            transition: all 120ms ease;
        }}
        .il-tab:hover {{
            border-color: #3b4048;
            background: #1e2126;
        }}
        .il-tab.active {{
            border-color: #5a606a;
            background: rgba(245,245,245,0.09);
            color: #f5f5f5;
        }}
        .tab-close {{
            width: 16px;
            height: 16px;
            border-radius: 4px;
            border: 1px solid transparent;
            background: transparent;
            color: #7d828d;
            display: inline-flex;
            align-items: center;
            justify-content: center;
            cursor: pointer;
            transition: all 120ms ease;
            flex-shrink: 0;
        }}
        .tab-close:hover {{
            border-color: #5a606a;
            background: #101113;
            color: #e2e5eb;
        }}

        .panel-header {{
            font-size: 10px;
            font-weight: 700;
            line-height: 1;
            letter-spacing: 1.2px;
            text-transform: uppercase;
            color: #7d828d;
            padding: 2px 14px 0px;
            min-height: 39px;
            box-sizing: border-box;
            border-bottom: 1px solid #2d3138;
            display: flex;
            align-items: center;
            justify-content: space-between;
            flex-shrink: 0;
        }}
        .badge {{
            font-size: 10px;
            font-weight: 600;
            padding: 1px 7px;
            border-radius: 999px;
            background: #1e2126;
            color: #8b919d;
            border: 1px solid #2d3138;
        }}
        .panel-header-detail {{
            display: inline-flex;
            align-items: center;
            min-width: 0;
            max-width: 320px;
            font-family: {font_mono};
            font-weight: 600;
            letter-spacing: 0;
            text-transform: none;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }}
        .resize-handle {{
            width: 8px;
            flex-shrink: 0;
            position: relative;
            margin-left: -4px;
            margin-right: -4px;
            z-index: 2;
            cursor: col-resize;
            background: transparent;
        }}
        .resize-handle::after {{
            content: "";
            position: absolute;
            top: 0;
            bottom: 0;
            left: 50%;
            transform: translateX(-50%);
            width: 1px;
            background: #2d3138;
            transition: background 120ms ease, width 120ms ease;
        }}
        .resize-handle:hover::after,
        .resize-handle.active::after {{
            background: #5a606a;
            width: 2px;
        }}

        .asm-item {{
            margin: 0 8px 6px;
            border-radius: 10px;
            border: 1px solid #2d3138;
            background: #17191c;
            padding: 9px 10px;
            cursor: pointer;
            transition: all 150ms ease;
            text-align: left;
            width: calc(100% - 16px);
        }}
        .asm-item:hover {{
            border-color: #3b4048;
            background: #1e2126;
        }}
        .asm-item.selected {{
            border-color: #8f96a2;
            background: rgba(245,245,245,0.06);
        }}

        .method-item {{
            margin: 0 8px 5px;
            border-radius: 8px;
            border: 1px solid transparent;
            background: transparent;
            padding: 8px 10px;
            cursor: pointer;
            transition: all 120ms ease;
            text-align: left;
            width: calc(100% - 16px);
        }}
        .method-item:hover {{
            border-color: #2d3138;
            background: #1e2126;
        }}
        .method-item.selected {{
            border-color: #8f96a266;
            background: rgba(245,245,245,0.05);
        }}

        .finding-item {{
            margin: 0 8px 6px;
            border-radius: 8px;
            border: 1px solid #2d3138;
            background: #17191c;
            padding: 9px 10px;
            cursor: pointer;
            transition: all 120ms ease;
            text-align: left;
            width: calc(100% - 16px);
        }}
        .finding-item:hover {{
            border-color: #3b4048;
            background: #1e2126;
        }}
        .finding-item.selected {{
            border-color: #90908a99;
            background: rgba(245,245,245,0.05);
        }}

        .sev-badge {{
            font-size: 9px;
            font-weight: 700;
            letter-spacing: 0.5px;
            text-transform: uppercase;
            padding: 2px 7px;
            border-radius: 999px;
            border: 1px solid currentColor;
        }}

        .il-row {{
            display: grid;
            grid-template-columns: 72px 120px 1fr;
            gap: 12px;
            padding: 3px 6px;
            border-radius: 4px;
            font-size: 12px;
            line-height: 1.7;
            transition: background 80ms;
        }}
        .il-row:hover {{
            background: rgba(245,245,245,0.04);
        }}
        .il-row.highlighted {{
            background: rgba(100, 180, 255, 0.12);
        }}

        .csharp-source {{
            white-space: pre-wrap;
            word-break: break-word;
            tab-size: 4;
            font-variant-ligatures: none;
        }}
        .csharp-line {{
            display: block;
            border-radius: 4px;
            padding: 0 4px;
        }}
        .csharp-line.highlighted {{
            background: rgba(100, 180, 255, 0.12);
        }}
        .csharp-token.keyword {{
            color: #7fc4ff;
        }}
        .csharp-token.type {{
            color: #8fd2c5;
        }}
        .csharp-token.string {{
            color: #d7c38a;
        }}
        .csharp-token.comment {{
            color: #6f8b73;
        }}
        .csharp-token.number {{
            color: #d9a77a;
        }}
        .csharp-token.preprocessor {{
            color: #b89ef0;
        }}
        .csharp-token.attribute {{
            color: #c7abda;
        }}

        .empty-state {{
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            height: 100%;
            gap: 10px;
            padding: 32px 16px;
            color: #6f7580;
            text-align: center;
        }}
        .empty-state svg {{
            opacity: 0.35;
        }}
        .empty-state p {{
            font-size: 12px;
            line-height: 1.5;
            color: #7d828d;
            max-width: 160px;
        }}

        .pulse {{
            animation: pulse 1.5s cubic-bezier(0.4, 0, 0.6, 1) infinite;
        }}
        @keyframes pulse {{
            0%, 100% {{ opacity: 1; }}
            50% {{ opacity: 0.4; }}
        }}

        .drag-region {{
            -webkit-app-region: drag;
            position: relative;
            overflow: visible;
            isolation: isolate;
        }}
        .no-drag {{
            -webkit-app-region: no-drag;
        }}

        .drop-overlay {{
            position: absolute;
            inset: 0;
            background: rgba(30, 33, 38, 0.85);
            backdrop-filter: blur(2px);
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            z-index: 1000;
            pointer-events: none;
            opacity: 0;
            transition: opacity 200ms ease;
        }}
        .drop-overlay.visible {{
            opacity: 1;
        }}
        .drop-zone {{
            border: 2px dashed #3b4048;
            border-radius: 16px;
            padding: 60px 80px;
            background: rgba(23, 25, 28, 0.9);
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 16px;
        }}
        .drop-zone.drag-over {{
            border-color: #5a606a;
            background: rgba(45, 49, 56, 0.95);
        }}
        "#,
        font_mono = FONT_MONO,
    )
}
