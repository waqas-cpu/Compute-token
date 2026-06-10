"use client";

import { BENTO_INNER } from "@/components/dashboard/bento-card";
import { MOCK_WALLET_CONNECTED } from "@/lib/mock-data";
import type { WalletState } from "@/types/dashboard";
import { Cloud, Wallet } from "lucide-react";
import Link from "next/link";
import { useState } from "react";

const NAV = [
  { href: "/dashboard", label: "Dashboard", active: true },
  { href: "#", label: "My Nodes" },
  { href: "#", label: "Payments" },
  { href: "#", label: "Support" },
];

function truncateAddress(addr: string): string {
  if (addr.length < 12) return addr;
  return `${addr.slice(0, 4)}...${addr.slice(-4)}`;
}

export function NavBar() {
  const [wallet, setWallet] = useState<WalletState>(MOCK_WALLET_CONNECTED);

  const handleWallet = () => {
    setWallet((prev) =>
      prev.connected
        ? { connected: false, address: "", balanceCtmp: 0 }
        : MOCK_WALLET_CONNECTED,
    );
  };

  return (
    <header className="sticky top-0 z-50 border-b border-zinc-800/80 bg-zinc-950/95 backdrop-blur-md">
      <div className="mx-auto flex max-w-[1400px] flex-wrap items-center justify-between gap-4 px-4 py-4 md:px-8">
        {/* Brand */}
        <Link href="/dashboard" className="flex items-center gap-3">
          <div className="relative flex h-10 w-10 items-center justify-center rounded-lg border border-zinc-800 bg-zinc-900">
            <Cloud className="h-5 w-5 text-cyan-400" />
            <span className="absolute -bottom-0.5 -right-0.5 flex h-4 w-4 items-center justify-center rounded-full bg-amber-500 text-[8px] font-bold text-zinc-950">
              ₿
            </span>
          </div>
          <span className="hidden text-xs font-semibold uppercase tracking-[0.2em] text-white sm:block">
            Compute Token Platform
          </span>
        </Link>

        {/* Nav links */}
        <nav className="flex flex-wrap items-center gap-1 text-sm">
          {NAV.map((item) => (
            <Link
              key={item.label}
              href={item.href}
              className={`
                rounded-lg px-3 py-2 transition-colors
                ${item.active
                  ? "text-white"
                  : "text-zinc-500 hover:text-zinc-300"}
              `}
            >
              {item.label}
            </Link>
          ))}
          <span className="mx-1 hidden h-4 w-px bg-zinc-800 sm:block" />
          <Link
            href="/dashboard"
            className="hidden rounded-lg border border-zinc-700 px-3 py-1.5 text-xs font-medium text-zinc-300 sm:inline-block"
          >
            Minting Portal
          </Link>
        </nav>

        {/* Wallet */}
        <div className="flex flex-col items-end gap-2">
          {wallet.connected ? (
            <button
              type="button"
              onClick={handleWallet}
              className={`flex items-center gap-2 px-3 py-2 text-sm ${BENTO_INNER}`}
            >
              <span className="h-2 w-2 rounded-full bg-emerald-500 shadow-[0_0_6px_rgba(16,185,129,0.8)]" />
              <span className="font-mono text-zinc-300">
                {truncateAddress(wallet.address)}
              </span>
              <span className="text-zinc-500">·</span>
              <span className="font-semibold tabular-nums text-white">
                {wallet.balanceCtmp.toLocaleString()} cTMP
              </span>
            </button>
          ) : (
            <button
              type="button"
              onClick={handleWallet}
              className="flex items-center gap-2 rounded-lg bg-white px-5 py-2.5 text-sm font-semibold text-zinc-950 transition-opacity hover:opacity-90"
            >
              <Wallet className="h-4 w-4" />
              Connect Wallet
            </button>
          )}
        </div>
      </div>
    </header>
  );
}
