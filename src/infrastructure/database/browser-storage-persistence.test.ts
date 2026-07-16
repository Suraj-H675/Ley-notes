import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  browserStoragePersistenceStatus,
  requestBrowserStoragePersistence,
} from './browser-local-vault';

describe('browser storage persistence', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('reports persistent storage when the browser has granted it', async () => {
    vi.stubGlobal('navigator', {
      storage: {
        persisted: vi.fn().mockResolvedValue(true),
        persist: vi.fn().mockResolvedValue(true),
      },
    });

    await expect(browserStoragePersistenceStatus()).resolves.toBe('persistent');
    await expect(requestBrowserStoragePersistence()).resolves.toBe('persistent');
  });

  it('keeps working honestly when persistence is declined or unsupported', async () => {
    vi.stubGlobal('navigator', {
      storage: {
        persisted: vi.fn().mockResolvedValue(false),
        persist: vi.fn().mockResolvedValue(false),
      },
    });

    await expect(browserStoragePersistenceStatus()).resolves.toBe('best-effort');
    await expect(requestBrowserStoragePersistence()).resolves.toBe('best-effort');

    vi.stubGlobal('navigator', {});
    await expect(browserStoragePersistenceStatus()).resolves.toBe('unavailable');
    await expect(requestBrowserStoragePersistence()).resolves.toBe('unavailable');
  });

  it('does not block vault startup when the browser storage API rejects', async () => {
    vi.stubGlobal('navigator', {
      storage: {
        persisted: vi.fn().mockRejectedValue(new DOMException('Denied', 'SecurityError')),
        persist: vi.fn().mockRejectedValue(new DOMException('Denied', 'SecurityError')),
      },
    });

    await expect(browserStoragePersistenceStatus()).resolves.toBe('unavailable');
    await expect(requestBrowserStoragePersistence()).resolves.toBe('unavailable');
  });
});
