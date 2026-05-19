"use client";

import Link from "next/link";

function LogoMark() {
  return (
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none" aria-hidden="true">
      <rect width="20" height="20" rx="4" fill="#ededed" />
      <rect x="4" y="4" width="5" height="5" rx="1" fill="#000" />
      <rect x="11" y="4" width="5" height="5" rx="1" fill="#000" />
      <rect x="4" y="11" width="5" height="5" rx="1" fill="#000" />
    </svg>
  );
}

export function Topbar() {
  return (
    <header className="sticky top-0 z-50 border-b border-border bg-black/80 backdrop-blur-xl">
      <div className="h-16 flex items-center gap-6 px-4 sm:px-6">
        <Link
          href="/"
          className="flex items-center gap-3 hover:opacity-80 transition-opacity"
        >
          <LogoMark />
          <span className="text-sm font-semibold tracking-tight">Sandbox</span>
        </Link>
        <Link
          href="/docs/quickstart"
          className="text-sm text-text-3 hover:text-text transition-colors"
        >
          Docs
        </Link>
      </div>
    </header>
  );
}
