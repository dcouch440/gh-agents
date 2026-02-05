import {renderHook} from "@testing-library/react";
import {useOutputSchemaContext} from "./useOutputSchemaContext";

describe("useOutputSchemaContext", () => {
  it("throws when used outside OutputSchemaProvider", () => {
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    expect(() => renderHook(() => useOutputSchemaContext())).toThrow(
      "useOutputSchemaContext must be used within OutputSchemaProvider",
    );
    spy.mockRestore();
  });
});
