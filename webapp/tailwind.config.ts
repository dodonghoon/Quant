import type { Config } from 'tailwindcss';

const config: Config = {
  content: [
    './src/pages/**/*.{js,ts,jsx,tsx,mdx}',
    './src/components/**/*.{js,ts,jsx,tsx,mdx}',
    './src/app/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // 트레이딩 대시보드 테마
        bg: {
          primary: '#0f1117',
          secondary: '#1a1d29',
          tertiary: '#252836',
        },
        accent: {
          blue: '#3b82f6',
          cyan: '#06b6d4',
        },
        profit: '#22c55e',
        loss: '#ef4444',
        warning: '#f59e0b',
        neutral: '#6b7280',
      },
      fontFamily: {
        mono: ['JetBrains Mono', 'Fira Code', 'monospace'],
      },
    },
  },
  plugins: [],
};

export default config;
