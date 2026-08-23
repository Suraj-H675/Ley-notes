/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        background: 'hsl(var(--background))',
        foreground: 'hsl(var(--foreground))',
        card: {
          DEFAULT: 'hsl(var(--card))',
          foreground: 'hsl(var(--card-foreground))',
        },
        popover: {
          DEFAULT: 'hsl(var(--popover))',
          foreground: 'hsl(var(--popover-foreground))',
        },
        primary: {
          DEFAULT: 'hsl(var(--primary))',
          foreground: 'hsl(var(--primary-foreground))',
        },
        secondary: {
          DEFAULT: 'hsl(var(--secondary))',
          foreground: 'hsl(var(--secondary-foreground))',
        },
        muted: {
          DEFAULT: 'hsl(var(--muted))',
          foreground: 'hsl(var(--muted-foreground))',
        },
        accent: {
          DEFAULT: 'hsl(var(--accent))',
          foreground: 'hsl(var(--accent-foreground))',
        },
        destructive: {
          DEFAULT: 'hsl(var(--destructive))',
          foreground: 'hsl(var(--destructive-foreground))',
        },
        border: 'hsl(var(--border))',
        input: 'hsl(var(--input))',
        ring: 'hsl(var(--ring))',
        // Knowledge graph node colors — desaturated, restraint palette
        node: {
          document: 'hsl(41 34% 66%)',
          task: 'hsl(155 16% 62%)',
          project: 'hsl(210 20% 64%)',
          concept: 'hsl(25 30% 63%)',
        },
        edge: {
          wiki: 'hsl(205 18% 61%)',
          explicit: 'hsl(155 16% 60%)',
          depends: 'hsl(8 48% 59%)',
          related: 'hsl(42 5% 56%)',
        },
      },
      borderRadius: {
        lg: '8px',
        md: '6px',
        sm: '4px',
      },
      fontFamily: {
        sans: [
          'Geist',
          '-apple-system',
          'BlinkMacSystemFont',
          'Segoe UI',
          'system-ui',
          'sans-serif',
        ],
        serif: [
          'Iowan Old Style',
          'Palatino Linotype',
          'Palatino',
          'Book Antiqua',
          'Georgia',
          'serif',
        ],
        mono: [
          'Geist Mono',
          'JetBrains Mono',
          'SF Mono',
          'Menlo',
          'monospace',
        ],
      },
      fontSize: {
        // Editorial scale: tight display, comfortable body, small metadata
        'display': ['28px', { lineHeight: '34px', letterSpacing: '-0.015em' }],
        'title': ['20px', { lineHeight: '26px', letterSpacing: '-0.01em' }],
        'body': ['14px', { lineHeight: '22px' }],
        'meta': ['12px', { lineHeight: '16px', letterSpacing: '0' }],
        'micro': ['11px', { lineHeight: '14px', letterSpacing: '0.01em' }],
      },
      boxShadow: {
        // Tinted, not black
        'panel': '0 1px 0 hsl(220 10% 8% / 0.4), 0 0 0 1px hsl(220 10% 100% / 0.04)',
        'popover': '0 8px 24px hsl(220 20% 4% / 0.32), 0 0 0 1px hsl(220 10% 100% / 0.06)',
        'menu': '0 12px 32px hsl(220 20% 4% / 0.4), 0 0 0 1px hsl(220 10% 100% / 0.06)',
      },
      keyframes: {
        'fade-in': {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        'scale-in': {
          '0%': { opacity: '0', transform: 'scale(0.96)' },
          '100%': { opacity: '1', transform: 'scale(1)' },
        },
        'slide-down': {
          '0%': { opacity: '0', transform: 'translateY(-4px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
      },
      animation: {
        'fade-in': 'fade-in 150ms cubic-bezier(0.16, 1, 0.3, 1)',
        'scale-in': 'scale-in 120ms cubic-bezier(0.16, 1, 0.3, 1)',
        'slide-down': 'slide-down 150ms cubic-bezier(0.16, 1, 0.3, 1)',
      },
    },
  },
  plugins: [],
};
