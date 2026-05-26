"use client";

import { useCallback, useState } from "react";
import { Copy, Check } from "lucide-react";
import { Highlight, themes } from "prism-react-renderer";
import { usePersistedLanguage } from "@/lib/use-persisted-language";

export function CodeTabs({ ts, python }: { ts: string; python: string }) {
  const [language, setLanguage] = usePersistedLanguage();
  const [copied, setCopied] = useState(false);
  const code = language === "typescript" ? ts : python;

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }, [code]);

  return (
    <div className="w-full min-w-0 border border-border rounded-lg overflow-hidden bg-bg-1 my-4">
      <div className="flex items-center justify-between px-3 py-2 border-b border-border">
        <div className="flex gap-1">
          {(["typescript", "python"] as const).map((lang) => (
            <button
              key={lang}
              type="button"
              onClick={() => setLanguage(lang)}
              className={`text-xs px-2 py-1 rounded-md transition-colors ${
                language === lang
                  ? "bg-bg-2 text-text"
                  : "text-text-3 hover:text-text"
              }`}
            >
              {lang === "typescript" ? "TypeScript" : "Python"}
            </button>
          ))}
        </div>
        <button
          type="button"
          onClick={handleCopy}
          className="text-text-3 hover:text-text transition-colors p-1"
          aria-label="Copy code"
        >
          {copied ? <Check size={14} className="text-status-green" /> : <Copy size={14} />}
        </button>
      </div>
      <Highlight theme={themes.oneDark} code={code.trim()} language={language}>
        {({ tokens, getLineProps, getTokenProps }) => (
          <pre className="p-3 text-xs leading-relaxed overflow-x-auto font-mono bg-transparent">
            <code role="code">
              {tokens.map((line, i) => (
                <div key={i} {...getLineProps({ line })}>
                  {line.map((token, j) => (
                    <span key={j} {...getTokenProps({ token })} />
                  ))}
                </div>
              ))}
            </code>
          </pre>
        )}
      </Highlight>
    </div>
  );
}
