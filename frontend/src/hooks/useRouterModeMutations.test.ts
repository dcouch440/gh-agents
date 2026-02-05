import { renderHook, waitFor } from "@testing-library/react";
import { useRouterModeMutations } from "./useRouterModeMutations";
import { mockRouterMode, mockTool } from "@/test/fixtures";

const {
  mockCreateForRouter,
  mockUpdate,
  mockDelete,
  mockGetTools,
  mockSetTools,
} = vi.hoisted(() => ({
  mockCreateForRouter: vi.fn(),
  mockUpdate: vi.fn(),
  mockDelete: vi.fn(),
  mockGetTools: vi.fn(),
  mockSetTools: vi.fn(),
}));

vi.mock("@/api", () => ({
  api: {
    routerModes: {
      createForRouter: mockCreateForRouter,
      update: mockUpdate,
      delete: mockDelete,
      getTools: mockGetTools,
      setTools: mockSetTools,
    },
  },
}));

describe("useRouterModeMutations", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("createMode", () => {
    it("creates a mode successfully", async () => {
      mockCreateForRouter.mockResolvedValue(mockRouterMode);
      const { result } = renderHook(() => useRouterModeMutations());

      expect(result.current.creating).toBe(false);

      const mode = await result.current.createMode("router-001", {
        mode_key: "test_mode",
        display_name: "Test Mode",
        description: "A test mode",
        system_prompt: "You are helpful",
      });

      expect(mode).toEqual(mockRouterMode);
      expect(result.current.creating).toBe(false);
      expect(mockCreateForRouter).toHaveBeenCalledWith("router-001", {
        mode_key: "test_mode",
        display_name: "Test Mode",
        description: "A test mode",
        system_prompt: "You are helpful",
      });
    });

    it("handles create error", async () => {
      mockCreateForRouter.mockRejectedValue(new Error("Create failed"));
      const { result } = renderHook(() => useRouterModeMutations());

      await expect(
        result.current.createMode("router-001", {
          mode_key: "test_mode",
          display_name: "Test Mode",
          description: "A test mode",
          system_prompt: "You are helpful",
        })
      ).rejects.toThrow("Create failed");

      await waitFor(() => expect(result.current.creating).toBe(false));
    });
  });

  describe("updateMode", () => {
    it("updates a mode successfully", async () => {
      const updated = { ...mockRouterMode, display_name: "Updated Mode" };
      mockUpdate.mockResolvedValue(updated);
      const { result } = renderHook(() => useRouterModeMutations());

      const mode = await result.current.updateMode("mode-001", {
        display_name: "Updated Mode",
      });

      expect(mode).toEqual(updated);
      expect(result.current.updating).toBe(false);
    });

    it("handles update error", async () => {
      mockUpdate.mockRejectedValue(new Error("Update failed"));
      const { result } = renderHook(() => useRouterModeMutations());

      await expect(
        result.current.updateMode("mode-001", {
          display_name: "Updated Mode",
        })
      ).rejects.toThrow("Update failed");

      await waitFor(() => expect(result.current.updating).toBe(false));
    });
  });

  describe("deleteMode", () => {
    it("deletes a mode successfully", async () => {
      mockDelete.mockResolvedValue(undefined);
      const { result } = renderHook(() => useRouterModeMutations());

      await result.current.deleteMode("mode-001");

      expect(result.current.deleting).toBe(false);
      expect(mockDelete).toHaveBeenCalledWith("mode-001");
    });

    it("handles delete error", async () => {
      mockDelete.mockRejectedValue(new Error("Delete failed"));
      const { result } = renderHook(() => useRouterModeMutations());

      await expect(result.current.deleteMode("mode-001")).rejects.toThrow(
        "Delete failed"
      );

      await waitFor(() => expect(result.current.deleting).toBe(false));
    });
  });

  describe("loadModeTools", () => {
    it("loads mode tools successfully", async () => {
      mockGetTools.mockResolvedValue([mockTool]);
      const { result } = renderHook(() => useRouterModeMutations());

      expect(result.current.toolsError).toBeNull();

      const tools = await result.current.loadModeTools("mode-001");

      expect(tools).toEqual([mockTool]);
      expect(result.current.loadingTools).toBe(false);
      expect(result.current.toolsError).toBeNull();
    });

    it("handles load tools error", async () => {
      mockGetTools.mockRejectedValue(new Error("Load failed"));
      const { result } = renderHook(() => useRouterModeMutations());

      await expect(result.current.loadModeTools("mode-001")).rejects.toThrow(
        "Load failed"
      );

      await waitFor(() => {
        expect(result.current.loadingTools).toBe(false);
        expect(result.current.toolsError).toBe("Load failed");
      });
    });
  });

  describe("saveModeTools", () => {
    it("saves mode tools successfully", async () => {
      mockSetTools.mockResolvedValue(undefined);
      const { result } = renderHook(() => useRouterModeMutations());

      await result.current.saveModeTools("mode-001", {
        tool_ids: ["tool-001", "tool-002"],
      });

      expect(result.current.savingTools).toBe(false);
      expect(result.current.toolsError).toBeNull();
      expect(mockSetTools).toHaveBeenCalledWith("mode-001", {
        tool_ids: ["tool-001", "tool-002"],
      });
    });

    it("handles save tools error", async () => {
      mockSetTools.mockRejectedValue(new Error("Save failed"));
      const { result } = renderHook(() => useRouterModeMutations());

      await expect(
        result.current.saveModeTools("mode-001", {
          tool_ids: ["tool-001"],
        })
      ).rejects.toThrow("Save failed");

      await waitFor(() => {
        expect(result.current.savingTools).toBe(false);
        expect(result.current.toolsError).toBe("Save failed");
      });
    });
  });
});
