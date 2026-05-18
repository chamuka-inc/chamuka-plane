"use client";

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
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
      <rect width="20" height="20" rx="4" fill="#ededed" />
      <rect x="4" y="4" width="5" height="5" rx="1" fill="#000" />
      <rect x="11" y="4" width="5" height="5" rx="1" fill="#000" />
      <rect x="4" y="11" width="5" height="5" rx="1" fill="#000" />
    </svg>
  );
}

export function Topbar() {
  const pathname = usePathname();
  const { role, isOperator, toggleRole } = useRole();

  function isActive(href: string): boolean {
    if (href === "/") return pathname === "/";
    return pathname.startsWith(href);
  }

  return (
    <header className="h-16 border-b border-border flex items-center px-6 sticky top-0 z-50 bg-black/80 backdrop-blur-xl">
      <div className="flex items-center gap-3 mr-8">
        <LogoMark />
        <span className="text-sm font-semibold tracking-tight">Sandbox</span>
      </div>

      <nav className="flex gap-0.5 flex-1">
        {navItems
          .filter((item) => !item.operatorOnly || isOperator)
          .map((item) => (
            <Link
              key={item.href}
              href={item.href}
              className={`text-sm px-3 py-2 rounded-md transition-colors ${
                isActive(item.href)
                  ? "text-text"
                  : "text-text-3 hover:text-text"
              }`}
            >
              {item.label}
            </Link>
          ))}
      </nav>

      <div className="flex items-center gap-3">
        <div className="bg-bg-1 border border-border rounded-md px-3 py-1.5 text-sm text-text-3 flex items-center gap-10 cursor-pointer hover:border-border-light transition-colors">
          <span>Search...</span>
          <kbd className="text-xs text-text-4 bg-bg border border-border px-1.5 py-0.5 rounded">
            ⌘K
          </kbd>
        </div>

        <button
          onClick={toggleRole}
          className="text-xs px-2.5 py-1 rounded-full border border-border text-text-2 hover:border-border-light transition-colors"
        >
          {role === "operator" ? "Operator" : "Developer"}
        </button>
      </div>
    </header>
  );
}
