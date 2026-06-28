import { useEffect, useRef, useState } from "react";
import { retrieveContext, type ContextPackage } from "../lib/api";

interface RetrieverPanelProps {
  onResult: (pkg: ContextPackage) => void;
  selectedNodeId: string | null;
}

// GRL tiers. H0 = exact match, H5 = broadest context.
const TIERS = ["H0", "H1", "H2", "H3", "H4", "H5"] as const;
type Tier = (typeof TIERS)[number];

const TIER_BADGE: Record<Tier, string> = {
  H0: "#e3b341",
  H1: "#388bfd",
  H2: "#3fb950",
  H3: "#a371f7",
  H4: "#db6d28",
  H5: "#8b949e",
};

export default function RetrieverPanel({
  onResult,
  selectedNodeId,
}: RetrieverPanelProps) {
  const [queryText, setQueryText] = useState("");
  const [tier, setTier] = useState<Tier>("H1");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pkg, setPkg] = useState<ContextPackage | null>(null);

  // Avoid stale closures inside the selectedNodeId effect.
  const tierRef = useRef(tier);
  tierRef.current = tier;

  async function runRetrieve(targetId: string, withTier: Tier) {
    const target = targetId.trim();
    if (!target) return;
    setLoading(true);
    setError(null);
    try {
      const result = await retrieveContext(target, withTier, null, true);
      setPkg(result);
      onResult(result);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }

  // When a graph node is tapped, retrieve its context (Obsidian expand loop).
  useEffect(() => {
    if (selectedNodeId) {
      setQueryText(selectedNodeId);
      void runRetrieve(selectedNodeId, tierRef.current);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedNodeId]);

  function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    void runRetrieve(queryText, tier);
  }

  return (
    <div className="retriever-panel">
      <form className="retriever-form" onSubmit={onSubmit}>
        <input
          className="retriever-input"
          type="text"
          inputMode="search"
          placeholder="Node id or query…"
          value={queryText}
          onChange={(e) => setQueryText(e.target.value)}
        />
        <select
          className="retriever-tier"
          value={tier}
          onChange={(e) => setTier(e.target.value as Tier)}
          aria-label="Retrieval tier"
        >
          {TIERS.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
        <button className="retriever-submit" type="submit" disabled={loading}>
          {loading ? "…" : "Retrieve"}
        </button>
      </form>

      {error && <div className="retriever-error">⚠ {error}</div>}

      {pkg && !error && (
        <div className="tier-card">
          <div className="tier-card-head">
            <span
              className="tier-badge"
              style={{ background: TIER_BADGE[tier] }}
            >
              {tier}
            </span>
            <span className="tier-stats">
              {pkg.nodes.length} nodes · {pkg.edges.length} edges ·{" "}
              {pkg.token_estimate} tok
            </span>
          </div>
          {pkg.reasoning_path && (
            <div className="tier-reasoning">{pkg.reasoning_path}</div>
          )}
          <ul className="tier-node-list">
            {pkg.nodes.slice(0, 50).map((n) => (
              <li key={n.id} className="tier-node">
                <span className="tier-node-id">{n.id}</span>
                {n.labels.length > 0 && (
                  <span className="tier-node-labels">
                    {n.labels.join(" · ")}
                  </span>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
