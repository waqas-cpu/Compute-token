/** Mock wallet state — replace with Sui wallet adapter / zkLogin later. */
export interface WalletState {
  connected: boolean;
  address: string;
  balanceCtmp: number;
}

/** Hero stat cards at the top of the dashboard. */
export interface HeroStats {
  tokenBalance: number;
  tokenSymbol: string;
  balanceTrendPercent: number;
  activeTasks: number;
  coreLoadPercent: number;
  totalSpentUsd: number;
}

/** Single compute task in the live status panel. */
export interface ComputeTask {
  id: string;
  name: string;
  status: "Running" | "Completed" | "Queued";
  gpuClass: string;
  startedAt: string;
}

/** Row in the transaction history table. */
export interface TransactionRow {
  hash: string;
  action: "Mint" | "Compute Payment";
  amount: string;
  timestamp: string;
  status: "Success" | "Pending" | "Failed";
}

/** Minting form state — wire to relayer / on-chain mint API. */
export interface MintFormState {
  amount: string;
  usdPerToken: number;
  isProcessing: boolean;
}
