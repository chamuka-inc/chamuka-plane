"use client";

import { useState } from "react";
import { DocsSidebar } from "@/components/docs/sidebar";
import { TableOfContents } from "@/components/docs/toc";
import { Menu, X } from "lucide-react";

export default function DocsLayout({ children }: { children: React.ReactNode }) {
  const [sidebarOpen, setSidebarOpen] = useState(false);

  return (
    <div className="flex min-h-[calc(100dvh-4rem)]">
      <button
        type="button"
        onClick={() => setSidebarOpen(!sidebarOpen)}
        className="fixed bottom-4 right-4 z-50 md:hidden p-3 rounded-full bg-bg-2 border border-border shadow-lg"
        aria-label="Toggle navigation"
      >
        {sidebarOpen ? <X size={18} /> : <Menu size={18} />}
      </button>

      <aside
        className={`fixed inset-y-16 left-0 z-40 w-[220px] border-r border-border bg-bg overflow-y-auto transition-transform md:sticky md:top-16 md:translate-x-0 md:h-[calc(100dvh-4rem)] ${
          sidebarOpen ? "translate-x-0" : "-translate-x-full"
        }`}
      >
        <DocsSidebar />
      </aside>

      {sidebarOpen && (
        <div
          className="fixed inset-0 z-30 bg-black/50 md:hidden"
          onClick={() => setSidebarOpen(false)}
          onKeyDown={() => {}}
          role="presentation"
        />
      )}

      <div className="flex-1 min-w-0 flex justify-center">
        <article data-docs-content className="w-full max-w-[720px] px-6 py-8">
          {children}
        </article>
      </div>

      <aside className="hidden lg:block w-[180px] shrink-0 sticky top-16 h-[calc(100dvh-4rem)] overflow-y-auto">
        <TableOfContents />
      </aside>
    </div>
  );
}
