import type { ReactNode } from "react";

type MethodCardProps = {
  name: string;
  signature: string;
  description: string;
  children?: ReactNode;
};

function slugify(name: string): string {
  return name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/(^-|-$)/g, "");
}

export function MethodCard({ name, signature, description, children }: MethodCardProps) {
  const id = slugify(name);
  return (
    <div className="border border-border rounded-lg p-4 my-6">
      <h3 id={id} className="text-base font-semibold text-text mb-1">
        <a href={`#${id}`} className="hover:text-accent-blue transition-colors">
          {name}
        </a>
      </h3>
      <p className="font-mono text-xs text-text-3 mb-3 break-all">{signature}</p>
      <p className="text-sm text-text-2 mb-4">{description}</p>
      {children}
    </div>
  );
}
