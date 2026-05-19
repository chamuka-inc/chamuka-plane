"use client";

import { useCallback, useEffect, useState } from "react";

export type SnippetLanguage = "typescript" | "python";

const STORAGE_KEY = "console-snippet-lang";

export function usePersistedLanguage(): [SnippetLanguage, (lang: SnippetLanguage) => void] {
  const [language, setLanguageState] = useState<SnippetLanguage>("typescript");

  useEffect(() => {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "typescript" || stored === "python") {
      setLanguageState(stored);
    }
  }, []);

  const setLanguage = useCallback((lang: SnippetLanguage) => {
    setLanguageState(lang);
    localStorage.setItem(STORAGE_KEY, lang);
  }, []);

  return [language, setLanguage];
}
