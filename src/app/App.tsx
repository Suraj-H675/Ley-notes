/**
 * App root. Loads settings from IndexedDB into the UI store, then renders Layout.
 */

import { useEffect, useState } from 'react';
import { Layout } from '@/app/Layout';
import { db } from '@/infrastructure/database/db';
import { seedIfEmpty } from '@/infrastructure/database/seed';
import { useUIStore, type Theme } from '@/shared/state/ui';
import { useNavStore } from '@/shared/state/nav';
import { VaultLauncher } from '@/features/vault/VaultLauncher';
import { WebVaultLauncher } from '@/features/vault/WebVaultLauncher';
import {
  chooseBrowserFolderVault,
  chooseDesktopVault,
  deactivateFilesystemVault,
  isDesktopApp,
  isBrowserFolderSupported,
  refreshBrowserFolderVault,
  refreshDesktopVault,
  restoreBrowserFolderVault,
  restoreDesktopVault,
  startDesktopVaultWatcher,
  type DesktopVault,
  type VaultPathChange,
} from '@/infrastructure/vault/filesystem-vault';
import {
  activeDataKind,
  BROWSER_LOCAL_KIND,
  clearActiveVaultData,
  markActiveDataKind,
  requestBrowserStoragePersistence,
  restoreBrowserLocalVault,
  stashBrowserLocalVault,
} from '@/infrastructure/database/browser-local-vault';
import { startNavigationSession, stopNavigationSession } from '@/core/vault/navigation-session';

type VaultMode = 'desktop' | 'browser-folder' | 'browser-local';
const WEB_VAULT_MODE_KEY = 'ley:web-vault-mode';

async function ensureBrowserLocalData(): Promise<void> {
  const kind = await activeDataKind();
  if (kind?.startsWith('filesystem:')) {
    if (!await restoreBrowserLocalVault()) await clearActiveVaultData();
  }
  await seedIfEmpty();
  await markActiveDataKind(BROWSER_LOCAL_KIND);
}

async function reconcileNavigation(): Promise<void> {
  const pages = (await db.pages.filter((page) => page.deletedAt === null).toArray()).sort((left, right) => right.updatedAt - left.updatedAt);
  const nav = useNavStore.getState();
  nav.reconcile(new Set(pages.map((page) => page.id)));
  const reconciled = useNavStore.getState();
  if (reconciled.openTabs.length === 0 && pages[0]) {
    reconciled.openPage(pages[0].id);
    reconciled.pushRecent(pages[0].id);
  }
}

function openPageIds(): string[] {
  const nav = useNavStore.getState();
  return [...new Set([
    ...nav.openTabs,
    ...[nav.activeTab, nav.primaryTab, nav.secondaryTab].filter((id): id is string => Boolean(id)),
  ])];
}

function pauseEditorAutosaveForAuthoritativeScan(): void {
  window.dispatchEvent(new CustomEvent('ley:vault-files-changed', {
    detail: { paths: [], changes: [], fullRescan: true },
  }));
}

