import { useEffect, useState } from "react";
import { nameProblem, names, type Settings } from "../names";
import { str } from "../lib/stellar";
import { fromUnits } from "../lib/format";
import { Link, useTitle } from "../lib/router";

export function Home() {
  useTitle("nameroot · human-readable names for Stellar");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [q, setQ] = useState("alice");
  const [hit, setHit] = useState<{ name: string; to: string | null; problem: string | null } | null>(null);
  const resolve = async (raw: string) => {
    const name = raw.trim().toLowerCase().replace(/\.xlm$/, "");
    const problem = nameProblem(name);
    if (problem) return setHit({ name, to: null, problem });
    const to = await names.read<string>("resolve", [str(name)]).catch(() => null);
    setHit({ name, to, problem: null });
  };
  useEffect(() => {
    names.read<Settings>("settings").then(setSettings).catch(() => {});
    resolve("alice");
  }, []);
  const STATS: [string, string][] = [
    ["Price", settings ? `${fromUnits(settings.price_per_year)} XLM/yr` : "…"],
    ["Terms", "1–10 yrs"],
    ["Grace", "30 days"],
  ];
  return (
    <>
      <section className="mx-auto grid max-w-6xl items-center gap-12 px-5 pb-20 pt-14 md:grid-cols-[1.2fr_1fr] md:pt-20">
        <div>
          <p className="text-xs font-bold uppercase tracking-[0.2em] text-violet">Names for Stellar addresses</p>
          <h1 className="mt-4 text-5xl leading-[1.03] md:text-6xl font-extrabold tracking-tight text-ink">Pay <span className="bg-gradient-to-r from-violet to-violet-deep bg-clip-text text-transparent">alice</span>, not a 56-character key.</h1>
          <p className="mt-6 max-w-xl text-lg text-soft">nameroot is an on-chain registry of human-readable names. Register alice.xlm, point it at any address, and let wallets and apps resolve it straight from the contract.</p>
          <div className="mt-8 flex flex-wrap gap-3">
            <Link to="/app" className="go go-v inline-block">Search names →</Link>
            <Link to="/docs" className="go go-o inline-block">How it works</Link>
          </div>
          <dl className="mt-12 grid max-w-lg grid-cols-3 gap-6">
            {STATS.map(([label, value]) => (
              <div key={label}>
                <dt className="text-[11px] uppercase tracking-wider text-soft">{label}</dt>
                <dd className="mt-1 text-2xl font-extrabold tracking-tight text-ink">{value}</dd>
              </div>
            ))}
          </dl>
        </div>
        <div className="glass p-7">
          <p className="text-xs font-bold uppercase tracking-wider text-soft">Try the resolver</p>
          <form
            className="mt-3 flex items-center gap-2 rounded-2xl border border-lilac/60 bg-white p-1.5"
            onSubmit={(e) => {
              e.preventDefault();
              resolve(q);
            }}
          >
            <span className="pl-2 text-lilac">@</span>
            <input className="min-w-0 flex-1 bg-transparent py-2 outline-none" value={q} onChange={(e) => setQ(e.target.value.toLowerCase())} aria-label="Name" />
            <button className="go go-v py-2">Resolve</button>
          </form>
          {hit && (
            <div className="mt-5 rounded-2xl bg-haze p-5">
              <p className="text-lg font-bold">{hit.name}.xlm</p>
              {hit.problem ? (
                <p className="mt-1 text-sm text-bad">{hit.problem}</p>
              ) : hit.to ? (
                <p className="mt-1 break-all font-mono text-sm text-ok">→ {hit.to}</p>
              ) : (
                <p className="mt-1 text-sm text-soft">
                  Not registered or expired.{" "}
                  <Link to="/app" className="text-violet underline">
                    Claim it →
                  </Link>
                </p>
              )}
            </div>
          )}
          <p className="mt-5 text-xs text-soft">Resolution reads the registry contract directly. Expired names never resolve.</p>
        </div>
      </section>

      <section className="border-y border-lilac/40 bg-white/50">
        <div className="mx-auto max-w-6xl px-5 py-20">
          <p className="text-xs font-bold uppercase tracking-[0.2em] text-violet">How it works</p>
          <h2 className="mt-3 text-3xl md:text-4xl font-extrabold tracking-tight text-ink">Register, point, get paid</h2>
          <ol className="mt-10 grid gap-6 md:grid-cols-3">
            {STEPS.map(([title, body], i) => (
              <li key={title} className="glass p-6">
                <span className="flex h-9 w-9 items-center justify-center rounded-full text-sm font-bold bg-violet text-white">{i + 1}</span>
                <h3 className="mt-4 text-xl font-extrabold tracking-tight text-ink">{title}</h3>
                <p className="mt-2 text-sm text-soft">{body}</p>
              </li>
            ))}
          </ol>
        </div>
      </section>

      <section className="mx-auto max-w-6xl px-5 py-20">
        <p className="text-xs font-bold uppercase tracking-[0.2em] text-violet">Use cases</p>
        <h2 className="mt-3 text-3xl md:text-4xl font-extrabold tracking-tight text-ink">Names make Stellar friendlier</h2>
        <div className="mt-10 grid gap-5 sm:grid-cols-2 lg:grid-cols-4">
          {USES.map(([icon, title, body]) => (
            <div key={title} className="glass p-6">
              <span className="text-3xl">{icon}</span>
              <h3 className="mt-3 text-lg font-extrabold tracking-tight text-ink">{title}</h3>
              <p className="mt-2 text-sm text-soft">{body}</p>
            </div>
          ))}
        </div>
      </section>

      <section className="mx-auto max-w-6xl px-5">
        <p className="text-xs font-bold uppercase tracking-[0.2em] text-violet">Guarantees</p>
        <h2 className="mt-3 text-3xl md:text-4xl font-extrabold tracking-tight text-ink">Designed so names can’t betray you</h2>
        <div className="mt-10 grid gap-5 md:grid-cols-3">
          {PROMISES.map(([title, body]) => (
            <div key={title} className="rounded-2xl p-7 bg-violet-deep text-white">
              <h3 className="text-xl font-extrabold tracking-tight">{title}</h3>
              <p className="mt-2 text-sm text-lilac">{body}</p>
            </div>
          ))}
        </div>
      </section>

      <section className="mx-auto max-w-6xl px-5 pt-20">
        <div className="glass flex flex-col items-start justify-between gap-6 p-10 md:flex-row md:items-center">
          <div>
            <h2 className="text-3xl font-extrabold tracking-tight text-ink">Claim your name before someone else does.</h2>
            <p className="mt-2 text-soft">Search, register and point it at your wallet in one go.</p>
          </div>
          <Link to="/app" className="go go-v inline-block shrink-0">Search names →</Link>
        </div>
      </section>
    </>
  );
}

