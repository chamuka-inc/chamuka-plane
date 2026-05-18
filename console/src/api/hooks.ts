"use client";

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { createBffClient } from "./client";
import type { CreateSandboxRequest, CreateSandboxResponse } from "./client";
import type {
  ListSandboxesResponse,
  Sandbox,
  FleetResponse,
  UsageResponse,
  UsagePeriod,
} from "./types";

const client = createBffClient();

export function useSandboxes(params?: { status?: string }) {
  return useQuery<ListSandboxesResponse>({
    queryKey: ["sandboxes", params?.status],
    queryFn: () => client.listSandboxes(params),
    refetchInterval: 10_000,
  });
}

export function useSandbox(id: string) {
  return useQuery<Sandbox>({
    queryKey: ["sandbox", id],
    queryFn: () => client.getSandbox(id),
    enabled: !!id,
    refetchInterval: 10_000,
  });
}

export function useFleet() {
  return useQuery<FleetResponse>({
    queryKey: ["fleet"],
    queryFn: () => client.getFleetState(),
    refetchInterval: 15_000,
  });
}

export function useUsage(params?: { period?: UsagePeriod; sandbox_id?: string }) {
  return useQuery<UsageResponse>({
    queryKey: ["usage", params?.period, params?.sandbox_id],
    queryFn: () => client.getUsage(params),
    refetchInterval: 60_000,
  });
}

export function useCreateSandbox() {
  const queryClient = useQueryClient();
  return useMutation<CreateSandboxResponse, Error, CreateSandboxRequest>({
    mutationFn: (body) => client.createSandbox(body),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["sandboxes"] });
    },
  });
}
