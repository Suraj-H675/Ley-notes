/**
 * Module shims. CodeMirror ships as ESM with `import type` chains; sometimes
 * Vitest's module resolver needs a nudge for side-effect-only modules.
 */

declare module '*.css';
declare module '*.svg' {
  const src: string;
  export default src;
}