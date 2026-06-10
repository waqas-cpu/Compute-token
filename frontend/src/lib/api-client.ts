import type {
  DashboardPayload,
  HealthResponse,
  MintResponse,
} from "@/lib/api-types";
import {
  MOCK_ACTIVE_TASKS,
  MOCK_HERO_STATS,
  MOCK_TRANSACTIONS,
} from "@/lib/mock-data";
import type { ComputeTask, HeroStats, TransactionRow } from "@/types/dashboard";

const API_BASE =
  process.env.NEXT_PUBLIC_CPUT_API_URL ?? "/api/backend";

async function fetchJson<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...init?.headers,
    },
    cache: "no-store",
  });
  if (!res.ok) {
    const err = await res.text();
    throw new Error(`API ${path} failed (${res.status}): ${err}`);
  }
  return res.json() as Promise<T>;
}

/** Fallback dashboard when the Rust API is unreachable. */
export function fallbackDashboard(): DashboardPayload {
  return {
    heroStats: MOCK_HERO_STATS,
    tasks: MOCK_ACTIVE_TASKS,
    transactions: MOCK_TRANSACTIONS,
    pipeline: {
      telemetry: 0,
      attestations: 0,
      epochReports: 0,
      mintInstructions: 0,
      registeredNodes: 0,
      lastReconciledEpoch: null,
    },
    source: "fallback",
  };
}

/** Load full dashboard from `GET /api/v1/dashboard`. */
export async function getDashboard(): Promise<DashboardPayload> {
  try {
    return await fetchJson<DashboardPayload>("/dashboard");
  } catch {
    return fallbackDashboard();
  }
}

/** Health check for pipeline integration tests. */
export async function getHealth(): Promise<HealthResponse> {
  return fetchJson<HealthResponse>("/health");
}

/** Submit mint intent to `POST /api/v1/mint`. */
export async function submitMint(amount: number): Promise<MintResponse> {
  return fetchJson<MintResponse>("/mint", {
    method: "POST",
    body: JSON.stringify({ amount }),
  });
}

export type { HeroStats, ComputeTask, TransactionRow, DashboardPayload };
