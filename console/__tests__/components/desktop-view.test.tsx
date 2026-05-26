import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";

vi.mock("@novnc/novnc", () => {
  return {
    default: vi.fn().mockImplementation(() => ({
      scaleViewport: false,
      resizeSession: false,
      focusOnClick: false,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      disconnect: vi.fn(),
      get _target() {
        return document.createElement("div");
      },
    })),
  };
});

import { DesktopView } from "@/components/desktop-view";

describe("DesktopView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the VNC toolbar with connecting state", () => {
    render(<DesktopView sandboxId="sb-test-1" />);
    expect(screen.getByText(/connecting/i)).toBeDefined();
  });

  it("calls onConnectionChange when state changes", () => {
    const onChange = vi.fn();
    render(<DesktopView sandboxId="sb-test-1" onConnectionChange={onChange} />);
    expect(onChange).toHaveBeenCalledWith("connecting");
  });

  it("renders fullscreen button", () => {
    render(<DesktopView sandboxId="sb-test-1" />);
    expect(screen.getByRole("button", { name: /fullscreen/i })).toBeDefined();
  });
});
