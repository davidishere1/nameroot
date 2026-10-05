import { CONTRACT_ID } from "../names";
import { contractLink } from "../lib/stellar";
import { useEffect } from "react";
import { Link, useSection, useTitle } from "../lib/router";

const SECTIONS = [
  ["start", "Getting started"],
  ["concepts", "Concepts"],
  ["reference", "Contract reference"],
  ["faq", "FAQ"],
] as const;

export function Docs() {
  useTitle("Docs · nameroot");
  const section = useSection();
  useEffect(() => {
    if (section) document.getElementById(section)?.scrollIntoView({ behavior: "smooth" });
  }, [section]);
  return (
    <div className="mx-auto grid max-w-6xl gap-12 px-5 py-14 lg:grid-cols-[210px_1fr]">
      <aside className="hidden lg:block">
        <nav className="sticky top-24 space-y-1 text-sm">
          <p className="mb-3 px-3 text-xs font-bold uppercase tracking-[0.2em] text-violet">On this page</p>
          {SECTIONS.map(([id, label]) => (
            <Link key={id} to={`/docs/${id}`} className="block rounded-lg px-3 py-2 text-violet-deep hover:bg-white">
              {label}
            </Link>
          ))}
        </nav>
      </aside>

      <article className="min-w-0 space-y-16">
        <header>
          <p className="text-xs font-bold uppercase tracking-[0.2em] text-violet">Documentation</p>
          <h1 className="mt-3 text-4xl md:text-5xl font-extrabold tracking-tight text-ink">How nameroot works</h1>
          <p className="mt-4 max-w-2xl text-lg text-soft">A Soroban registry that maps names to Stellar addresses, with expiry, renewals and verified reverse lookups.</p>
        </header>

        <section id="start" className="scroll-mt-24 space-y-5">
          <h2 className="text-3xl font-extrabold tracking-tight text-ink">Getting started</h2>
          <ol className="space-y-3">
            {START.map((step, i) => (
              <li key={i} className="flex gap-4">
                <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full text-xs font-bold bg-violet text-white">{i + 1}</span>
                <p className="pt-0.5 text-ink/85">{step}</p>
              </li>
            ))}
          </ol>
          <Link to="/app" className="go go-v inline-block inline-block">Search names →</Link>
        </section>

        <section id="concepts" className="scroll-mt-24 space-y-5">
          <h2 className="text-3xl font-extrabold tracking-tight text-ink">Concepts</h2>
          <div className="grid gap-4 sm:grid-cols-2">
            {CONCEPTS.map(([term, body]) => (
              <div key={term} className="glass p-5">
                <h3 className="text-lg font-extrabold tracking-tight text-ink">{term}</h3>
                <p className="mt-1.5 text-sm text-soft">{body}</p>
              </div>
            ))}
          </div>
        </section>

        <section id="reference" className="scroll-mt-24 space-y-5">
          <h2 className="text-3xl font-extrabold tracking-tight text-ink">Contract reference</h2>
          <p className="text-soft">
            Deployed on testnet at{" "}
            <a className="break-all font-mono text-sm underline text-violet" href={contractLink(CONTRACT_ID)} target="_blank" rel="noreferrer">{CONTRACT_ID}</a>
          </p>
          <div className="glass overflow-x-auto">
            <table className="w-full min-w-[560px] text-left text-sm">
              <thead className="border-b border-lilac/50 text-xs uppercase tracking-wider text-soft">
                <tr>
                  <th className="p-3.5">Function</th>
                  <th className="p-3.5">Signed by</th>
                  <th className="p-3.5">What it does</th>
                </tr>
              </thead>
              <tbody>
                {REFERENCE.map(([fn, who, what]) => (
                  <tr key={fn} className="border-t border-lilac/50">
                    <td className="p-3.5 font-mono text-xs text-ink">{fn}</td>
                    <td className="p-3.5 text-soft">{who}</td>
                    <td className="p-3.5 text-soft">{what}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>

        <section id="faq" className="scroll-mt-24 space-y-3">
          <h2 className="text-3xl font-extrabold tracking-tight text-ink">FAQ</h2>
          {FAQ.map(([q, a]) => (
            <details key={q} className="glass group p-5">
              <summary className="flex cursor-pointer list-none items-center justify-between gap-4 font-semibold text-ink">
                {q}
                <span className="transition group-open:rotate-45 text-violet">+</span>
              </summary>
              <p className="mt-3 text-sm text-soft">{a}</p>
            </details>
          ))}
        </section>
      </article>
    </div>
  );
}

const START: string[] = [
  "Install the Freighter browser wallet, switch it to Testnet and fund the account with test XLM from Friendbot (lab.stellar.org/account/fund).",
  "Search for a name in the app. If it’s available, choose how many years and the target address (your wallet by default).",
  "Register and sign. The fee goes to the registry treasury.",
  "Optionally set it as your primary name so apps can show it for your address, and renew before it expires."
];

const CONCEPTS: [string, string][] = [
  [
    "Record",
    "The owner, target and expiry date of a name."
  ],
  [
    "Owner vs target",
    "The owner controls the name; the target is where it resolves. They can be different addresses."
  ],
  [
    "Expiry & grace",
    "Names last 1–10 years. For 30 days after expiry the name doesn’t resolve, and nobody else can register it yet."
  ],
  [
    "Primary name",
    "A reverse record from an address to a name, honoured only while the name still points back."
  ]
];

const REFERENCE: [string, string, string][] = [
  [
    "register(owner, name, target, years)",
    "owner",
    "Registers an available name"
  ],
  [
    "renew(payer, name, years)",
    "payer",
    "Extends the expiry"
  ],
  [
    "set_target(name, target)",
    "owner",
    "Points the name at a new address"
  ],
  [
    "transfer(name, new_owner)",
    "owner",
    "Hands the name to someone else; it then points at them"
  ],
  [
    "set_primary(address, name)",
    "address",
    "Sets the reverse record"
  ],
  [
    "resolve(name) · primary_name(address)",
    "—",
    "Forward (including pay.alice subnames) and reverse lookups"
  ],
  [
    "set_subname(name, label, target) · remove_subname(name, label)",
    "owner",
    "Manage label.name subnames; they die with the parent or on transfer"
  ],
  [
    "get_record · is_available · settings · price_for",
    "—",
    "Read state"
  ],
  [
    "clear_primary(address)",
    "address",
    "Removes your reverse record"
  ],
  [
    "set_price · set_length_pricing · set_treasury · set_admin",
    "admin",
    "Registry settings (set_admin also needs the new admin's signature)"
  ]
];

const FAQ: [string, string][] = [
  [
    "What names are allowed?",
    "3 to 32 characters: lowercase letters, digits and hyphens, not starting or ending with a hyphen."
  ],
  [
    "What happens when my name expires?",
    "It stops resolving straight away. During the 30-day grace period nobody else can register it, so renew it then."
  ],
  [
    "Can a name point at a contract?",
    "Yes. The target can be any Stellar address, including a C… contract."
  ],
  [
    "How much does a name cost?",
    "A flat yearly price set in the registry settings and shown in the app. On testnet you pay in test XLM."
  ],
  [
    "Can I give a name away?",
    "Yes. Transfer it to any address; the new owner controls it from then on, and the name starts pointing at them."
  ],
  [
    "Is it audited?",
    "Not yet. It runs on Stellar testnet and is open source; treat it as a working prototype until it has been audited."
  ]
];
