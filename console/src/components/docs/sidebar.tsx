"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { docsNavigation } from "@/lib/docs-navigation";

export function DocsSidebar({ pathname: pathnameProp }: { pathname?: string }) {
  const resolved = usePathname();
  const pathname = pathnameProp ?? resolved;

  return (
    <nav className="py-6 pr-4 space-y-6">
      {docsNavigation.map((section) => (
        <div key={section.title}>
          <h4 className="text-xs font-medium text-text-3 uppercase tracking-wider mb-2 px-3">
            {section.title}
          </h4>
          <ul className="space-y-0.5">
            {section.items.map((item) => {
              const active = pathname === item.href;
              return (
                <li key={item.href}>
                  <Link
                    href={item.href}
                    className={`block text-sm px-3 py-1.5 rounded-md border-l-2 transition-colors ${
                      active
                        ? "border-accent-blue text-text bg-bg-hover"
                        : "border-transparent text-text-3 hover:text-text hover:bg-bg-hover"
                    }`}
                  >
                    {item.title}
                  </Link>
                </li>
              );
            })}
          </ul>
        </div>
      ))}
    </nav>
  );
}
