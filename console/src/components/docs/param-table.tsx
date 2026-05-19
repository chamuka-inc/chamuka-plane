type Param = {
  name: string;
  type: string;
  required: boolean;
  description: string;
};

export function ParamTable({ params }: { params: Param[] }) {
  return (
    <div className="overflow-x-auto my-4">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-border text-left text-text-3">
            <th className="py-2 pr-4 font-medium">Name</th>
            <th className="py-2 pr-4 font-medium">Type</th>
            <th className="py-2 pr-4 font-medium">Description</th>
          </tr>
        </thead>
        <tbody>
          {params.map((p) => (
            <tr key={p.name} className="border-b border-border/50">
              <td className="py-2 pr-4 font-mono text-xs text-text">
                {p.name}
                {p.required && (
                  <span className="ml-1.5 text-[10px] text-status-amber font-sans">required</span>
                )}
              </td>
              <td className="py-2 pr-4">
                <code className="text-xs text-accent-cyan">{p.type}</code>
              </td>
              <td className="py-2 text-text-2">{p.description}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
