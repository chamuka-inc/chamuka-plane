import type { Sandbox } from "@/api/types";
import { formatAge } from "@/lib/format";
import Link from "next/link";

type ActivityItem = {
  sandbox: Sandbox;
  event: string;
  time: string;
};

function deriveActivity(sandboxes: Sandbox[]): ActivityItem[] {
  const items: ActivityItem[] = [];
  for (const s of sandboxes) {
    if (s.ready_at) {
      items.push({ sandbox: s, event: "became ready", time: s.ready_at });
    }
    if (s.terminated_at) {
      items.push({ sandbox: s, event: "terminated", time: s.terminated_at });
    }
    items.push({
      sandbox: s,
      event: `created · ${s.runtime}`,
      time: s.created_at,
    });
  }
  return items.sort((a, b) => new Date(b.time).getTime() - new Date(a.time).getTime()).slice(0, 8);
}

const eventDotColor: Record<string, string> = {
  "became ready": "bg-status-green",
  terminated: "bg-text-4",
};

type ActivityFeedProps = {
  sandboxes: Sandbox[];
};

export function ActivityFeed({ sandboxes }: ActivityFeedProps) {
  const items = deriveActivity(sandboxes);

  return (
    <div className="border border-border rounded-lg p-5">
      <h3 className="text-sm font-medium mb-4">Activity</h3>
      <div className="space-y-4">
        {items.map((item, i) => (
          <div key={`${item.sandbox.id}-${item.event}-${i}`} className="flex items-start gap-3">
            <span
              className={`w-2 h-2 rounded-full mt-1.5 ${
                eventDotColor[item.event] ??
                (item.event.startsWith("created") ? "bg-accent-blue" : "bg-accent-pink")
              }`}
            />
            <div>
              <div className="text-sm">
                <Link
                  href={`/sandboxes/${item.sandbox.id}`}
                  className="font-mono font-medium hover:underline"
                >
                  {item.sandbox.id}
                </Link>{" "}
                <span className="text-text-2">{item.event}</span>
              </div>
              <div className="text-xs text-text-4">{formatAge(item.time)}</div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
