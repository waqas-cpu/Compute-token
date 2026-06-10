"use client";

import { ApyGauge } from "@/components/dashboard/apy-gauge";
import {
  BENTO_CARD,
  BENTO_INNER,
  TYPO_LABEL,
  TYPO_METRIC,
  TYPO_METRIC_SM,
} from "@/components/dashboard/bento-card";
import type { DashboardPayload } from "@/lib/api-types";
import { Signal } from "lucide-react";

interface StakingPanelProps {
  data: DashboardPayload;
  mintAmount?: number;
}

interface ContractRow {
  type: string;
  amount: string;
  apy: string;
  status: "active" | "staked";
  contract: string;
}

function contractRows(data: DashboardPayload): ContractRow[] {
  if (data.transactions.length > 0) {
    return data.transactions.slice(0, 4).map((tx, i) => ({
      type: tx.action === "Mint" ? "Minting" : "Compute",
      amount: tx.amount.replace(/^[+-]/, ""),
      apy: `${14 + i * 2}%`,
      status: tx.status === "Success" ? "staked" : "active",
      contract: tx.hash.slice(0, 10) + "…",
    }));
  }
  return [
    { type: "Minting", amount: "5,000 cTMP", apy: "18%", status: "active", contract: "0x4a1a…cc2" },
    { type: "Staking", amount: "2,500 cTMP", apy: "16%", status: "staked", contract: "0x7804…7939" },
    { type: "Compute", amount: "1,200 cTMP", apy: "12%", status: "active", contract: "0x9f3a…2f1a" },
  ];
}

export function StakingPanel({ data, mintAmount = 5000 }: StakingPanelProps) {
  const apy = 18;
  const totalStaked = Math.round(data.heroStats.tokenBalance * 0.65);
  const rewardsEarned = Math.round(totalStaked * 0.0135);
  const inferenceHours = Math.round(mintAmount * 3.6);
  const renderFrames = Math.round(mintAmount * 0.7);
  const rows = contractRows(data);
  const live = data.source === "live";

  return (
    <div className="flex flex-col gap-5">
      <section className={`p-6 ${BENTO_CARD}`}>
        <p className={`mb-5 ${TYPO_LABEL}`}>
          Staking pool & computing utility breakdown
        </p>

        <div className={`flex flex-col gap-5 p-5 sm:flex-row sm:items-start ${BENTO_INNER}`}>
          <div className="min-w-0 flex-1 space-y-4">
            <div>
              <p className="text-xs uppercase tracking-wider text-zinc-500">
                Rewards
              </p>
              <p className={`mt-1 ${TYPO_METRIC}`}>
                {mintAmount.toLocaleString()}
                <span className="ml-2 text-sm font-medium text-zinc-500">
                  newly minted cTMP
                </span>
              </p>
            </div>
            <div>
              <p className="text-xs uppercase tracking-wider text-zinc-500">
                Compute usage utility
              </p>
              <p className="mt-2 text-sm text-zinc-300">
                <span className="font-semibold text-white">
                  {inferenceHours.toLocaleString()}
                </span>{" "}
                ML inference hours
              </p>
              <p className="mt-1 text-sm text-zinc-400">
                or{" "}
                <span className="font-semibold text-zinc-200">
                  {renderFrames.toLocaleString()}
                </span>{" "}
                render frames
              </p>
            </div>
          </div>
          <div className="flex shrink-0 justify-center sm:justify-end">
            <ApyGauge value={apy} />
          </div>
        </div>
      </section>

      <section className={`p-6 ${BENTO_CARD}`}>
        <p className={`mb-4 ${TYPO_LABEL}`}>Active staking profile</p>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <div className={`p-5 ${BENTO_INNER}`}>
            <p className="text-xs uppercase tracking-wider text-zinc-500">
              Total staked
            </p>
            <p className={`mt-2 ${TYPO_METRIC_SM}`}>
              {totalStaked.toLocaleString()}
              <span className="ml-1 text-sm font-medium text-zinc-500">cTMP</span>
            </p>
          </div>
          <div className={`p-5 ${BENTO_INNER}`}>
            <p className="text-xs uppercase tracking-wider text-zinc-500">
              Staking rewards earned
            </p>
            <p className={`mt-2 ${TYPO_METRIC_SM}`}>
              {rewardsEarned.toLocaleString()}
              <span className="ml-1 text-sm font-medium text-zinc-500">cTMP</span>
            </p>
          </div>
        </div>
      </section>

      <section className={`overflow-hidden p-6 ${BENTO_CARD}`}>
        <p className={`mb-4 ${TYPO_LABEL}`}>
          Active minting & staking contracts
        </p>
        <div className={`overflow-x-auto ${BENTO_INNER}`}>
          <table className="w-full min-w-[480px] text-left">
            <thead>
              <tr className="border-b border-zinc-800">
                {["Type", "Amount", "APY", "Contract"].map((h) => (
                  <th
                    key={h}
                    className={`px-5 py-4 ${TYPO_LABEL} text-[10px]`}
                  >
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr
                  key={row.contract + row.type}
                  className="border-b border-zinc-800/60 last:border-0 hover:bg-zinc-900/60"
                >
                  <td className="px-5 py-4 text-sm text-zinc-300">{row.type}</td>
                  <td className="px-5 py-4 text-sm font-semibold tabular-nums text-white">
                    {row.amount}
                  </td>
                  <td className="px-5 py-4 text-sm font-medium text-cyan-400">
                    {row.apy}
                  </td>
                  <td className="px-5 py-4">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-mono text-xs text-zinc-500">
                        {row.contract}
                      </span>
                      <span
                        className={`
                          rounded-md border px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide
                          ${row.status === "active"
                            ? "border-emerald-500/40 bg-emerald-500/10 text-emerald-400"
                            : "border-amber-500/40 bg-amber-500/10 text-amber-400"}
                        `}
                      >
                        {row.status === "active" ? "Contract active" : "Staked"}
                      </span>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      <div className="flex items-center justify-end gap-3 px-1">
        <Signal className={`h-4 w-4 ${live ? "text-emerald-400" : "text-zinc-500"}`} />
        <p className="text-xs text-zinc-500">
          Network health status:{" "}
          <span className={live ? "font-medium text-emerald-400" : "text-zinc-400"}>
            {live ? "Optimal" : "Degraded — API offline"}
          </span>
        </p>
        <div className="flex gap-0.5">
          {[1, 2, 3, 4].map((i) => (
            <div
              key={i}
              className={`h-3 w-1 rounded-sm ${live || i <= 2 ? "bg-emerald-500" : "bg-zinc-700"}`}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