const STEPS: [string, string][] = [
  [
    "Search a name",
    "3–32 lowercase letters, digits or hyphens. Check availability and the yearly price."
  ],
  [
    "Register it",
    "Pay for 1–10 years. You own the name and choose which address it points to."
  ],
  [
    "Use it everywhere",
    "Apps resolve the name on-chain. Make it your primary name so apps can show it instead of your key."
  ]
];

const USES: [string, string, string][] = [
  [
    "💸",
    "Payments",
    "Send to a name and the wallet looks up the current address."
  ],
  [
    "🪪",
    "Profiles",
    "Show alice.xlm instead of GBRP…X2H in your app."
  ],
  [
    "🏢",
    "Organisations",
    "Point treasury at a multisig and rotate it without announcing a new address."
  ],
  [
    "🧊",
    "Cold ownership",
    "Own the name from cold storage while it points at a hot wallet."
  ]
];

const PROMISES: [string, string][] = [
  [
    "Owner ≠ target",
    "The key that owns a name and the address it pays are separate. Change the target any time."
  ],
  [
    "Expired never resolves",
    "A lapsed name stops resolving at once, so payments can’t quietly go to whoever registers it next."
  ],
  [
    "Verified reverse lookups",
    "A primary name is shown for your address only while the name still points back at you."
  ]
];
