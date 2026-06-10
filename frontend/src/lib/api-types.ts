/** Types returned by `cput-api` — mirror `crates/cput-api/src/types.rs`. */

export interface HeroStats {
  tokenBalance: number;
  tokenSymbol: string;
  balanceTrendPercent: number;
  activeTasks: number;
  coreLoadPercent: number;
  totalSpentUsd: number;
}

export interface ComputeTask {
  id: string;
  name: string;
  status: "Running" | "Completed" | "Queued";
  gpuClass: string;
  startedAt: string;
}

export interface TransactionRow {
  hash: string;
  action: "Mint" | "Compute Payment";
  amount: string;
  timestamp: string;
  status: "Success" | "Pending" | "Failed";
}

export interface PipelineStatus {
  telemetry: number;
  attestations: number;
  epochReports: number;
  mintInstructions: number;
  registeredNodes: number;
  lastReconciledEpoch: number | null;
}

export interface DashboardPayload {
  heroStats: HeroStats;
  tasks: ComputeTask[];
  transactions: TransactionRow[];
  pipeline: PipelineStatus;
  source: "live" | "fallback";
}

export interface MintResponse {
  status: string;
  amount: number;
  epoch: number;
  message: string;
  dryRun: boolean;
}

export interface HealthResponse {
  status: string;
  version: string;
}
