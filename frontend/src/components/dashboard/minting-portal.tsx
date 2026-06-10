"use client";

import {
  ACCENT_BUTTON,
  BENTO_CARD,
  BENTO_INNER,
  TYPO_LABEL,
} from "@/components/dashboard/bento-card";
import { submitMint } from "@/lib/api-client";
import { CreditCard, Loader2 } from "lucide-react";
import { useMemo, useState } from "react";

type NetworkTier = "standard" | "hpc" | "edge";
type PaymentMethod = "usdc" | "eth" | "bnb" | "card";

const TIERS: {
  id: NetworkTier;
  title: string;
  subtitle: string;
  multiplier: string;
  perf: string;
}[] = [
  {
    id: "standard",
    title: "Standard Compute",
    subtitle: "Tier 1",
    multiplier: "1.0× cost",
    perf: "84% avg load",
  },
  {
    id: "hpc",
    title: "Dedicated HPC",
    subtitle: "Tier 2",
    multiplier: "1.4× cost",
    perf: "H100 cluster",
  },
  {
    id: "edge",
    title: "Edge Computing",
    subtitle: "Tier 3",
    multiplier: "0.85× cost",
    perf: "Low-latency",
  },
];

const PAYMENTS: { id: PaymentMethod; label: string; rate: string }[] = [
  { id: "usdc", label: "USDC", rate: "1 USDC = 10 cTMP" },
  { id: "eth", label: "ETH", rate: "1 ETH = 18,000 cTMP" },
  { id: "bnb", label: "BNB", rate: "1 BNB = 4,200 cTMP" },
  { id: "card", label: "Card", rate: "Fiat on-ramp" },
];

const MINT_MIN = 500;
const MINT_MAX = 25000;

interface MintingPortalProps {
  defaultBalance?: number;
  onAmountChange?: (amount: number) => void;
  onMintSuccess?: () => void;
}

