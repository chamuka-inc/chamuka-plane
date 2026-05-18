type StatCardProps = {
  label: string;
  value: string;
  suffix?: string;
  sub?: string;
  subColor?: string;
};

export function StatCard({ label, value, suffix, sub, subColor }: StatCardProps) {
  return (
    <div className="bg-bg px-4 py-4 sm:px-6 sm:py-5">
      <div className="text-sm text-text-3 mb-1">{label}</div>
      <div className="text-2xl sm:text-4xl font-semibold tracking-tighter leading-none">
        {value}
        {suffix && <span className="text-base sm:text-lg font-normal text-text-3 ml-0.5">{suffix}</span>}
      </div>
      {sub && (
        <div className={`text-xs sm:text-sm mt-1 truncate ${subColor ?? "text-text-4"}`}>{sub}</div>
      )}
    </div>
  );
}