export function App() {
  const setTheme = useUIStore((s) => s.setTheme);
  const [ready, setReady] = useState(false);
  const [desktopVault, setDesktopVault] = useState<DesktopVault | null>(null);
  const [vaultMode, setVaultMode] = useState<VaultMode | null>(null);
  const [vaultBusy, setVaultBusy] = useState(false);
  const [vaultError, setVaultError] = useState<string | null>(null);
  const [returnVault, setReturnVault] = useState<{ mode: Exclude<VaultMode, 'desktop'>; name: string } | null>(null);
  const [watcherStatus, setWatcherStatus] = useState<'inactive' | 'starting' | 'watching' | 'error'>('inactive');
  const desktopVaultPath = desktopVault?.path;

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
            if (!restored) await markActiveDataKind('filesystem:unavailable');
          } else {
            const kind = await activeDataKind();
            const hasLegacyLocalData = !kind && await db.pages.count() > 0;
            if (preferred === 'browser-local' || kind === BROWSER_LOCAL_KIND || hasLegacyLocalData) {
              await ensureBrowserLocalData();
              localStorage.setItem(WEB_VAULT_MODE_KEY, 'browser-local');
              if (!cancelled) setVaultMode('browser-local');
            }
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
    // Desktop changes are driven by the native watcher. Browser folder handles
    // have no watcher, so focus is their authoritative external-change check.
    if (vaultBusy || !desktopVault || vaultMode !== 'browser-folder') return;
    const refresh = async () => {
      pauseEditorAutosaveForAuthoritativeScan();
      const continuity = { openPageIds: openPageIds() };
      const next = await refreshBrowserFolderVault(continuity);
      if (next) {
        setDesktopVault(next);
        await reconcileNavigation();
      }
    };
    window.addEventListener('focus', refresh);
    return () => window.removeEventListener('focus', refresh);
  }, [desktopVault, vaultBusy, vaultMode]);

  useEffect(() => {
    if (!desktopVaultPath || vaultMode !== 'desktop') return;
    let active = true;
    let dispose: () => void = () => undefined;
    let refreshTimer: number | null = null;
    let refreshInFlight = false;
    let refreshPending = false;
    let pendingChanges: VaultPathChange[] = [];
    queueMicrotask(() => { if (active) setWatcherStatus('starting'); });
    const scheduleRefresh = () => {
      if (refreshTimer !== null) window.clearTimeout(refreshTimer);
      refreshTimer = window.setTimeout(() => {
        refreshTimer = null;
        if (refreshInFlight) return;
        refreshInFlight = true;
        refreshPending = false;
        const changes = pendingChanges;
        pendingChanges = [];
        void refreshDesktopVault({ openPageIds: openPageIds(), changes }).then(async (next) => {
          if (!active || !next) return;
          setDesktopVault(next);
          await reconcileNavigation();
        }).catch((error) => {
          console.error('[vault] Live refresh failed', error);
          if (active) setWatcherStatus('error');
        }).finally(() => {
          refreshInFlight = false;
          if (active && refreshPending) scheduleRefresh();
        });
      }, 250);
    };
    void startDesktopVaultWatcher((change) => {
      window.dispatchEvent(new CustomEvent('ley:vault-files-changed', { detail: change }));
      refreshPending = true;
      pendingChanges.push(...change.changes);
      scheduleRefresh();
    }).then((stop) => {
      if (!active) { stop(); return; }
      dispose = stop;
      setWatcherStatus('watching');
    }).catch((error) => {
      console.error('[vault] Could not start filesystem watcher', error);
      if (active) setWatcherStatus('error');
    });
    return () => {
      active = false;
      if (refreshTimer !== null) window.clearTimeout(refreshTimer);
      dispose();
    };
  }, [desktopVaultPath, vaultMode]);

  async function openDesktopVault(): Promise<DesktopVault | null> {
    setVaultBusy(true);
    setVaultError(null);
    const switching = vaultMode === 'desktop';
    try {
      if (switching) await stopNavigationSession();
      const vault = await chooseDesktopVault();
      if (vault) {
        useNavStore.getState().reset();
        setDesktopVault(vault);
        setVaultMode('desktop');
      } else if (switching) {
        void startNavigationSession().catch((error) => console.error('[navigation] Could not resume workspace session', error));
      }
      return vault;
    } catch (error) {
      if (switching) void startNavigationSession().catch((cause) => console.error('[navigation] Could not resume workspace session', cause));
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
      await stashBrowserLocalVault();
      const vault = await chooseBrowserFolderVault();
      if (vault) {
        useNavStore.getState().reset();
        localStorage.setItem(WEB_VAULT_MODE_KEY, 'browser-folder');
        setDesktopVault(vault);
        setVaultMode('browser-folder');
        setReturnVault(null);
      }
    } catch (error) {
      setVaultError(error instanceof Error ? error.message : String(error));
    } finally {
      setVaultBusy(false);
    }
  }

  async function activateBrowserLocalVault(): Promise<void> {
    setVaultBusy(true);
    setVaultError(null);
    try {
      deactivateFilesystemVault();
      await requestBrowserStoragePersistence();
      await ensureBrowserLocalData();
      useNavStore.getState().reset();
      localStorage.setItem(WEB_VAULT_MODE_KEY, 'browser-local');
      setDesktopVault(null);
      setVaultMode('browser-local');
      setReturnVault(null);
    } catch (error) {
      setVaultError(error instanceof Error ? error.message : String(error));
    } finally {
      setVaultBusy(false);
    }
  }

  async function refreshActiveVault(): Promise<DesktopVault | null> {
    if (vaultMode === 'desktop' || vaultMode === 'browser-folder') pauseEditorAutosaveForAuthoritativeScan();
    const continuity = { openPageIds: openPageIds() };
    const next = vaultMode === 'desktop'
      ? await refreshDesktopVault(continuity)
      : vaultMode === 'browser-folder'
        ? await refreshBrowserFolderVault(continuity)
        : null;
    if (next) setDesktopVault(next);
    if (next) await reconcileNavigation();
    return next;
  }

  async function switchVault(): Promise<void> {
    if (!vaultMode) return;
    if (vaultMode === 'desktop') {
      await openDesktopVault();
      return;
    }
    await stopNavigationSession();
    if (vaultMode === 'browser-local') await stashBrowserLocalVault();
    setReturnVault({ mode: vaultMode, name: desktopVault?.name ?? 'Browser-local vault' });
    localStorage.removeItem(WEB_VAULT_MODE_KEY);
    useNavStore.getState().reset();
    setDesktopVault(null);
    setVaultMode(null);
  }

  async function returnToCurrentVault(): Promise<void> {
    if (!returnVault) return;
    setVaultBusy(true);
    setVaultError(null);
    try {
      localStorage.setItem(WEB_VAULT_MODE_KEY, returnVault.mode === 'browser-folder' ? 'browser-folder' : 'browser-local');
      useNavStore.getState().reset();
      if (returnVault.mode === 'browser-folder') {
        const vault = await refreshBrowserFolderVault({ openPageIds: openPageIds() });
        if (!vault) throw new Error('The browser folder is no longer available. Choose it again to continue.');
        setDesktopVault(vault);
      } else {
        setDesktopVault(null);
      }
      setVaultMode(returnVault.mode);
      setReturnVault(null);
    } catch (error) {
      localStorage.removeItem(WEB_VAULT_MODE_KEY);
      setVaultError(error instanceof Error ? error.message : String(error));
    } finally {
      setVaultBusy(false);
    }
  }

  if (!ready) return null;
  if (isDesktopApp() && !desktopVault) {
    return <VaultLauncher busy={vaultBusy} error={vaultError} onOpen={openDesktopVault} />;
  }
  if (!isDesktopApp() && !vaultMode) {
    return <WebVaultLauncher folderSupported={isBrowserFolderSupported()} busy={vaultBusy} error={vaultError} returnLabel={returnVault?.name} onReturn={returnVault ? () => void returnToCurrentVault() : undefined} onOpenFolder={() => void openBrowserFolder()} onBrowserLocal={() => void activateBrowserLocalVault()} />;
  }
  if (!vaultMode) return null;
  return <Layout vaultMode={vaultMode} vaultKey={desktopVault?.path ?? BROWSER_LOCAL_KIND} vaultName={desktopVault?.name ?? 'Browser-local vault'} watcherStatus={watcherStatus} onRefreshVault={refreshActiveVault} onSwitchVault={switchVault} />;
}
