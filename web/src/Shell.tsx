import { useState, type ReactNode } from "react";
import { short } from "./lib/format";
import type { Wallet } from "./Workspace";
import { CONTRACT_ID } from "./names";
import { contractLink } from "./lib/stellar";
import { Link, useTitle } from "./lib/router";

const NAV = [
  ["/", "Home"],
  ["/app", "App"],
  ["/docs", "Docs"],
] as const;

const REPO = "https://github.com/davidishere1/nameroot";

function HeaderAction({ wallet }: { wallet: Wallet }) {
  if (wallet.address)
    return <span className="glass px-4 py-2 font-mono text-xs">● {short(wallet.address, 5)}</span>;
  return (
    <button className="go go-v inline-block" onClick={wallet.connect} disabled={wallet.connecting}>
      {wallet.connecting ? "Connecting…" : "Connect wallet"}
    </button>
  );
}

export function Shell({ route, wallet, children }: { route: string; wallet: Wallet; children: ReactNode }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="flex min-h-screen flex-col">
      <header className="sticky top-0 z-30 border-b border-lilac/40 bg-haze/80 backdrop-blur">
        <div className="mx-auto flex max-w-6xl items-center justify-between gap-4 px-5 py-3.5">
          <Link to="/" className="flex items-center gap-2.5">
            <img src={`${import.meta.env.BASE_URL}favicon.svg`} alt="" className="h-8 w-8" />
            <span className="text-xl font-bold text-violet-deep">nameroot</span>
          </Link>
          <nav className="hidden items-center gap-1 md:flex">
            {NAV.map(([to, label]) => (
              <Link key={to} to={to} className={`rounded-xl px-4 py-2 text-sm font-semibold ${route === to ? "bg-violet text-white" : "text-violet-deep hover:bg-white"}`}>
                {label}
              </Link>
            ))}
          </nav>
          <div className="hidden md:block">
            <HeaderAction wallet={wallet} />
          </div>
          <button className="go go-o px-3 py-2 md:hidden" onClick={() => setOpen((v) => !v)} aria-label="Menu" aria-expanded={open}>
            {open ? "✕" : "☰"}
          </button>
        </div>
        {open && (
          <div className="space-y-1 border-t border-lilac/50 px-5 py-4 md:hidden" onClick={() => setOpen(false)}>
            {NAV.map(([to, label]) => (
              <Link key={to} to={to} className={`block rounded-xl px-4 py-2 text-sm font-semibold ${route === to ? "bg-violet text-white" : "text-violet-deep hover:bg-white"}`}>
                {label}
              </Link>
            ))}
            <div className="pt-2">
              <HeaderAction wallet={wallet} />
            </div>
          </div>
        )}
        {wallet.error && <p className="bg-bad/10 text-bad py-2 text-center text-sm">{wallet.error}</p>}
      </header>

      <main className="flex-1">{children}</main>

      <footer className="mt-20 border-t border-lilac/40 bg-white/60">
        <div className="mx-auto grid max-w-6xl gap-8 px-5 py-12 sm:grid-cols-[1.4fr_1fr_1fr]">
          <div>
            <p className="text-xl font-bold text-violet-deep">nameroot</p>
            <p className="mt-2 max-w-xs text-sm text-soft">Human-readable names for Stellar addresses, resolved on-chain.</p>
          </div>
          <div className="text-sm">
            <p className="font-semibold text-ink">Product</p>
            <ul className="mt-3 space-y-2 text-soft">
              <li><Link to="/app" className="hover:underline">App</Link></li>
              <li><Link to="/docs" className="hover:underline">Documentation</Link></li>
              <li><a href="#/docs" onClick={() => setTimeout(() => document.getElementById("faq")?.scrollIntoView(), 60)} className="hover:underline">FAQ</a></li>
            </ul>
          </div>
          <div className="text-sm">
            <p className="font-semibold text-ink">Open source</p>
            <ul className="mt-3 space-y-2 text-soft">
              <li><a href={REPO} target="_blank" rel="noreferrer" className="hover:underline">GitHub</a></li>
              <li><a href={contractLink(CONTRACT_ID)} target="_blank" rel="noreferrer" className="hover:underline">Registry on testnet</a></li>
              <li><a href={`${REPO}/blob/main/LICENSE`} target="_blank" rel="noreferrer" className="hover:underline">MIT license</a></li>
            </ul>
          </div>
        </div>
        <p className="pb-8 text-center text-xs text-soft opacity-80">Runs on Stellar testnet. Not audited; don’t use with real funds yet.</p>
      </footer>
    </div>
  );
}

export function NotFound() {
  useTitle("Not found · nameroot");
  return (
    <section className="mx-auto max-w-xl px-5 py-28 text-center">
      <p className="text-8xl font-extrabold tracking-tight text-violet">404</p>
      <p className="mt-4 text-lg text-soft">There’s nothing at this address.</p>
      <div className="mt-8 flex justify-center gap-3">
        <Link to="/" className="go go-v inline-block">Back home</Link>
        <Link to="/docs" className="go go-o inline-block">Read the docs</Link>
      </div>
    </section>
  );
}
