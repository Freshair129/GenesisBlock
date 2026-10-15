import unittest
from unittest.mock import patch

from genesisdb.client import GenesisClient
from genesisdb.exceptions import QueryError


class FakeResponse:
    def __init__(self, status_code, payload, text=""):
        self.status_code = status_code
        self._payload = payload
        self.text = text or str(payload)

    def json(self):
        if isinstance(self._payload, Exception):
            raise self._payload
        return self._payload


class ClientContractTests(unittest.TestCase):
    @patch("genesisdb.client.requests.post")
    def test_query_ir_sends_api_key_and_timeout(self, post):
        post.return_value = FakeResponse(
            200,
            {"contract_version": "query-ir.v1", "operation_kind": "context"},
        )
        client = GenesisClient("http://db", timeout=3.5, api_key="secret")
        request = {
            "contract_version": "query-ir.v1",
            "request_id": "py-1",
            "operation": {
                "kind": "context",
                "target_id": "n1",
                "tier": "H0",
            },
        }

        result = client.execute_query_ir(request)

        self.assertEqual(result["operation_kind"], "context")
        post.assert_called_once_with(
            "http://db/v1/query/ir",
            json=request,
            headers={
                "Content-Type": "application/json",
                "Authorization": "Bearer secret",
            },
            timeout=3.5,
        )

    @patch("genesisdb.client.requests.get")
    def test_capabilities_are_read_with_finite_timeout(self, get):
        get.return_value = FakeResponse(
            200,
            {"operations": {"context": "implemented"}},
        )
        client = GenesisClient(timeout=2.0)

        self.assertEqual(
            client.query_ir_capabilities()["operations"]["context"], "implemented"
        )
        get.assert_called_once_with(
            "http://localhost:3000/v1/query/ir/capabilities",
            headers={"Content-Type": "application/json"},
            timeout=2.0,
        )

    @patch("genesisdb.client.requests.post")
    def test_structured_server_error_is_typed(self, post):
        post.return_value = FakeResponse(
            400,
            {
                "code": "QUERY_CAPABILITY_UNSUPPORTED",
                "message": "context temporal selectors are not implemented",
            },
        )
        client = GenesisClient()

        with self.assertRaises(QueryError) as raised:
            client.execute_query_ir({"operation": {"kind": "context"}})

        self.assertEqual(raised.exception.code, "QUERY_CAPABILITY_UNSUPPORTED")
        self.assertIsNone(raised.exception.stable_code)
        self.assertEqual(raised.exception.legacy_code, "QUERY_CAPABILITY_UNSUPPORTED")
        self.assertEqual(raised.exception.status_code, 400)

    @patch("genesisdb.client.requests.post")
    def test_error_envelope_is_read_while_legacy_fields_remain_supported(self, post):
        post.return_value = FakeResponse(
            400,
            {
                "error": {
                    "category": "validation",
                    "code": "GENESIS_VALIDATION_ERROR",
                    "message": "collection dimension is invalid",
                    "status": 400,
                },
                "code": "QUERY_IR_VALIDATION_FAILED",
                "message": "collection dimension is invalid",
            },
        )
        client = GenesisClient()

        with self.assertRaises(QueryError) as raised:
            client.add_node(["Memory"], id="n1", embedding=[])

        self.assertEqual(raised.exception.code, "GENESIS_VALIDATION_ERROR")
        self.assertEqual(raised.exception.stable_code, "GENESIS_VALIDATION_ERROR")
        self.assertEqual(raised.exception.legacy_code, "QUERY_IR_VALIDATION_FAILED")
        self.assertEqual(raised.exception.category, "validation")
        self.assertEqual(raised.exception.message, "collection dimension is invalid")

    @patch("genesisdb.client.requests.post")
    def test_error_envelope_reads_all_required_categories(self, post):
        cases = [
            ("validation", "GENESIS_VALIDATION_ERROR"),
            ("capability", "GENESIS_CAPABILITY_UNSUPPORTED"),
            ("query", "GENESIS_QUERY_ERROR"),
            ("conflict", "GENESIS_CONFLICT"),
            ("durability", "GENESIS_DURABILITY_ERROR"),
            ("integrity", "GENESIS_INTEGRITY_ERROR"),
        ]
        client = GenesisClient()
        for category, stable_code in cases:
            post.return_value = FakeResponse(
                409,
                {
                    "error": {
                        "category": category,
                        "code": stable_code,
                        "message": f"nested detail for {category}",
                        "status": 409,
                    },
                    "code": f"LEGACY_{category.upper()}",
                    "message": f"legacy detail for {category}",
                },
            )
            with self.subTest(category=category):
                with self.assertRaises(QueryError) as raised:
                    client.execute_query_ir({})
                self.assertEqual(raised.exception.code, stable_code)
                self.assertEqual(raised.exception.stable_code, stable_code)
                self.assertEqual(raised.exception.legacy_code, f"LEGACY_{category.upper()}")
                self.assertEqual(raised.exception.category, category)
                self.assertEqual(
                    raised.exception.message, f"nested detail for {category}"
                )

    def test_timeout_must_be_positive(self):
        with self.assertRaises(ValueError):
            GenesisClient(timeout=0)


if __name__ == "__main__":
    unittest.main()
