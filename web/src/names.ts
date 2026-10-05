import { nativeToScVal, scValToNative } from "@stellar/stellar-sdk";
import { client, server, str } from "./lib/stellar";

export const CONTRACT_ID = import.meta.env.VITE_CONTRACT_ID ?? "CDF3GDRQZDLDJR5HSTGADOOTJI3FNQNMSWBJLQDH5JQ5OC2K5JUE3SWG";
export const ERRORS: Record<number, string> = {
  1: "Registry already initialized.",
  2: "Registry isn't initialized.",
  3: "Names are 3–32 characters: lowercase a–z, 0–9 and inner hyphens.",
  4: "That name is taken (or in its 30-day grace period).",
  5: "That name has never been registered.",
  6: "That name has expired.",
  7: "Register for 1 to 10 years.",
  8: "Only the owner can do that.",
  9: "Invalid price.",
  10: "That name doesn't point at your address, so it can't be your primary name.",
};
export const names = client(CONTRACT_ID, ERRORS);

export interface NameRecord {
  owner: string;
  target: string;
  expires_at: bigint;
}
export interface Settings {
  admin: string;
  fee_token: string;
  price_per_year: bigint;
  treasury: string;
}

export const GRACE = 30 * 86_400;

/** Same rules as the contract's validate_name. */
export function nameProblem(n: string): string | null {
  if (n.length < 3 || n.length > 32) return "Use 3–32 characters.";
  if (!/^[a-z0-9-]+$/.test(n)) return "Only lowercase letters, digits and hyphens.";
  if (n.startsWith("-") || n.endsWith("-")) return "Can't start or end with a hyphen.";
  return null;
}

export type Phase = "active" | "grace" | "available";
export function phaseOf(r: NameRecord | null, now = Date.now() / 1000): Phase {
  if (!r) return "available";
  const exp = Number(r.expires_at);
  if (now < exp) return "active";
  if (now < exp + GRACE) return "grace";
  return "available";
}

/**
 * Names currently owned by `owner`, discovered from registration and transfer
 * events. Public RPC nodes keep about a week of events, so older names that
 * haven't changed since won't show up here (search finds them as usual).
 */
export async function ownedNames(owner: string): Promise<{ name: string; record: NameRecord }[]> {
  const sym = (s: string) => nativeToScVal(s, { type: "symbol" }).toXDR("base64");
  const filters = [
    { type: "contract" as const, contractIds: [CONTRACT_ID], topics: [[sym("name"), sym("registered"), "*"], [sym("name"), sym("updated"), "*"]] },
  ];
  const candidates = new Set<string>();
  const latest = (await server.getLatestLedger()).sequence;
  let res = await server.getEvents({ startLedger: Math.max(1, latest - 17_280 * 7 + 100), filters, limit: 200 });
  // Each request scans a slice of ledgers; follow the cursor across the window.
  for (let page = 0; page < 20; page++) {
    for (const e of res.events) {
      const value = scValToNative(e.value) as { owner?: string };
      if (value.owner === owner) candidates.add(String(scValToNative(e.topic[2])));
    }
    const next = await server.getEvents({ cursor: res.cursor, filters, limit: 200 });
    if (next.cursor === res.cursor) break;
    res = next;
  }
  const records = await Promise.all(
    [...candidates].map(async (name) => {
      try {
        return { name, record: await names.read<NameRecord>("get_record", [str(name)]) };
      } catch {
        return null;
      }
    }),
  );
  return records
    .filter((r): r is { name: string; record: NameRecord } => !!r && r.record.owner === owner && phaseOf(r.record) !== "available")
    .sort((a, b) => Number(a.record.expires_at - b.record.expires_at));
}