export function MintingPortal({
  defaultBalance = 14250,
  onAmountChange,
  onMintSuccess,
}: MintingPortalProps) {
  const [tier, setTier] = useState<NetworkTier>("standard");
  const [payment, setPayment] = useState<PaymentMethod>("usdc");
  const [amount, setAmount] = useState(5000);

  const updateAmount = (n: number) => {
    setAmount(n);
    onAmountChange?.(n);
  };
  const [isProcessing, setIsProcessing] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const tierMult = tier === "hpc" ? 1.4 : tier === "edge" ? 0.85 : 1;
  const usdcCost = useMemo(
    () => Math.round((amount / 10) * tierMult * 100) / 100,
    [amount, tierMult],
  );
  const mintSeconds = useMemo(
    () => Math.max(8, Math.round(12 + amount / 1200)),
    [amount],
  );
  const sliderPct = ((amount - MINT_MIN) / (MINT_MAX - MINT_MIN)) * 100;

  const handleExecute = async () => {
    setIsProcessing(true);
    setMessage(null);
    try {
      const resp = await submitMint(amount);
      setMessage(resp.message);
      onMintSuccess?.();
    } catch (e) {
      setMessage(e instanceof Error ? e.message : "Mint failed");
    } finally {
      setIsProcessing(false);
    }
  };

  return (
    <section className={`p-6 lg:p-8 ${BENTO_CARD}`}>
      <p className={`mb-6 ${TYPO_LABEL}`}>
        Configure your computing resource minting session
      </p>

      {/* Network tier */}
      <p className={`mb-3 ${TYPO_LABEL} text-xs`}>Select network tier</p>
      <div className="mb-8 grid grid-cols-1 gap-3 sm:grid-cols-3">
        {TIERS.map((t) => {
          const active = tier === t.id;
          return (
            <button
              key={t.id}
              type="button"
              onClick={() => setTier(t.id)}
              className={`
                rounded-lg border p-4 text-left transition-all duration-200
                ${active
                  ? "border-cyan-400/80 bg-cyan-500/5 shadow-[0_0_20px_rgba(34,211,238,0.15)]"
                  : "border-zinc-800 bg-zinc-950 hover:border-zinc-700"}
              `}
            >
              <p className="text-sm font-semibold text-white">{t.title}</p>
              <p className="text-xs text-zinc-500">{t.subtitle}</p>
              <p className="mt-3 text-[11px] text-zinc-500">{t.multiplier}</p>
              <p className="text-[11px] text-zinc-500">{t.perf}</p>
            </button>
          );
        })}
      </div>

      {/* Mint amount slider */}
      <p className={`mb-3 ${TYPO_LABEL} text-xs`}>cTMP mint amount configuration</p>
      <div className={`mb-4 p-5 ${BENTO_INNER}`}>
        <div className="mb-4 flex flex-wrap items-center gap-4">
          <div className="relative min-w-0 flex-1">
            <input
              type="range"
              min={MINT_MIN}
              max={MINT_MAX}
              step={100}
              value={amount}
              onChange={(e) => updateAmount(Number(e.target.value))}
              className="h-2 w-full cursor-pointer appearance-none rounded-lg bg-zinc-800 accent-cyan-400 [&::-webkit-slider-thumb]:h-5 [&::-webkit-slider-thumb]:w-5 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-white [&::-webkit-slider-thumb]:shadow-[0_0_12px_rgba(255,255,255,0.5)]"
              style={{
                background: `linear-gradient(to right, #22d3ee 0%, #22d3ee ${sliderPct}%, #27272a ${sliderPct}%, #27272a 100%)`,
              }}
            />
          </div>
          <input
            type="number"
            min={MINT_MIN}
            max={MINT_MAX}
            value={amount}
            onChange={(e) => {
              const n = Number(e.target.value);
              if (Number.isFinite(n)) {
                updateAmount(Math.min(MINT_MAX, Math.max(MINT_MIN, n)));
              }
            }}
            className="w-36 rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-right font-mono text-sm font-semibold tabular-nums text-white outline-none focus:border-cyan-500/50"
          />
        </div>
        <div className="mb-2 h-1.5 overflow-hidden rounded-full bg-zinc-800">
          <div
            className="h-full rounded-full bg-gradient-to-r from-cyan-600 to-cyan-400 transition-all"
            style={{ width: `${sliderPct}%` }}
          />
        </div>
        <p className="text-xs text-zinc-500">
          Estimated mint time:{" "}
          <span className="text-zinc-300">{mintSeconds} seconds</span>
        </p>
        <p className="mt-1 text-xs text-zinc-500">
          Minting{" "}
          <span className="font-medium text-white">
            {amount.toLocaleString()} cTMP
          </span>{" "}
          costs{" "}
          <span className="font-medium text-cyan-400">{usdcCost} USDC</span>
        </p>
      </div>

      {/* Payment method */}
      <p className={`mb-3 ${TYPO_LABEL} text-xs`}>Select payment method</p>
      <div className="mb-8 grid grid-cols-2 gap-3 sm:grid-cols-4">
        {PAYMENTS.map((p) => {
          const active = payment === p.id;
          return (
            <button
              key={p.id}
              type="button"
              onClick={() => setPayment(p.id)}
              className={`
                flex flex-col items-center gap-2 rounded-lg border p-4 transition-all
                ${active
                  ? "border-cyan-400/80 bg-cyan-500/5 shadow-[0_0_16px_rgba(34,211,238,0.12)]"
                  : "border-zinc-800 bg-zinc-950 hover:border-zinc-700"}
              `}
            >
              {p.id === "card" ? (
                <CreditCard className="h-6 w-6 text-zinc-400" />
              ) : (
                <span className="flex h-8 w-8 items-center justify-center rounded-full bg-zinc-800 text-xs font-bold text-white">
                  {p.label.slice(0, 1)}
                </span>
              )}
              <span className="text-xs font-semibold text-zinc-200">{p.label}</span>
              <span className="text-center text-[10px] leading-tight text-zinc-500">
                {p.rate}
              </span>
            </button>
          );
        })}
      </div>

      {message ? (
        <p className="mb-4 text-xs text-zinc-400">{message}</p>
      ) : null}

      <button
        type="button"
        onClick={handleExecute}
        disabled={isProcessing}
        className={`
          flex w-full items-center justify-center gap-2 uppercase tracking-wide
          shadow-[0_0_28px_rgba(34,211,238,0.35)] ${ACCENT_BUTTON}
        `}
      >
        {isProcessing ? (
          <>
            <Loader2 className="h-4 w-4 animate-spin" />
            Executing mint…
          </>
        ) : (
          "Execute mint & optional stake"
        )}
      </button>

      <p className="mt-3 text-center text-[10px] text-zinc-600">
        Wallet balance: {defaultBalance.toLocaleString()} cTMP · {payment.toUpperCase()} selected
      </p>
    </section>
  );
}
