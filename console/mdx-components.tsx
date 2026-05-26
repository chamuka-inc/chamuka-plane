import type { MDXComponents } from "mdx/types";
import { Callout } from "@/components/docs/callout";
import { CodeTabs } from "@/components/docs/code-tabs";
import { MethodCard } from "@/components/docs/method-card";
import { ParamTable } from "@/components/docs/param-table";

export function useMDXComponents(components: MDXComponents): MDXComponents {
  return {
    h1: (props) => (
      <h1
        className="text-2xl font-semibold tracking-tight text-text pb-4 mb-6 border-b border-border"
        {...props}
      />
    ),
    h2: (props) => (
      <h2
        className="text-xl font-semibold text-text mt-12 mb-4 scroll-mt-20"
        {...props}
      />
    ),
    h3: (props) => (
      <h3
        className="text-base font-semibold text-text mt-8 mb-3 scroll-mt-20"
        {...props}
      />
    ),
    h4: (props) => (
      <h4
        className="text-sm font-semibold text-text mt-6 mb-2 scroll-mt-20"
        {...props}
      />
    ),
    p: (props) => (
      <p className="text-sm text-text-2 leading-relaxed mb-4" {...props} />
    ),
    a: (props) => (
      <a className="text-accent-blue hover:underline" {...props} />
    ),
    code: (props) => {
      const isBlock = typeof props.children === "string" && props.children.includes("\n");
      if (isBlock) return <code {...props} />;
      return (
        <code
          className="text-xs bg-bg-2 px-1.5 py-0.5 rounded text-accent-cyan font-mono"
          {...props}
        />
      );
    },
    pre: (props) => (
      <pre
        className="p-4 text-xs leading-relaxed overflow-x-auto font-mono bg-bg-1 border border-border rounded-lg my-4"
        {...props}
      />
    ),
    table: (props) => (
      <div className="overflow-x-auto my-4">
        <table className="w-full text-sm" {...props} />
      </div>
    ),
    thead: (props) => <thead {...props} />,
    th: (props) => (
      <th
        className="py-2 pr-4 text-left font-medium text-text-3 border-b border-border"
        {...props}
      />
    ),
    td: (props) => (
      <td
        className="py-2 pr-4 text-text-2 border-b border-border/50"
        {...props}
      />
    ),
    blockquote: (props) => (
      <blockquote
        className="border-l-2 border-accent-purple pl-4 my-4 italic text-sm text-text-3"
        {...props}
      />
    ),
    ul: (props) => (
      <ul className="list-disc pl-5 text-sm text-text-2 space-y-1 mb-4" {...props} />
    ),
    ol: (props) => (
      <ol className="list-decimal pl-5 text-sm text-text-2 space-y-1 mb-4" {...props} />
    ),
    li: (props) => <li className="leading-relaxed" {...props} />,
    hr: () => <hr className="border-border my-8" />,
    Callout,
    CodeTabs,
    MethodCard,
    ParamTable,
    ...components,
  };
}
