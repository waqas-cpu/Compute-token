import type { ComputeTask, HeroStats, TransactionRow, WalletState } from "@/types/dashboard";

/** Default mock wallet — toggle in NavBar until wallet SDK is integrated. */
export const MOCK_WALLET_CONNECTED: WalletState = {
  connected: true,
  address: "0x7a3f8b2c9d1e4f6a8b0c2d4e6f8a0b2c4d6e8f0a2f4c",
  balanceCtmp: 14250,
};

export const MOCK_HERO_STATS: HeroStats = {
  tokenBalance: 14250,
  tokenSymbol: "cTMP",
  balanceTrendPercent: 4.2,
  activeTasks: 3,
  coreLoadPercent: 84,
  totalSpentUsd: 420.5,
};

export const MOCK_ACTIVE_TASKS: ComputeTask[] = [
  {
    id: "task-1",
    name: "LLM inference batch #8842",
    status: "Running",
    gpuClass: "H100",
    startedAt: "2m ago",
  },
  {
    id: "task-2",
    name: "ZK proof aggregation epoch 3001",
    status: "Running",
    gpuClass: "A100",
    startedAt: "8m ago",
  },
  {
    id: "task-3",
    name: "Fine-tune checkpoint export",
    status: "Queued",
    gpuClass: "H100",
    startedAt: "—",
  },
  {
    id: "task-4",
    name: "Oracle attestation verify",
    status: "Completed",
    gpuClass: "TPU v5",
    startedAt: "18m ago",
  },
];

export const MOCK_TRANSACTIONS: TransactionRow[] = [
  {
    hash: "0x9f3a2b1c8d7e6f5a4b3c2d1e0f9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
    action: "Mint",
    amount: "+2,500 cTMP",
    timestamp: "2026-06-05 14:22 UTC",
    status: "Success",
  },
  {
    hash: "0x1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b",
    action: "Compute Payment",
    amount: "-120 cTMP",
    timestamp: "2026-06-05 11:05 UTC",
    status: "Success",
  },
  {
    hash: "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
    action: "Mint",
    amount: "+1,000 cTMP",
    timestamp: "2026-06-04 09:41 UTC",
    status: "Pending",
  },
  {
    hash: "0xdeadbeefcafebabedeadbeefcafebabedeadbeefcafebabedeadbeefcafebabe",
    action: "Compute Payment",
    amount: "-45 cTMP",
    timestamp: "2026-06-03 22:18 UTC",
    status: "Success",
  },
];
