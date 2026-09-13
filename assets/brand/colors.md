# Phenotype Fabric Brand Colors

## Primary Palette

| Name | Hex | RGB | Usage |
|------|-----|-----|-------|
| Deep Navy | `#1a1a2e` | `rgb(26, 26, 46)` | Primary background, app chrome |
| Teal Accent | `#00d4aa` | `rgb(0, 212, 170)` | Accent, highlights, active states, CTA |
| Purple Blue | `#6464c8` | `rgb(100, 100, 200)` | Secondary nodes, connections, secondary actions |
| Near White | `#f0f0f5` | `rgb(240, 240, 245)` | Primary text on dark backgrounds |

## Extended Palette

| Name | Hex | RGB | Usage |
|------|-----|-----|-------|
| Dark Text | `#2a2a3e` | `rgb(42, 42, 62)` | Text on light backgrounds |
| Subtle Gray | `#3a3a50` | `rgb(58, 58, 80)` | Borders, inactive states, placeholder text |
| Error Red | `#ef4444` | `rgb(239, 68, 68)` | Errors, warnings, destructive actions |
| Success Green | `#22c55e` | `rgb(34, 197, 94)` | Success states, confirmations |

## Background Variants

| Context | Hex | Notes |
|---------|-----|-------|
| Dark (default) | `#1a1a2e` | Main app, splash, icons |
| Dark elevated | `#222240` | Panels, modals, cards on dark |
| Light | `#f8f8fa` | Light theme background |
| Light elevated | `#ffffff` | Cards, sheets on light theme |

## Opacity Scale

Use opacity to create depth and hierarchy on dark backgrounds:

| Level | Opacity | Usage |
|-------|---------|-------|
| 100% | 1.0 | Primary text, icons |
| 80% | 0.8 | Secondary text |
| 60% | 0.6 | Tertiary text, placeholder |
| 40% | 0.4 | Disabled states |
| 20% | 0.2 | Subtle overlays |
| 10% | 0.1 | Background patterns |

## Color Usage Rules

1. **Never use teal accent for large filled areas** - It is an accent, not a background
2. **Dark backgrounds are the default** - Light backgrounds are opt-in via theme toggle
3. **Purple-blue is for connections and secondary elements** - Not for primary actions
4. **Error red is reserved for errors** - Never use for decorative purposes
5. **Maintain 4.5:1 minimum contrast ratio** between text and background (WCAG AA)
