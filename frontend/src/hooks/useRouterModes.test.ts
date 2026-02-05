import { renderHook, waitFor } from "@testing-library/react";
import { useRouterModes } from "./useRouterModes";
import { mockRouterMode } from "@/test/fixtures";

const { mockListByRouter } = vi.hoisted(() => ({
  mockListByRouter: vi.fn(),
}));

vi.mock("@/api", () => ({
  api: { routerModes: { listByRouter: mockListByRouter } },
}));

describe("useRouterModes", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("loads modes on mount", async () => {
    mockListByRouter.mockResolvedValue([mockRouterMode]);
    const { result } = renderHook(() => useRouterModes("router-001"));

    expect(result.current.loading).toBe(true);
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.modes).toEqual([mockRouterMode]);
    expect(result.current.error).toBeNull();
  });

  it("sets error on failure", async () => {
    mockListByRouter.mockRejectedValue(new Error("Load failed"));
    const { result } = renderHook(() => useRouterModes("router-001"));

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toBe("Load failed");
    expect(result.current.modes).toEqual([]);
  });

  it("reloads modes when reload is called", async () => {
    mockListByRouter.mockResolvedValue([mockRouterMode]);
    const { result } = renderHook(() => useRouterModes("router-001"));

    await waitFor(() => expect(result.current.loading).toBe(false));
    mockListByRouter.mockClear();
    mockListByRouter.mockResolvedValue([]);
    await result.current.reload();
    expect(mockListByRouter).toHaveBeenCalledOnce();
  });
});
