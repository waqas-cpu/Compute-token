"use client";

import { BENTO_CARD } from "@/components/dashboard/bento-card";
import { MintingPortal } from "@/components/dashboard/minting-portal";
import { NavBar } from "@/components/dashboard/nav-bar";
import { StakingPanel } from "@/components/dashboard/staking-panel";
import { getDashboard } from "@/lib/api-client";
import type { DashboardPayload } from "@/lib/api-types";
import { Loader2, Wifi, WifiOff } from "lucide-react";
import { useCallback, useEffect, useState } from "react";

export function DashboardShell() {
  const [data, setData] = useState<DashboardPayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [mintAmount, setMintAmount] = useState(5000);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const payload = await getDashboard();
      setData(payload);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load dashboard");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const live = data?.source === "live";

  return (
    <div className="dashboard-grid min-h-screen">
      <NavBar />

      <main className="mx-auto max-w-[1400px] px-4 py-6 md:px-8 md:py-8">
        {/* Connection strip */}
        <div className="mb-6 flex flex-wrap items-center justify-between gap-3">
          <div
            className={`
              flex items-center gap-2 rounded-lg border px-3 py-1.5 text-xs font-medium
              ${live
                ? "border-emerald-500/30 bg-emerald-500/10 text-emerald-400"
                : "border-zinc-700 bg-zinc-900 text-zinc-400"}
            `}
          >
            {live ? <Wifi className="h-3.5 w-3.5" /> : <WifiOff className="h-3.5 w-3.5" />}
            {live ? "Pipeline live · cput-api connected" : "Demo mode · start cput-api for live data"}
          </div>
        </div>

        {loading && !data ? (
          <div className={`flex items-center justify-center gap-2 py-32 text-zinc-400 ${BENTO_CARD}`}>
            <Loader2 className="h-5 w-5 animate-spin text-cyan-400" />
            Loading minting portal…
          </div>
        ) : data ? (
          <>
            {error ? (
              <p className="mb-6 rounded-lg border border-amber-500/30 bg-amber-500/10 px-4 py-2 text-sm text-amber-200">
                {error}
              </p>
            ) : null}

            <div className="grid grid-cols-1 gap-6 lg:grid-cols-2 lg:gap-8">
              <MintingPortal
                defaultBalance={data.heroStats.tokenBalance}
                onAmountChange={setMintAmount}
                onMintSuccess={() => {
                  void refresh();
                }}
              />
              <StakingPanel data={data} mintAmount={mintAmount} />
            </div>
          </>
        ) : null}
      </main>
    </div>
  );
}
