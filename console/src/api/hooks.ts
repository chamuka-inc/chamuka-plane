"use client";

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { createBffClient } from "./client";
import type { CreateSandboxRequest, CreateSandboxResponse } from "./client";
import type {
  ListSandboxesResponse,
  Sandbox,
  FleetResponse,
  UsageResponse,
} from "./types";

const client = createBffClient();

export function useSandboxes(params?: { status?: string }) {
  return useQuery<ListSandboxesResponse>({
    queryKey: ["sandboxes", params?.status],
    queryFn: () => client.listSandboxes(params),
  });
}

export function useSandbox(id: string) {
  return useQuery<Sandbox>({
    queryKey: ["sandbox", id],
    queryFn: () => client.getSandbox(id),
    enabled: !!id,
  });
}

export function useFleet() {
  return useQuery<FleetResponse>({
    queryKey: ["fleet"],
    queryFn: () => client.getFleetState(),
  });
}

export function useUsage(params?: { period?: string; sandbox_id?: string }) {
  return useQuery<UsageResponse>({
    queryKey: ["usage", params?.period, params?.sandbox_id],
    queryFn: () => client.getUsage(params),
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
