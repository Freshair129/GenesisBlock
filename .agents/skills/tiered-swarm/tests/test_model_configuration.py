from contextlib import redirect_stderr, redirect_stdout
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch


REPO_ROOT = Path(__file__).resolve().parents[4]
SKILL_ROOTS = {
    "agents": REPO_ROOT / ".agents" / "skills" / "tiered-swarm",
    "claude": REPO_ROOT / ".claude" / "skills" / "tiered-swarm",
}


def load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


MODULES = {
    variant: {
        "route": load_module(f"route_{variant}", root / "scripts" / "route.py"),
        "cost_estimate": load_module(
            f"cost_estimate_{variant}", root / "scripts" / "cost_estimate.py"
        ),
    }
    for variant, root in SKILL_ROOTS.items()
}


class ModelConfigurationTests(unittest.TestCase):
    def test_t2_and_t3_use_configured_ids_in_both_skill_copies(self):
        configured = {
            "TIERED_SWARM_T2_MODEL_ID": "test-t2-model-id",
            "TIERED_SWARM_T3_MODEL_ID": "test-t3-model-id",
        }
        for variant, modules in MODULES.items():
            with self.subTest(variant=variant), patch.dict(os.environ, configured, clear=True):
                route = modules["route"].route_task
                t2 = route({"id": "needs-t2", "description": "routine task"})
                t3 = route({"id": "needs-t3", "description": "author RCA"})
                self.assertEqual(t2["computed_tier"], "T2")
                self.assertEqual(t2["executor_model"], configured["TIERED_SWARM_T2_MODEL_ID"])
                self.assertEqual(t3["computed_tier"], "T3")
                self.assertEqual(t3["executor_model"], configured["TIERED_SWARM_T3_MODEL_ID"])

    def test_router_cli_emits_configured_ids(self):
        configured = {
            "TIERED_SWARM_T2_MODEL_ID": "test-t2-model-id",
            "TIERED_SWARM_T3_MODEL_ID": "test-t3-model-id",
        }
        spec = json.dumps(
            {
                "tasks": [
                    {"id": "needs-t2", "description": "routine task"},
                    {"id": "needs-t3", "description": "author RCA"},
                ]
            }
        )
        for variant, modules in MODULES.items():
            route = modules["route"]
            with self.subTest(variant=variant), patch.dict(os.environ, configured, clear=True):
                with patch.object(route.sys, "stdin", io.StringIO(spec)), redirect_stderr(
                    io.StringIO()
                ), redirect_stdout(io.StringIO()) as output:
                    route.main(["-", "--json"])
                rows = {row["id"]: row for row in json.loads(output.getvalue())}
                self.assertEqual(
                    rows["needs-t2"]["executor_model"],
                    configured["TIERED_SWARM_T2_MODEL_ID"],
                )
                self.assertEqual(
                    rows["needs-t3"]["executor_model"],
                    configured["TIERED_SWARM_T3_MODEL_ID"],
                )

    def test_t2_or_t3_without_configured_id_fails_closed(self):
        for variant, modules in MODULES.items():
            with self.subTest(variant=variant), patch.dict(os.environ, {}, clear=True):
                with self.assertRaisesRegex(ValueError, "TIERED_SWARM_T2_MODEL_ID"):
                    modules["route"].route_task(
                        {"id": "needs-t2", "description": "routine task"}
                    )

            with self.subTest(variant=variant, tier="T3"), patch.dict(
                os.environ,
                {"TIERED_SWARM_T2_MODEL_ID": "test-t2-model-id"},
                clear=True,
            ):
                with self.assertRaisesRegex(ValueError, "TIERED_SWARM_T3_MODEL_ID"):
                    modules["route"].route_task(
                        {"id": "needs-t3", "description": "author RCA"}
                    )

    def test_router_cli_exits_two_when_a_required_tier_is_unconfigured(self):
        for variant, modules in MODULES.items():
            route = modules["route"]
            with self.subTest(variant=variant), patch.dict(os.environ, {}, clear=True):
                with patch.object(route.sys, "stdin", io.StringIO(
                    '{"tasks":[{"id":"needs-t2","description":"routine task"}]}'
                )), redirect_stderr(io.StringIO()):
                    with self.assertRaises(SystemExit) as raised:
                        route.main(["-", "--json"])
                self.assertEqual(raised.exception.code, 2)

    def test_cost_ledger_cli_exits_two_for_an_unknown_model_id(self):
        for variant, root in SKILL_ROOTS.items():
            ledger = root / "scripts" / "cost_ledger.py"
            row = json.dumps({"model": "test-unlisted-billed-id", "in_uncached": 100})
            with self.subTest(variant=variant):
                result = subprocess.run(
                    [sys.executable, str(ledger), "-"],
                    input=row + "\n",
                    capture_output=True,
                    check=False,
                    text=True,
                )
                self.assertEqual(result.returncode, 2)
                self.assertIn("unknown model ID", result.stderr)

    def test_local_routing_needs_no_billed_model_configuration(self):
        for variant, modules in MODULES.items():
            with self.subTest(variant=variant), patch.dict(os.environ, {}, clear=True):
                result = modules["route"].route_task(
                    {"id": "local-rust", "description": "Rust task", "verify_command": "cargo test"}
                )
                self.assertEqual(result["computed_tier"], "T1")
                self.assertEqual(result["executor_model"], "aroow-rust-coder-9b")

    def test_unknown_model_ids_never_receive_fallback_prices(self):
        for variant, modules in MODULES.items():
            estimator = modules["cost_estimate"]
            with self.subTest(variant=variant):
                for model in ("test-unlisted-billed-id", "test-unlisted-cloud:cloud"):
                    with self.subTest(model=model), self.assertRaisesRegex(ValueError, model):
                        estimator.rates_for(model)
                    with self.subTest(cost_model=model), self.assertRaises(ValueError):
                        estimator.cost_of_call(model, in_uncached=100, out=10)

    def test_explicit_local_and_priced_entries_keep_their_rates(self):
        for variant, modules in MODULES.items():
            estimator = modules["cost_estimate"]
            with self.subTest(variant=variant):
                self.assertEqual(estimator.cost_of_call("aroow-rust-coder-9b", 1000, out=100), 0)
                self.assertEqual(estimator.cost_of_call("claude-opus-4-8", 1_000_000, out=1_000_000), 30)


if __name__ == "__main__":
    unittest.main()
