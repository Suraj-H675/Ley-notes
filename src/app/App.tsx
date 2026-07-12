/**
 * App root. Loads settings from IndexedDB into the UI store, then renders Layout.
 */

import { useEffect, useState } from 'react';
import { Layout } from '@/app/Layout';
import { db } from '@/infrastructure/database/db';
import { seedIfEmpty } from '@/infrastructure/database/seed';
import { useUIStore, type Theme } from '@/shared/state/ui';
import { useNavStore } from '@/shared/state/nav';
import { useLiveQuery } from 'dexie-react-hooks';
import { VaultLauncher } from '@/features/vault/VaultLauncher';
import { WebVaultLauncher } from '@/features/vault/WebVaultLauncher';
import {
  chooseBrowserFolderVault,
  chooseDesktopVault,
  isDesktopApp,
  isBrowserFolderSupported,
  refreshBrowserFolderVault,
  refreshDesktopVault,
  restoreBrowserFolderVault,
  restoreDesktopVault,
  type DesktopVault,
} from '@/infrastructure/vault/filesystem-vault';

type VaultMode = 'desktop' | 'browser-folder' | 'browser-local';
const WEB_VAULT_MODE_KEY = 'ley:web-vault-mode';

export function App() {
  const setTheme = useUIStore((s) => s.setTheme);
  const [ready, setReady] = useState(false);
  const [desktopVault, setDesktopVault] = useState<DesktopVault | null>(null);
  const [vaultMode, setVaultMode] = useState<VaultMode | null>(null);
  const [vaultBusy, setVaultBusy] = useState(false);
  const [vaultError, setVaultError] = useState<string | null>(null);

  // Auto-open the most recently updated page on first launch, so the user
  // lands somewhere useful. If the vault is empty, the welcome page will be
  // the first result from seedIfEmpty.
  const firstPage = useLiveQuery(async () => {
    const pages = await db.pages
      .filter((p) => p.deletedAt === null)
      .reverse()
      .sortBy('updatedAt');
    return pages[0] ?? null;
  }, []);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        if (isDesktopApp()) {
          const restored = await restoreDesktopVault();
          if (!cancelled) {
            if (restored) useNavStore.getState().reset();
            setDesktopVault(restored);
            if (restored) setVaultMode('desktop');
          }
        } else {
          const preferred = localStorage.getItem(WEB_VAULT_MODE_KEY);
          if (preferred === 'browser-folder') {
            const restored = await restoreBrowserFolderVault();
            if (!cancelled && restored) {
              useNavStore.getState().reset();
              setDesktopVault(restored);
              setVaultMode('browser-folder');
            }
          } else if (preferred === 'browser-local' || await db.pages.count() > 0) {
            await seedIfEmpty();
            localStorage.setItem(WEB_VAULT_MODE_KEY, 'browser-local');
            if (!cancelled) setVaultMode('browser-local');
          }
        }
        const themeRow = await db.settings.get('theme');
        if (!cancelled && themeRow) {
          setTheme(themeRow.value as Theme);
        }
      } catch (e) {
        console.error('[app] init failed:', e);
      } finally {
        if (!cancelled) setReady(true);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [setTheme]);

  useEffect(() => {
    if (!desktopVault || (vaultMode !== 'desktop' && vaultMode !== 'browser-folder')) return;
    const refresh = async () => {
      const next = vaultMode === 'desktop' ? await refreshDesktopVault() : await refreshBrowserFolderVault();
      if (next) setDesktopVault(next);
    };
    window.addEventListener('focus', refresh);
    return () => window.removeEventListener('focus', refresh);
  }, [desktopVault, vaultMode]);

  async function openDesktopVault(): Promise<DesktopVault | null> {
    setVaultBusy(true);
    setVaultError(null);
    try {
      const vault = await chooseDesktopVault();
      if (vault) { useNavStore.getState().reset(); setDesktopVault(vault); setVaultMode('desktop'); }
      return vault;
    } catch (error) {
      setVaultError(error instanceof Error ? error.message : String(error));
      return null;
    } finally {
      setVaultBusy(false);
    }
  }

  async function openBrowserFolder(): Promise<void> {
    setVaultBusy(true);
    setVaultError(null);
    try {
      const vault = await chooseBrowserFolderVault();
      if (vault) {
        useNavStore.getState().reset();
        localStorage.setItem(WEB_VAULT_MODE_KEY, 'browser-folder');
        setDesktopVault(vault);
        setVaultMode('browser-folder');
      }
    } catch (error) {
      setVaultError(error instanceof Error ? error.message : String(error));
    } finally {
      setVaultBusy(false);
    }
  }

  async function activateBrowserLocalVault(): Promise<void> {
    setVaultBusy(true);
    await seedIfEmpty();
    localStorage.setItem(WEB_VAULT_MODE_KEY, 'browser-local');
    setVaultMode('browser-local');
    setVaultBusy(false);
  }

  // Open the first page once it loads. Only fires once per app session.
  useEffect(() => {
    if (!firstPage) return;
    const nav = useNavStore.getState();
    if (nav.openTabs.length === 0) {
      nav.openPage(firstPage.id);
      nav.pushRecent(firstPage.id);
    }
    // Intentionally only run when firstPage.id changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [firstPage?.id]);

  if (!ready) return null;
  if (isDesktopApp() && !desktopVault) {
    return <VaultLauncher busy={vaultBusy} error={vaultError} onOpen={openDesktopVault} />;
  }
  if (!isDesktopApp() && !vaultMode) {
    return <WebVaultLauncher folderSupported={isBrowserFolderSupported()} busy={vaultBusy} error={vaultError} onOpenFolder={() => void openBrowserFolder()} onBrowserLocal={() => void activateBrowserLocalVault()} />;
  }
  return <Layout />;
}
