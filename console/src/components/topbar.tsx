"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useRole } from "@/lib/role";

const navItems = [
  { label: "Overview", href: "/", operatorOnly: false },
  { label: "Sandboxes", href: "/sandboxes", operatorOnly: false },
  { label: "Fleet", href: "/fleet", operatorOnly: true },
  { label: "Analytics", href: "/analytics", operatorOnly: false },
];

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

function MenuIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
      <line x1="3" y1="5" x2="17" y2="5" />
      <line x1="3" y1="10" x2="17" y2="10" />
      <line x1="3" y1="15" x2="17" y2="15" />
    </svg>
  );
}

function CloseIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
      <line x1="4" y1="4" x2="16" y2="16" />
      <line x1="16" y1="4" x2="4" y2="16" />
    </svg>
  );
}

export function Topbar() {
  const pathname = usePathname();
  const { role, isOperator, toggleRole } = useRole();
  const [menuOpen, setMenuOpen] = useState(false);

  useEffect(() => {
    setMenuOpen(false);
  }, [pathname]);

  function isActive(href: string): boolean {
    if (href === "/") return pathname === "/";
    return pathname.startsWith(href);
  }

  const visibleItems = navItems.filter((item) => !item.operatorOnly || isOperator);

  return (
    <header className="sticky top-0 z-50 border-b border-border bg-black/80 backdrop-blur-xl">
      <div className="h-16 flex items-center px-4 sm:px-6">
        <div className="flex items-center gap-3 mr-6 md:mr-8">
          <LogoMark />
          <span className="text-sm font-semibold tracking-tight">Sandbox</span>
        </div>

        <nav className="hidden md:flex gap-0.5 flex-1">
          {visibleItems.map((item) => (
            <Link
              key={item.href}
              href={item.href}
              aria-current={isActive(item.href) ? "page" : undefined}
              className={`text-sm px-3 py-2 rounded-md transition-colors ${
                isActive(item.href) ? "text-text" : "text-text-3 hover:text-text"
              }`}
            >
              {item.label}
            </Link>
          ))}
        </nav>

        <div className="hidden md:flex items-center gap-3">
          <button
            type="button"
            aria-label="Search"
            className="bg-bg-1 border border-border rounded-md px-3 py-1.5 text-sm text-text-3 flex items-center gap-10 cursor-pointer hover:border-border-light transition-colors"
          >
            <span>Search...</span>
            <kbd className="text-xs text-text-4 bg-bg border border-border px-1.5 py-0.5 rounded-sm">
              ⌘K
            </kbd>
          </button>

          <button
            onClick={toggleRole}
            className="text-xs px-2.5 py-1 rounded-full border border-border text-text-2 hover:border-border-light transition-colors"
          >
            {role === "operator" ? "Operator" : "Developer"}
          </button>
        </div>

        <div className="flex items-center gap-2 ml-auto md:hidden">
          <button
            onClick={toggleRole}
            className="text-xs px-2.5 py-1 rounded-full border border-border text-text-2 hover:border-border-light transition-colors"
          >
            {role === "operator" ? "Op" : "Dev"}
          </button>
          <button
            onClick={() => setMenuOpen(!menuOpen)}
            aria-label={menuOpen ? "Close menu" : "Open menu"}
            className="p-2 text-text-2 hover:text-text transition-colors"
          >
            {menuOpen ? <CloseIcon /> : <MenuIcon />}
          </button>
        </div>
      </div>

      {menuOpen && (
        <nav className="md:hidden border-t border-border bg-bg animate-slide-down">
          {visibleItems.map((item) => (
            <Link
              key={item.href}
              href={item.href}
              onClick={() => setMenuOpen(false)}
              aria-current={isActive(item.href) ? "page" : undefined}
              className={`block px-6 py-3 text-sm transition-colors ${
                isActive(item.href)
                  ? "text-text bg-bg-hover"
                  : "text-text-3 hover:text-text hover:bg-bg-hover"
              }`}
            >
              {item.label}
            </Link>
          ))}
        </nav>
      )}
    </header>
  );
}
