"use client";

import { useCallback, useState } from "react";
import { Copy, Check } from "lucide-react";
import type { SnippetLanguage } from "@/lib/use-persisted-language";

type CodeSnippetProps = {
  code: string;
  language: SnippetLanguage;
  onLanguageChange: (lang: SnippetLanguage) => void;
  showLanguageTabs?: boolean;
};

export function CodeSnippet({
  code,
  language,
  onLanguageChange,
  showLanguageTabs = true,
}: CodeSnippetProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }, [code]);

  return (
    <div className="w-full min-w-0 border border-border rounded-lg overflow-hidden bg-bg-1">
      <div className="flex items-center justify-between px-3 py-2 border-b border-border">
        {showLanguageTabs ? (
          <div className="flex gap-1">
            <button
              type="button"
              onClick={() => onLanguageChange("typescript")}
              className={`text-xs px-2 py-1 rounded-md transition-colors ${
                language === "typescript"
                  ? "bg-bg-2 text-text"
                  : "text-text-3 hover:text-text"
              }`}
            >
              TypeScript
            </button>
            <button
              type="button"
              onClick={() => onLanguageChange("python")}
              className={`text-xs px-2 py-1 rounded-md transition-colors ${
                language === "python"
                  ? "bg-bg-2 text-text"
                  : "text-text-3 hover:text-text"
              }`}
            >
              Python
            </button>
          </div>
        ) : (
          <span />
        )}
        <button
          type="button"
          onClick={handleCopy}
          className="text-text-3 hover:text-text transition-colors p-1"
          aria-label="Copy code"
        >
          {copied ? <Check size={14} className="text-status-green" /> : <Copy size={14} />}
        </button>
      </div>
      <pre className="p-3 text-xs leading-relaxed overflow-x-auto font-mono text-text-2">
        <code role="code">{code}</code>
      </pre>
    </div>
  );
}
