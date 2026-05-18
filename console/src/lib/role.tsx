"use client";

import { createContext, useContext, useState, useCallback, useMemo, type ReactNode } from "react";

type Role = "developer" | "operator";

type RoleContextValue = {
  role: Role;
  isOperator: boolean;
  toggleRole: () => void;
};

const RoleContext = createContext<RoleContextValue | null>(null);

export function RoleProvider({ children }: { children: ReactNode }) {
  const [role, setRole] = useState<Role>("operator");

  const toggleRole = useCallback(() => {
    setRole((r) => (r === "operator" ? "developer" : "operator"));
  }, []);

  const value = useMemo<RoleContextValue>(
    () => ({ role, isOperator: role === "operator", toggleRole }),
    [role, toggleRole],
  );

  return (
    <RoleContext.Provider value={value}>
      {children}
    </RoleContext.Provider>
  );
}

export function useRole(): RoleContextValue {
  const ctx = useContext(RoleContext);
  if (!ctx) throw new Error("useRole must be used within RoleProvider");
  return ctx;
}
