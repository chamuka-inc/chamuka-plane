"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { Menu, X } from "lucide-react";

const navItems = [{ label: "Quickstart", href: "/quickstart" }];

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
  const pathname = usePathname();
  const [menuOpen, setMenuOpen] = useState(false);

  useEffect(() => {
    setMenuOpen(false);
  }, [pathname]);

  function isActive(href: string): boolean {
    return pathname === href || pathname.startsWith("/sandboxes/");
  }

  return (
    <header className="sticky top-0 z-50 border-b border-border bg-black/80 backdrop-blur-xl">
      <div className="h-16 flex items-center px-4 sm:px-6">
        <div className="flex items-center gap-3 mr-6 md:mr-8">
          <LogoMark />
          <span className="text-sm font-semibold tracking-tight">Sandbox</span>
        </div>

        <nav className="hidden md:flex gap-0.5 flex-1">
          {navItems.map((item) => (
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

        <div className="ml-auto md:hidden">
          <button
            onClick={() => setMenuOpen(!menuOpen)}
            aria-label={menuOpen ? "Close menu" : "Open menu"}
            className="p-2 text-text-2 hover:text-text transition-colors"
          >
            {menuOpen ? <X size={20} /> : <Menu size={20} />}
          </button>
        </div>
      </div>

      {menuOpen && (
        <nav className="md:hidden border-t border-border bg-bg">
          {navItems.map((item) => (
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
