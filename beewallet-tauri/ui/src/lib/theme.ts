/**
 * BeeWallet Theme - Exact match to Flutter beewallet theme.dart
 *
 * Design Philosophy:
 * - Honey-accented borders: The signature visual element
 * - Warm palettes: Parchment light, OLED-optimized dark
 * - 8pt spacing grid for consistency
 * - Minimal shadows, relies on borders and backgrounds
 */

export const colors = {
  // Primary - Honey Gold (constant across themes)
  honey: '#FBBF24',
  honeyLight: '#FDE68A',
  honeyDark: '#D97706',

  // Semantic Colors
  success: '#10B981',
  successLight: '#34D399',
  warning: '#F59E0B',
  warningLight: '#FBBF24',
  error: '#EF4444',
  errorLight: '#F87171',
  info: '#3B82F6',

  // Asset Brand Colors
  bitcoin: '#F7931A',
  lightning: '#FFD93D',
  usdt: '#26A17B',

  // Dark Theme (OLED-Optimized)
  dark: {
    background: '#0D0D0D',      // Pure black for OLED
    surface: '#141414',          // Very dark surface
    card: '#1C1C1E',             // iOS dark card
    cardElevated: '#252528',
    textPrimary: '#F5F5F5',      // Off-white
    textSecondary: '#A1A1AA',    // Zinc-400
    textTertiary: '#71717A',     // Zinc-500
    border: '#3F3F46',           // Zinc-700
    borderStrong: '#52525B',
    divider: '#27272A',
  },

  // Light Theme (Warm Parchment)
  light: {
    background: '#FFFBF4',       // Parchment - warm cream
    surface: '#FFFDF7',          // Warmer cream for surfaces
    card: '#FFFFFF',             // White cards
    cardElevated: '#FFFFFF',
    textPrimary: '#1F2937',      // Deep ink
    textSecondary: '#64748B',    // Slate-500
    textTertiary: '#94A3B8',     // Slate-400
    border: 'rgba(251, 191, 36, 0.35)',  // Honey-tinted! (magic sauce)
    borderStrong: 'rgba(251, 191, 36, 0.5)',
    divider: 'rgba(251, 191, 36, 0.25)',
  },
};

// 8pt spacing grid (BeeSpacing)
export const spacing = {
  xs: '4px',
  sm: '8px',
  md: '12px',
  df: '16px',
  lg: '20px',
  xl: '24px',
  xxl: '32px',
  xxxl: '48px',
};

// Border radius
export const radius = {
  sm: '6px',
  md: '10px',
  df: '12px',
  lg: '14px',
  xl: '20px',
  xxl: '24px',
};

// Icon sizes
export const iconSize = {
  sm: '16px',
  df: '20px',
  md: '24px',
  lg: '32px',
  xl: '48px',
};

// Typography
export const fonts = {
  mono: "'SF Mono', 'Fira Code', 'Consolas', monospace",
  sans: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
};

// Text sizes matching Flutter
export const textSize = {
  xs: '10px',
  sm: '12px',
  df: '14px',
  md: '16px',
  lg: '18px',
  xl: '20px',
  xxl: '24px',
  hero: '32px',
};

// CSS custom properties for theming
export function getThemeCSS(isDark = true) {
  const theme = isDark ? colors.dark : colors.light;
  // Dark mode uses solid borders, light mode uses honey-tinted
  const borderColor = isDark ? theme.border : 'rgba(251, 191, 36, 0.35)';
  const borderStrong = isDark ? theme.borderStrong : 'rgba(251, 191, 36, 0.5)';
  const dividerColor = isDark ? theme.divider : 'rgba(251, 191, 36, 0.25)';

  return `
    /* Primary Colors */
    --color-honey: ${colors.honey};
    --color-honey-light: ${colors.honeyLight};
    --color-honey-dark: ${colors.honeyDark};
    --color-honey-alpha: rgba(251, 191, 36, 0.35);
    --color-honey-alpha-strong: rgba(251, 191, 36, 0.5);
    --color-honey-alpha-light: rgba(251, 191, 36, 0.15);

    /* Theme Colors */
    --color-background: ${theme.background};
    --color-surface: ${theme.surface};
    --color-card: ${theme.card};
    --color-card-elevated: ${theme.cardElevated};
    --color-text: ${theme.textPrimary};
    --color-text-secondary: ${theme.textSecondary};
    --color-text-tertiary: ${theme.textTertiary};
    --color-border: ${borderColor};
    --color-border-strong: ${borderStrong};
    --color-divider: ${dividerColor};
    --color-ink: #1F2937;

    /* Semantic Colors */
    --color-success: ${colors.success};
    --color-success-light: ${colors.successLight};
    --color-success-muted: rgba(16, 185, 129, 0.15);
    --color-warning: ${colors.warning};
    --color-warning-light: ${colors.warningLight};
    --color-warning-muted: rgba(245, 158, 11, 0.15);
    --color-error: ${colors.error};
    --color-error-light: ${colors.errorLight};
    --color-error-muted: rgba(239, 68, 68, 0.15);
    --color-info: ${colors.info};
    --color-info-muted: rgba(59, 130, 246, 0.15);

    /* Asset Colors */
    --color-bitcoin: ${colors.bitcoin};
    --color-lightning: ${colors.lightning};

    /* Spacing (8pt grid) */
    --spacing-xs: ${spacing.xs};
    --spacing-sm: ${spacing.sm};
    --spacing-md: ${spacing.md};
    --spacing-df: ${spacing.df};
    --spacing-lg: ${spacing.lg};
    --spacing-xl: ${spacing.xl};
    --spacing-xxl: ${spacing.xxl};
    --spacing-xxxl: ${spacing.xxxl};

    /* Border Radius */
    --radius-sm: ${radius.sm};
    --radius-md: ${radius.md};
    --radius-df: ${radius.df};
    --radius-lg: ${radius.lg};
    --radius-xl: ${radius.xl};
    --radius-xxl: ${radius.xxl};

    /* Icon Sizes */
    --icon-sm: ${iconSize.sm};
    --icon-df: ${iconSize.df};
    --icon-md: ${iconSize.md};
    --icon-lg: ${iconSize.lg};
    --icon-xl: ${iconSize.xl};

    /* Typography */
    --font-mono: ${fonts.mono};
    --font-sans: ${fonts.sans};

    /* Text Sizes */
    --text-xs: ${textSize.xs};
    --text-sm: ${textSize.sm};
    --text-df: ${textSize.df};
    --text-md: ${textSize.md};
    --text-lg: ${textSize.lg};
    --text-xl: ${textSize.xl};
    --text-xxl: ${textSize.xxl};
    --text-hero: ${textSize.hero};

    /* Button Dimensions */
    --button-height: 52px;
    --button-radius: ${radius.lg};
  `;
}
