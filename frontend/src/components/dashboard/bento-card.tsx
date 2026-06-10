import type { ReactNode } from "react";

/** Canvas layer — page background (zinc-950). */
export const BENTO_CANVAS = "bg-zinc-950";

/** Surface layer — bento cards sit above canvas (zinc-900). */
export const BENTO_CARD =
  "rounded-2xl border border-zinc-800 bg-zinc-900 shadow-sm shadow-black/20";

/** Inset layer — inputs, rows, nested panels (zinc-950 on zinc-900). */
export const BENTO_INNER =
  "rounded-lg border border-zinc-800 bg-zinc-950";

/** Financial ledger labels — subtle, uppercase. */
export const TYPO_LABEL =
  "text-sm font-medium uppercase tracking-wider text-zinc-400";

/** Primary metric — hero numbers. */
export const TYPO_METRIC =
  "text-4xl font-bold tracking-tight text-white tabular-nums";

/** Secondary metric — supporting figures. */
export const TYPO_METRIC_SM =
  "text-2xl font-bold tracking-tight text-white tabular-nums";

/** Muted caption under metrics. */
export const TYPO_CAPTION = "text-sm text-zinc-500";

/** Vibrant primary action — mint / submit only. */
export const ACCENT_BUTTON = `
  rounded-lg bg-cyan-400 px-4 py-3.5
  text-sm font-semibold text-zinc-950
  shadow-[0_0_20px_rgba(34,211,238,0.35)]
  transition-all duration-200
  hover:bg-cyan-300 hover:shadow-[0_0_28px_rgba(34,211,238,0.5)]
  disabled:cursor-not-allowed disabled:bg-zinc-800 disabled:text-zinc-500 disabled:shadow-none
`;

interface BentoCardProps {
  children: ReactNode;
  className?: string;
}

/** Wrapper for top-level dashboard bento cells. */
export function BentoCard({ children, className = "" }: BentoCardProps) {
  return <div className={`${BENTO_CARD} ${className}`.trim()}>{children}</div>;
}
