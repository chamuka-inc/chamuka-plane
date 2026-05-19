"use client";

import { use } from "react";
import Link from "next/link";
import { useSandbox } from "@/api/hooks";
import { TerminalView } from "@/components/terminal-view";
import { statusBorderColor, statusTextColor } from "@/lib/sandbox-status";

export default function TerminalPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const { data: sandbox } = useSandbox(id);

  return (
    <div className="flex flex-col h-[calc(100dvh-4rem)]">
      <div className="px-4 sm:px-6 py-3 border-b border-border">
        <div className="text-sm text-text-3 truncate">
          <Link href="/" className="hover:text-text">Home</Link>
          {" / "}
          <Link href={`/sandboxes/${id}`} className="hover:text-text">{id}</Link>
          {" / "}
          <span className="text-text">Terminal</span>
        </div>
        <div className="flex items-center gap-3 mt-1">
          <h1 className="text-base sm:text-lg font-semibold font-mono truncate">{id}</h1>
          {sandbox && (
            <span className={`text-xs px-2 py-0.5 rounded-md border shrink-0 ${statusBorderColor(sandbox.status)} ${statusTextColor(sandbox.status)}`}>
              {sandbox.status}
            </span>
          )}
        </div>
      </div>

      <div className="flex-1 min-h-0">
        <TerminalView sandboxId={id} />
      </div>
    </div>
  );
}
