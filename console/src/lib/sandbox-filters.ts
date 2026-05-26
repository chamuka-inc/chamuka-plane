import type { Sandbox } from "@/api/types";

/** Sandboxes connectable from the quickstart live list (spec: status=ready only). */
export function filterReadySandboxes(sandboxes: Sandbox[]): Sandbox[] {
  return sandboxes.filter((s) => s.status === "ready");
}
