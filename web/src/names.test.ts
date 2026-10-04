import { describe, expect, it } from "vitest";
import { nameProblem, phaseOf } from "./names";

describe("nameProblem mirrors the contract", () => {
  it.each(["abc", "alice-01", "7eleven", "z".repeat(32)])("accepts %s", (n) => expect(nameProblem(n)).toBeNull());
  it.each(["ab", "Alice", "al ice", "-alice", "alice-", "alice_1", "ålice", "a".repeat(33)])("rejects %s", (n) =>
    expect(nameProblem(n)).not.toBeNull(),
  );
});

describe("phaseOf", () => {
  const r = { owner: "G", target: "G", expires_at: 1_000n };
  it("tracks active, grace and available", () => {
    expect(phaseOf(r, 999)).toBe("active");
    expect(phaseOf(r, 1_000)).toBe("grace");
    expect(phaseOf(r, 1_000 + 30 * 86_400)).toBe("available");
    expect(phaseOf(null)).toBe("available");
  });
});
