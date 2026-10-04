import { client } from "./lib/stellar";

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
