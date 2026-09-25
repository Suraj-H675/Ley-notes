/**
 * Native Ley application root.
 *
 * The public website is a separate Vite entry/build. This component is only
 * mounted inside Tauri and therefore has one storage mode: the native
 * filesystem vault currently used by the legacy desktop workspace.
 */

import { useEffect, useState } from 'react';
import { Layout } from '@/app/Layout';
import { db } from '@/infrastructure/database/db';
import { useUIStore, type Theme } from '@/shared/state/ui';
import { useNavStore } from '@/shared/state/nav';
import { VaultLauncher } from '@/features/vault/VaultLauncher';
import {
  chooseDesktopVault,
  refreshDesktopVault,
  restoreDesktopVault,
  startDesktopVaultWatcher,
  type DesktopVault,
  type VaultPathChange,
} from '@/infrastructure/vault/filesystem-vault';
import { startNavigationSession, stopNavigationSession } from '@/core/vault/navigation-session';

async function reconcileNavigation(): Promise<void> {
  const pages = (await db.pages.filter((page) => page.deletedAt === null).toArray())
    .sort((left, right) => right.updatedAt - left.updatedAt);
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
  return [
    ...new Set([
      ...nav.openTabs,
      ...[nav.activeTab, nav.primaryTab, nav.secondaryTab].filter(
        (id): id is string => Boolean(id),
      ),
    ]),
  ];
}

function pauseEditorAutosaveForAuthoritativeScan(): void {
  window.dispatchEvent(
    new CustomEvent('ley:vault-files-changed', {
      detail: { paths: [], changes: [], fullRescan: true },
    }),
  );
}

export function App() {
  const setTheme = useUIStore((state) => state.setTheme);
  const [ready, setReady] = useState(false);
  const [desktopVault, setDesktopVault] = useState<DesktopVault | null>(null);
  const [vaultBusy, setVaultBusy] = useState(false);
  const [vaultError, setVaultError] = useState<string | null>(null);
  const [watcherStatus, setWatcherStatus] = useState<
    'inactive' | 'starting' | 'watching' | 'error'
  >('inactive');
  const desktopVaultPath = desktopVault?.path;

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const restored = await restoreDesktopVault();
        if (!cancelled) {
          if (restored) useNavStore.getState().reset();
          setDesktopVault(restored);
        }
        const themeRow = await db.settings.get('theme');
        if (!cancelled && themeRow) setTheme(themeRow.value as Theme);
      } catch (error) {
        console.error('[app] native initialization failed:', error);
      } finally {
        if (!cancelled) setReady(true);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [setTheme]);

  useEffect(() => {
    if (!desktopVaultPath) return;
    let active = true;
    let dispose: () => void = () => undefined;
    let refreshTimer: number | null = null;
    let refreshInFlight = false;
    let refreshPending = false;
    let pendingChanges: VaultPathChange[] = [];

    queueMicrotask(() => {
      if (active) setWatcherStatus('starting');
    });

    const scheduleRefresh = () => {
      if (refreshTimer !== null) window.clearTimeout(refreshTimer);
      refreshTimer = window.setTimeout(() => {
        refreshTimer = null;
        if (refreshInFlight) return;
        refreshInFlight = true;
        refreshPending = false;
        const changes = pendingChanges;
        pendingChanges = [];
        void refreshDesktopVault({ openPageIds: openPageIds(), changes })
          .then(async (next) => {
            if (!active || !next) return;
            setDesktopVault(next);
            await reconcileNavigation();
          })
          .catch((error) => {
            console.error('[vault] Live refresh failed', error);
            if (active) setWatcherStatus('error');
          })
          .finally(() => {
            refreshInFlight = false;
            if (active && refreshPending) scheduleRefresh();
          });
      }, 250);
    };

    void startDesktopVaultWatcher((change) => {
      window.dispatchEvent(
        new CustomEvent('ley:vault-files-changed', { detail: change }),
      );
      refreshPending = true;
      pendingChanges.push(...change.changes);
      scheduleRefresh();
    })
      .then((stop) => {
        if (!active) {
          stop();
          return;
        }
        dispose = stop;
        setWatcherStatus('watching');
      })
      .catch((error) => {
        console.error('[vault] Could not start filesystem watcher', error);
        if (active) setWatcherStatus('error');
      });

    return () => {
      active = false;
      if (refreshTimer !== null) window.clearTimeout(refreshTimer);
      dispose();
    };
  }, [desktopVaultPath]);

  async function openDesktopVault(): Promise<DesktopVault | null> {
    setVaultBusy(true);
    setVaultError(null);
    const switching = desktopVault !== null;
    try {
      if (switching) await stopNavigationSession();
      const vault = await chooseDesktopVault();
      if (vault) {
        useNavStore.getState().reset();
        setDesktopVault(vault);
      } else if (switching) {
        void startNavigationSession().catch((error) =>
          console.error('[navigation] Could not resume workspace session', error),
        );
      }
      return vault;
    } catch (error) {
      if (switching) {
        void startNavigationSession().catch((cause) =>
          console.error('[navigation] Could not resume workspace session', cause),
        );
      }
      setVaultError(error instanceof Error ? error.message : String(error));
      return null;
    } finally {
      setVaultBusy(false);
    }
  }

  async function refreshActiveVault(): Promise<DesktopVault | null> {
    pauseEditorAutosaveForAuthoritativeScan();
    const next = await refreshDesktopVault({ openPageIds: openPageIds() });
    if (next) {
      setDesktopVault(next);
      await reconcileNavigation();
    }
    return next;
  }

  if (!ready) return null;
  if (!desktopVault) {
    return (
      <VaultLauncher busy={vaultBusy} error={vaultError} onOpen={openDesktopVault} />
    );
  }

  return (
    <Layout
      vaultKey={desktopVault.path}
      vaultName={desktopVault.name}
      watcherStatus={watcherStatus}
      onRefreshVault={refreshActiveVault}
      onSwitchVault={async () => {
        await openDesktopVault();
      }}
    />
  );
}
