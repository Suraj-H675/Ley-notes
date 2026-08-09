import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useMediaQuery } from "./useMediaQuery";

describe("useMediaQuery", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("updates when the browser crosses the requested breakpoint", () => {
    let matches = false;
    const listeners = new Set<EventListener>();
    vi.stubGlobal(
      "matchMedia",
      vi.fn(
        () =>
          ({
            matches,
            media: "(max-width: 767px)",
            onchange: null,
            addEventListener: (
              _type: string,
              listener: EventListenerOrEventListenerObject,
            ) => {
              if (typeof listener === "function") listeners.add(listener);
            },
            removeEventListener: (
              _type: string,
              listener: EventListenerOrEventListenerObject,
            ) => {
              if (typeof listener === "function") listeners.delete(listener);
            },
            addListener: () => undefined,
            removeListener: () => undefined,
            dispatchEvent: () => true,
          }) satisfies MediaQueryList,
      ),
    );

    const { result, unmount } = renderHook(() =>
      useMediaQuery("(max-width: 767px)"),
    );
    expect(result.current).toBe(false);

    act(() => {
      matches = true;
      listeners.forEach((notify) => notify(new Event("change")));
    });
    expect(result.current).toBe(true);

    unmount();
    expect(listeners).toHaveLength(0);
  });
});
