"use client";

import { useCallback, useState } from "react";
import Link from "next/link";
import type { Sandbox } from "@/api/types";
import { connectSnippet } from "@/lib/quickstart-snippets";
import type { SnippetLanguage } from "@/lib/use-persisted-language";
import { CodeSnippet } from "@/components/code-snippet";
import { Check, Copy } from "lucide-react";

type LiveSandboxCardProps = {
  sandbox: Sandbox;
  language: SnippetLanguage;
  onLanguageChange: (lang: SnippetLanguage) => void;
};

export function LiveSandboxCard({ sandbox, language, onLanguageChange }: LiveSandboxCardProps) {
  const [copiedId, setCopiedId] = useState(false);

  const handleCopyId = useCallback(() => {
    navigator.clipboard.writeText(sandbox.id);
    setCopiedId(true);
    setTimeout(() => setCopiedId(false), 1500);
  }, [sandbox.id]);

  const snippet = connectSnippet(sandbox.id, language);

  return (
    <div className="border border-status-green/30 rounded-lg bg-bg-1 p-4 space-y-4">
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
        <div className="flex items-center gap-3 min-w-0">
          <span className="w-2 h-2 rounded-full bg-status-green shrink-0" />
          <span className="font-mono text-sm font-medium truncate">{sandbox.id}</span>
          <span className="text-xs px-2 py-0.5 rounded-md border border-border text-text-3 font-mono shrink-0">
            {sandbox.runtime}
          </span>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <Link
            href={`/sandboxes/${sandbox.id}`}
            className="text-xs px-3 py-1.5 rounded-md border border-border text-text-2 hover:border-border-light hover:text-text transition-colors"
          >
            Open
          </Link>
          <button
            type="button"
            onClick={handleCopyId}
            className="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded-md border border-border text-text-2 hover:border-border-light hover:text-text transition-colors"
          >
            {copiedId ? <Check size={12} className="text-status-green" /> : <Copy size={12} />}
            Copy ID
          </button>
        </div>
      </div>
      <CodeSnippet
        code={snippet}
        language={language}
        onLanguageChange={onLanguageChange}
      />
    </div>
  );
}
