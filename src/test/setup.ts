/**
 * Vitest setup. Loads happy-dom (browser-like env), fake-indexeddb (for Dexie),
 * and jest-dom matchers for Testing Library.
 */

import '@testing-library/jest-dom/vitest';

// fake-indexeddb is auto-installed by Dexie when indexedDB is missing;
// we only need to import it to register the polyfill early.
import 'fake-indexeddb/auto';