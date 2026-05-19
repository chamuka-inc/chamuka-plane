import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RuntimeSwitcher } from "@/components/runtime-switcher";

describe("RuntimeSwitcher", () => {
  it("renders CLI and Desktop options", () => {
    render(<RuntimeSwitcher selected="cli" onChange={() => {}} />);
    const group = screen.getByRole("group", { name: "Environment" });
    expect(group.querySelectorAll("button").length).toBe(2);
    expect(screen.getByRole("button", { name: /CLI/i })).toBeDefined();
    expect(screen.getByRole("button", { name: /Desktop/i })).toBeDefined();
  });

  it("calls onChange when Desktop is selected", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<RuntimeSwitcher selected="cli" onChange={onChange} />);
    await user.click(screen.getByRole("button", { name: /Desktop/i }));
    expect(onChange).toHaveBeenCalledWith("desktop");
  });
});
