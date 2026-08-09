import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useNavStore } from "@/shared/state/nav";
import { useUIStore } from "@/shared/state/ui";
import { RecentPane } from "./RecentPane";

vi.mock("@/features/notes/usePages", () => ({
  useRecentPages: () => [
    {
      id: "page-1",
      title: "Responsive notes",
      path: "Responsive notes.md",
    },
  ],
}));

let mobileViewport = false;

describe("RecentPane responsive navigation", () => {
  beforeEach(() => {
    mobileViewport = false;
    vi.stubGlobal(
      "matchMedia",
      vi.fn((query: string) => ({
        matches: query === "(max-width: 767px)" && mobileViewport,
        media: query,
        onchange: null,
        addEventListener: () => undefined,
        removeEventListener: () => undefined,
        addListener: () => undefined,
        removeListener: () => undefined,
        dispatchEvent: () => true,
      })),
    );
    useNavStore.getState().reset();
    useNavStore.setState({ recentPages: ["page-1"] });
    useUIStore.getState().setSidebarOpen(true);
  });

  afterEach(() => vi.unstubAllGlobals());

  it("closes the drawer after opening a recent note on narrow screens", () => {
    mobileViewport = true;
    render(<RecentPane />);

    fireEvent.click(screen.getByRole("button", { name: "Responsive notes" }));

    expect(useNavStore.getState().activeTab).toBe("page-1");
    expect(useUIStore.getState().sidebarOpen).toBe(false);
  });

  it("keeps the persistent sidebar open after desktop navigation", () => {
    render(<RecentPane />);

    fireEvent.click(screen.getByRole("button", { name: "Responsive notes" }));

    expect(useNavStore.getState().activeTab).toBe("page-1");
    expect(useUIStore.getState().sidebarOpen).toBe(true);
  });
});
