# Score recall@k of a single vbench-genesis run (genesis_results.json) against the
# exact-L2 ground truth — used by the MARK XV P1 real-embedding recall sweep
# (docs/AUDIT--P33-RSS-QUANT-MATRIX.md §3.4).
#
# Usage:  python score_recall.py <quant> <rerank>   (run from any dir; set BENCH below
#         or pass GB_VBENCH). Reads <BENCH>/genesis_results.json + ground_truth.json.
# Prints: "<quant> <rerank> recall@<k>=<value> p50=<us> rss=<MB>"
import json, sys, os
import numpy as np
BENCH = os.environ.get("GB_VBENCH", r"C:\Users\freshair\gb_vbench")
quant = sys.argv[1] if len(sys.argv) > 1 else "?"
rerank = sys.argv[2] if len(sys.argv) > 2 else "?"
gt = np.asarray(json.load(open(os.path.join(BENCH, "ground_truth.json"))))
g = json.load(open(os.path.join(BENCH, "genesis_results.json")))
topk = g["topk"]
r = float(np.mean([
    len(set(map(int, topk[i])) & set(map(int, gt[i]))) / len(gt[i])
    for i in range(len(gt))
]))
print(f"{quant} {rerank} recall@{g['k']}={r:.4f} p50={g['q_p50_us']:.1f}us rss={g['peak_rss_mb']}MB")
