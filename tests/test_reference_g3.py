import json
import unittest
from pathlib import Path

from tools.reference_oracle import evaluate_case


FIXTURE = Path(__file__).parent / "fixtures" / "g3_oracle_cases.json"


class ReferenceG3Tests(unittest.TestCase):
    def test_golden_fixtures_match_the_pure_reference_model(self):
        document = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(document["fixture_version"], "g3.oracle.v1")
        for case in document["cases"]:
            with self.subTest(case=case["id"]):
                self.assertEqual(evaluate_case(case), case["expected"])


if __name__ == "__main__":
    unittest.main()
