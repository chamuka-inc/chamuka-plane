"use client";

import { createContext, useContext, useState, useCallback, type ReactNode } from "react";

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

  return (
    <RoleContext.Provider value={{ role, isOperator: role === "operator", toggleRole }}>
      {children}
    </RoleContext.Provider>
  );
}

export function useRole(): RoleContextValue {
  const ctx = useContext(RoleContext);
  if (!ctx) throw new Error("useRole must be used within RoleProvider");
  return ctx;
}
