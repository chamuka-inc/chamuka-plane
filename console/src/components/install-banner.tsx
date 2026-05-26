export function InstallBanner() {
  return (
    <div className="w-full min-w-0 flex flex-col sm:flex-row sm:items-center sm:justify-center gap-3 sm:gap-4 py-4 px-4 rounded-lg border border-border bg-bg-1 font-mono text-sm">
      <span className="font-mono text-text-2">$ npm i @sandbox-platform/sdk</span>
      <span className="text-text-4 text-xs sm:text-sm text-center">or</span>
      <span className="font-mono text-text-2">$ pip install sandbox-platform</span>
    </div>
  );
}
