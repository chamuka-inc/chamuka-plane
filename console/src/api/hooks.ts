"use client";

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { createBffClient } from "./client";
import type { CreateSandboxRequest, CreateSandboxResponse } from "./client";
import type { ListSandboxesResponse, Sandbox } from "./types";

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

export function useCreateSandbox() {
  const queryClient = useQueryClient();
  return useMutation<CreateSandboxResponse, Error, CreateSandboxRequest>({
    mutationFn: (body) => client.createSandbox(body),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["sandboxes"] });
    },
  });
}

export function useTerminateSandbox() {
  const queryClient = useQueryClient();
  return useMutation<void, Error, { id: string; force?: boolean }>({
    mutationFn: ({ id, force }) => client.terminateSandbox(id, force),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["sandboxes"] });
    },
  });
}
