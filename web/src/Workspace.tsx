import { useEffect, useState } from "react";
import { StrKey } from "@stellar/stellar-sdk";
import { nameProblem, names, phaseOf, type NameRecord, type Phase, type Settings } from "./names";
import { accountLink, addr, str, txLink, u32 } from "./lib/stellar";
import { dateOf, fromUnits, short, timeLeft } from "./lib/format";
import { useWallet } from "./lib/useWallet";
import { useAction } from "./lib/useAction";

export type Wallet = ReturnType<typeof useWallet>;

export function Workspace({ wallet }: { wallet: Wallet }) {
  const [query, setQuery] = useState("");
  const [looked, setLooked] = useState<{ name: string; record: NameRecord | null } | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [myName, setMyName] = useState<string | null>(null);
  const search = useAction();

  useEffect(() => {
    names.read<Settings>("settings").then(setSettings).catch(() => {});
  }, []);
  useEffect(() => {
    if (wallet.address) names.read<string | null>("primary_name", [addr(wallet.address)]).then((n) => setMyName(n ?? null));
  }, [wallet.address]);

  const lookup = (raw: string) =>
    search.run("lookup", async () => {
      const name = raw.trim().toLowerCase().replace(/\.xlm$/, "");
      const problem = nameProblem(name);
      if (problem) throw new Error(problem);
      let record: NameRecord | null = null;
      try {
        record = await names.read<NameRecord>("get_record", [str(name)]);
      } catch {
        record = null;
      }
      setLooked({ name, record });
    });

  return (
    <div className="min-h-screen bg-[radial-gradient(900px_500px_at_15%_-10%,#d9ccff_0%,transparent_60%),radial-gradient(700px_500px_at_100%_0%,#c3e9ff_0%,transparent_55%)]">

      <main className="mx-auto max-w-5xl px-5 pb-16">
        {myName && (
          <p className="pt-6 text-center text-sm text-soft">
            Connected as <b className="text-violet-deep">{myName}.xlm</b>
          </p>
        )}
        <section className="pt-10 text-center md:pt-16">
          <h1 className="mx-auto max-w-3xl text-5xl font-extrabold leading-[1.05] tracking-tight md:text-6xl">
            Pay <span className="bg-gradient-to-r from-violet to-violet-deep bg-clip-text text-transparent">alice</span>, not{" "}
            <span className="font-mono text-3xl font-medium text-soft md:text-4xl">GBRPYH…X2H</span>
          </h1>
          <p className="mx-auto mt-4 max-w-xl text-lg text-soft">
            Human-readable names for Stellar addresses, registered on-chain and resolvable by any wallet.
          </p>
          <form
            className="glass mx-auto mt-10 flex max-w-2xl items-center gap-2 p-2"
            onSubmit={(e) => {
              e.preventDefault();
              lookup(query);
            }}
          >
            <span className="pl-3 text-xl text-lilac">@</span>
            <input
              className="flex-1 bg-transparent px-2 py-3 text-xl outline-none"
              placeholder="search a name"
              value={query}
              onChange={(e) => setQuery(e.target.value.toLowerCase())}
            />
            <button className="go go-v" disabled={!!search.busy}>
              {search.busy ? "…" : "Search"}
            </button>
          </form>
          {search.error && <p className="mt-3 text-sm text-bad">{search.error}</p>}
          {settings && (
            <p className="mt-3 text-sm text-soft">
              {fromUnits(settings.price_per_year)} XLM per year · 1–10 years · 30-day grace period after expiry
            </p>
          )}
        </section>

        {looked && (
          <section className="mx-auto mt-8 max-w-2xl">
            <NameResult key={looked.name} name={looked.name} record={looked.record} settings={settings} wallet={wallet} onChange={() => lookup(looked.name)} />
          </section>
        )}

        <section className="mt-16 grid gap-5 md:grid-cols-3">
          {[
            ["Owner ≠ target", "Own a name from your cold wallet and point it at your hot wallet. Change the target any time."],
            ["Expired names never resolve", "A lapsed name can't quietly redirect a payment to whoever registers it next."],
            ["Verified reverse lookups", "Apps can show your name instead of your key, only while it still points at you."],
          ].map(([t, d]) => (
            <div key={t} className="glass p-6">
              <h3 className="font-semibold text-violet-deep">{t}</h3>
              <p className="mt-2 text-sm text-soft">{d}</p>
            </div>
          ))}
        </section>

        <ReverseLookup />
      </main>

    </div>
  );
}

