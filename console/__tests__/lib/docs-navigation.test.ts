import { describe, it, expect } from "vitest";
import { docsNavigation } from "@/lib/docs-navigation";

describe("docsNavigation", () => {
  it("is a non-empty array of sections", () => {
    expect(Array.isArray(docsNavigation)).toBe(true);
    expect(docsNavigation.length).toBeGreaterThan(0);
  });

  it("every section has a title and non-empty items", () => {
    for (const section of docsNavigation) {
      expect(section.title).toBeTruthy();
      expect(section.items.length).toBeGreaterThan(0);
    }
  });

  it("every item has a title and href starting with /docs", () => {
    for (const section of docsNavigation) {
      for (const item of section.items) {
        expect(item.title).toBeTruthy();
        expect(item.href.startsWith("/docs")).toBe(true);
      }
    }
  });

  it("has no duplicate hrefs", () => {
    const hrefs = docsNavigation.flatMap((s) => s.items.map((i) => i.href));
    expect(new Set(hrefs).size).toBe(hrefs.length);
  });
});
