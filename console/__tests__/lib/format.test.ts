import { describe, it, expect } from "vitest";
import { formatDuration, formatAge } from "@/lib/format";

describe("formatDuration", () => {
  it("formats seconds", () => {
    expect(formatDuration(12)).toBe("12s");
  });
  it("formats minutes", () => {
    expect(formatDuration(842)).toBe("14m");
  });
  it("formats hours", () => {
    expect(formatDuration(3661)).toBe("1h");
  });
  it("formats days", () => {
    expect(formatDuration(86400 * 3)).toBe("3d");
  });
  it("floors at minute boundary", () => {
    expect(formatDuration(3599)).toBe("59m");
  });
  it("floors at hour boundary", () => {
    expect(formatDuration(86399)).toBe("23h");
  });
});

describe("formatAge", () => {
  it("formats ISO timestamp to relative age", () => {
    const now = new Date("2026-05-18T14:14:00Z");
    const created = "2026-05-18T14:00:00Z";
    expect(formatAge(created, now)).toBe("14m");
  });
});
