import { describe, it, expect } from "vitest";
import {
  formatDuration,
  formatBytes,
  formatCpuHours,
  formatCost,
  formatAge,
  formatHeartbeatAge,
} from "@/lib/format";

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
});

describe("formatBytes", () => {
  it("formats bytes", () => {
    expect(formatBytes(512)).toBe("512 B");
  });
  it("formats kilobytes", () => {
    expect(formatBytes(1024 * 50)).toBe("50 KB");
  });
  it("formats megabytes", () => {
    expect(formatBytes(1024 * 1024 * 142)).toBe("142 MB");
  });
  it("formats gigabytes", () => {
    expect(formatBytes(1024 * 1024 * 1024 * 2.5)).toBe("2.5 GB");
  });
});

describe("formatCpuHours", () => {
  it("converts nanoseconds to hours", () => {
    expect(formatCpuHours(3600000000000)).toBe("1.0h");
  });
  it("formats partial hours", () => {
    expect(formatCpuHours(5400000000000)).toBe("1.5h");
  });
});

describe("formatCost", () => {
  it("formats dollars", () => {
    expect(formatCost(0.84)).toBe("$0.84");
  });
  it("formats zero", () => {
    expect(formatCost(0)).toBe("$0.00");
  });
  it("formats large amounts", () => {
    expect(formatCost(12.5)).toBe("$12.50");
  });
});

describe("formatAge", () => {
  it("formats ISO timestamp to relative age", () => {
    const now = new Date("2026-05-18T14:14:00Z");
    const created = "2026-05-18T14:00:00Z";
    expect(formatAge(created, now)).toBe("14m");
  });
});

describe("formatHeartbeatAge", () => {
  it("formats seconds age with color hint", () => {
    expect(formatHeartbeatAge(3)).toEqual({ text: "3s ago", status: "healthy" });
  });
  it("marks stale heartbeats", () => {
    expect(formatHeartbeatAge(35)).toEqual({ text: "35s ago", status: "stale" });
  });
  it("marks disconnected heartbeats", () => {
    expect(formatHeartbeatAge(120)).toEqual({ text: "2m ago", status: "disconnected" });
  });
});
