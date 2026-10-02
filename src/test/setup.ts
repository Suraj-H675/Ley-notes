/**
 * Vitest setup. Loads happy-dom (browser-like env), fake-indexeddb for the
 * explicit legacy-browser recovery tests, and jest-dom matchers.
 */

import '@testing-library/jest-dom/vitest';

import 'fake-indexeddb/auto';
