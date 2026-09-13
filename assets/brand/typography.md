# Phenotype Fabric Typography

## Font Stack

```css
/* Primary (system font) */
font-family: -apple-system, BlinkMacSystemFont, "Helvetica Neue", Helvetica, "SF Pro Text", "Segoe UI", sans-serif;

/* Monospace (code, logs, terminal) */
font-family: "SF Mono", "Fira Code", "Cascadia Code", "JetBrains Mono", Menlo, Consolas, monospace;

/* Display (logos, splash, hero text) */
font-family: -apple-system, "Helvetica Neue", Helvetica, Arial, sans-serif;
font-weight: 300;  /* Light weight for elegance */
```

## Type Scale

| Name | Size | Weight | Line Height | Usage |
|------|------|--------|-------------|-------|
| Display | 48px | 300 (Light) | 1.1 | Splash screen, hero headings |
| H1 | 32px | 400 (Regular) | 1.2 | Page titles |
| H2 | 24px | 500 (Medium) | 1.3 | Section headings |
| H3 | 20px | 500 (Medium) | 1.4 | Subsection headings |
| Body | 14px | 400 (Regular) | 1.5 | Default text |
| Body Small | 12px | 400 (Regular) | 1.4 | Captions, metadata |
| Code | 13px | 400 (Regular) | 1.6 | Code blocks, terminal output |
| Label | 11px | 500 (Medium) | 1.2 | UI labels, badges |

## Rules

1. **Light weight for display text** - Use font-weight 300 for large headings and the logo
2. **Never use decorative fonts** - Phenotype Fabric uses system fonts exclusively
3. **Monospace for code** - Always use the monospace stack for code, file paths, and terminal output
4. **Minimum 12px body text** - Never go below 12px for readable content
5. **Line height scales with size** - Larger text gets tighter line-height, smaller text gets more

## Weight Assignments

| Weight | Name | Context |
|--------|------|---------|
| 300 | Light | Display headings, splash text, logo |
| 400 | Regular | Body text, descriptions |
| 500 | Medium | Subheadings, labels, UI elements |
| 600 | Semibold | Emphasis within body text |
| 700 | Bold | Avoid in UI; reserve for external docs |
