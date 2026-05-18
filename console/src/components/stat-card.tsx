type StatCardProps = {
  label: string;
  value: string;
  suffix?: string;
  sub?: string;
  subColor?: string;
};

export function StatCard({ label, value, suffix, sub, subColor }: StatCardProps) {
  return (
    <div className="px-6 py-5 border-r border-border last:border-r-0">
      <div className="text-sm text-text-3 mb-1">{label}</div>
      <div className="text-4xl font-semibold tracking-tighter leading-none">
        {value}
        {suffix && <span className="text-lg font-normal text-text-3 ml-0.5">{suffix}</span>}
      </div>
      {sub && (
        <div className={`text-sm mt-1 ${subColor ?? "text-text-4"}`}>{sub}</div>
      )}
    </div>
  );
}
