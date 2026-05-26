export type NavItem = {
  title: string;
  href: string;
};

export type NavSection = {
  title: string;
  items: NavItem[];
};

export const docsNavigation: NavSection[] = [
  {
    title: "Getting Started",
    items: [{ title: "Quickstart", href: "/docs/quickstart" }],
  },
  {
    title: "Reference",
    items: [{ title: "SDK Reference", href: "/docs/sdk-reference" }],
  },
  {
    title: "Guides",
    items: [
      { title: "Runtimes & Trust", href: "/docs/guides/runtimes" },
      { title: "Environment", href: "/docs/guides/environment" },
      { title: "Credentials", href: "/docs/guides/credentials" },
      { title: "Lifecycle", href: "/docs/guides/lifecycle" },
    ],
  },
];
