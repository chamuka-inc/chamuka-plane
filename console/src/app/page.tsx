"use client";

import { useRouter } from "next/navigation";
import { useSandboxes } from "@/api/hooks";
import { BootSandboxPanel } from "@/components/boot-sandbox-panel";
import { CodeSnippet } from "@/components/code-snippet";
import { InstallBanner } from "@/components/install-banner";
import { LiveSandboxCard } from "@/components/live-sandbox-card";
import { QuickstartStep } from "@/components/quickstart-step";
import { RuntimeSwitcher } from "@/components/runtime-switcher";
import { getStepSnippet, type QuickstartEnv } from "@/lib/quickstart-snippets";
import { usePersistedLanguage } from "@/lib/use-persisted-language";
import { filterReadySandboxes } from "@/lib/sandbox-filters";
import { useState, useCallback } from "react";
import { Play } from "lucide-react";

export default function QuickstartPage() {
  const router = useRouter();
  const [env, setEnv] = useState<QuickstartEnv>("cli");
  const [language, setLanguage] = usePersistedLanguage();
  const { data, isLoading } = useSandboxes({ status: "ready" });

  const readySandboxes = filterReadySandboxes(data?.items ?? []);

  const handleBooted = useCallback(
    (sandbox: { id: string }) => {
      router.push(`/sandboxes/${sandbox.id}`);
    },
    [router],
  );

  return (
    <div className="max-w-3xl mx-auto px-4 sm:px-6 py-8">
      <header className="text-center mb-10">
        <h1 className="text-2xl sm:text-3xl font-semibold tracking-tight mb-2">
          Run code in secure sandboxes
        </h1>
        <p className="text-text-3 text-sm max-w-lg mx-auto">
          Create, connect, and execute — from your app, in three lines.
        </p>
      </header>

      <div className="flex gap-4 mb-10">
        <div className="w-8 shrink-0" aria-hidden="true" />
        <div className="flex-1 min-w-0 space-y-8">
          <InstallBanner />
          <RuntimeSwitcher selected={env} onChange={setEnv} />
        </div>
      </div>

      <div className="space-y-0">
        <QuickstartStep number={1} title="Create a sandbox">
          <CodeSnippet
            code={getStepSnippet(1, env, language)}
            language={language}
            onLanguageChange={setLanguage}
          />
        </QuickstartStep>

        <QuickstartStep
          number={2}
          title={env === "desktop" ? "Capture the screen" : "Run code"}
        >
          <CodeSnippet
            code={getStepSnippet(2, env, language)}
            language={language}
            onLanguageChange={setLanguage}
          />
        </QuickstartStep>

        <QuickstartStep
          number={3}
          title={env === "desktop" ? "Automate the desktop" : "Interact with the sandbox"}
        >
          <CodeSnippet
            code={getStepSnippet(3, env, language)}
            language={language}
            onLanguageChange={setLanguage}
          />
        </QuickstartStep>

        <QuickstartStep
          number={4}
          title="Connect to a running sandbox"
          description="Attach to a sandbox that is already running in your fleet."
        >
          {isLoading && (
            <p className="text-sm text-text-3">Loading sandboxes…</p>
          )}
          {!isLoading && readySandboxes.length === 0 && (
            <p className="text-sm text-text-3">
              No sandboxes running.{" "}
              <a href="#try-live" className="text-accent-blue hover:underline">
                Boot one below
              </a>
            </p>
          )}
          {!isLoading && readySandboxes.length > 0 && (
            <div className="space-y-4">
              {readySandboxes.map((sandbox) => (
                <LiveSandboxCard
                  key={sandbox.id}
                  sandbox={sandbox}
                  language={language}
                  onLanguageChange={setLanguage}
                />
              ))}
            </div>
          )}
        </QuickstartStep>

        <QuickstartStep
          id="try-live"
          marker={<Play size={14} aria-hidden />}
          title="Try it live"
          description="Boot a sandbox and open the terminal — no SDK required."
          isLast
        >
          <BootSandboxPanel onBooted={handleBooted} />
        </QuickstartStep>
      </div>
    </div>
  );
}