function Result({ a }: { a: ReturnType<typeof useAction> }) {
  if (a.error) return <p className="mt-3 rounded-xl bg-bad/10 px-3 py-2 text-sm text-bad">{a.error}</p>;
  if (a.notice)
    return (
      <p className="mt-3 rounded-xl bg-ok/10 px-3 py-2 text-sm text-ok">
        {a.notice.text}{" "}
        {a.notice.hash && (
          <a className="underline" href={txLink(a.notice.hash)} target="_blank" rel="noreferrer">
            tx
          </a>
        )}
      </p>
    );
  return null;
}

const BADGE: Record<Phase, [string, string]> = {
  available: ["Available", "bg-ok/12 text-ok"],
  active: ["Registered", "bg-violet/12 text-violet"],
  grace: ["Expired · grace period", "bg-wait/15 text-wait"],
};

function NameResult({
  name,
  record,
  settings,
  wallet,
  onChange,
}: {
  name: string;
  record: NameRecord | null;
  settings: Settings | null;
  wallet: Wallet;
  onChange: () => void;
}) {
  const phase = phaseOf(record);
  const [years, setYears] = useState(1);
  const [target, setTarget] = useState("");
  const [newOwner, setNewOwner] = useState("");
  const act = useAction();
  const isOwner = !!record && wallet.address === record.owner;
  const cost = settings ? settings.price_per_year * BigInt(years) : 0n;
  const call = (label: string, method: string, args: Parameters<typeof names.invoke>[2], text: string) =>
    act.run(label, async () => {
      const me = wallet.address ?? (await wallet.connect());
      if (!me) throw new Error("Connect a wallet first.");
      const r = await names.invoke(me, method, args);
      onChange();
      return r;
    }, (r) => ({ text, hash: r.hash }));

  return (
    <div className="glass p-6">
      <div className="flex items-center justify-between">
        <p className="text-3xl font-bold">
          {name}
          <span className="text-lilac">.xlm</span>
        </p>
        <span className={`rounded-full px-3 py-1 text-xs font-semibold ${BADGE[phase][1]}`}>{BADGE[phase][0]}</span>
      </div>

      {phase === "available" && (
        <div className="mt-6 space-y-3">
          <div className="flex items-center gap-3">
            <select className="fld w-36" value={years} onChange={(e) => setYears(Number(e.target.value))}>
              {Array.from({ length: 10 }, (_, i) => i + 1).map((y) => (
                <option key={y} value={y}>
                  {y} year{y > 1 ? "s" : ""}
                </option>
              ))}
            </select>
            <p className="text-soft">
              <b className="text-ink">{fromUnits(cost)} XLM</b> total
            </p>
          </div>
          <input className="fld font-mono text-xs" placeholder="Points to (defaults to your wallet)" value={target} onChange={(e) => setTarget(e.target.value.trim())} />
          <button
            className="go go-v w-full"
            disabled={!!act.busy || (!!target && !StrKey.isValidEd25519PublicKey(target) && !StrKey.isValidContract(target))}
            onClick={async () => {
              const me = wallet.address ?? (await wallet.connect());
              if (!me) return;
              call("register", "register", [addr(me), str(name), addr(target || me), u32(years)], `${name}.xlm is yours!`);
            }}
          >
            {act.busy ? "Confirm in wallet…" : `Register ${name}.xlm`}
          </button>
        </div>
      )}

      {record && phase !== "available" && (
        <>
          <dl className="mt-6 grid gap-4 sm:grid-cols-3">
            <div>
              <dt className="text-xs uppercase tracking-wider text-soft">Resolves to</dt>
              <dd className="mt-1 font-mono text-sm">
                {phase === "active" ? (
                  <a className="underline" href={accountLink(record.target)} target="_blank" rel="noreferrer">
                    {short(record.target, 6)}
                  </a>
                ) : (
                  "— (expired)"
                )}
              </dd>
            </div>
            <div>
              <dt className="text-xs uppercase tracking-wider text-soft">Owner</dt>
              <dd className="mt-1 font-mono text-sm">{short(record.owner, 6)}</dd>
            </div>
            <div>
              <dt className="text-xs uppercase tracking-wider text-soft">{phase === "active" ? "Expires" : "Expired"}</dt>
              <dd className="mt-1 text-sm">
                {dateOf(record.expires_at)} ({timeLeft(record.expires_at)})
              </dd>
            </div>
          </dl>

          <div className="mt-6 flex flex-wrap items-center gap-2 border-t border-[#ece7ff] pt-5">
            <select className="fld w-32" value={years} onChange={(e) => setYears(Number(e.target.value))}>
              {[1, 2, 3, 5].map((y) => (
                <option key={y} value={y}>
                  +{y} yr
                </option>
              ))}
            </select>
            <button className="go go-o" disabled={!!act.busy} onClick={() => call("renew", "renew", [addr(wallet.address ?? record.owner), str(name), u32(years)], "Renewed.")}>
              Renew for {fromUnits(cost)} XLM
            </button>
            {isOwner && phase === "active" && wallet.address === record.target && (
              <button className="go go-o" disabled={!!act.busy} onClick={() => call("primary", "set_primary", [addr(wallet.address!), str(name)], "Set as your primary name.")}>
                Make primary
              </button>
            )}
          </div>

          {isOwner && phase === "active" && (
            <div className="mt-5 grid gap-3 sm:grid-cols-2">
              <div className="flex gap-2">
                <input className="fld font-mono text-xs" placeholder="New target G…/C…" value={target} onChange={(e) => setTarget(e.target.value.trim())} />
                <button className="go go-o" disabled={!!act.busy || !target} onClick={() => call("target", "set_target", [str(name), addr(target)], "Target updated.")}>
                  Point
                </button>
              </div>
              <div className="flex gap-2">
                <input className="fld font-mono text-xs" placeholder="New owner G…" value={newOwner} onChange={(e) => setNewOwner(e.target.value.trim())} />
                <button className="go go-o" disabled={!!act.busy || !newOwner} onClick={() => confirm(`Transfer ${name}.xlm?`) && call("transfer", "transfer", [str(name), addr(newOwner)], "Ownership transferred.")}>
                  Transfer
                </button>
              </div>
            </div>
          )}
        </>
      )}
      <Result a={act} />
    </div>
  );
}

function ReverseLookup() {
  const [address, setAddress] = useState("");
  const [result, setResult] = useState<string | null | undefined>(undefined);
  return (
    <section className="glass mt-10 p-6">
      <h2 className="text-lg font-semibold">Reverse lookup</h2>
      <p className="text-sm text-soft">Which name does an address go by? Only answers while the name still points back at it.</p>
      <form
        className="mt-4 flex gap-2"
        onSubmit={async (e) => {
          e.preventDefault();
          if (!StrKey.isValidEd25519PublicKey(address) && !StrKey.isValidContract(address)) return setResult(null);
          setResult((await names.read<string | null>("primary_name", [addr(address)])) ?? null);
        }}
      >
        <input className="fld font-mono text-xs" placeholder="G… or C…" value={address} onChange={(e) => setAddress(e.target.value.trim())} />
        <button className="go go-d whitespace-nowrap">Look up</button>
      </form>
      {result !== undefined && (
        <p className="mt-3 text-lg">{result ? <b>{result}.xlm</b> : <span className="text-soft">No primary name set.</span>}</p>
      )}
    </section>
  );
}
