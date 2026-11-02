import unittest
from unittest.mock import Mock, patch

from genesisdb import GenesisClient
from genesisdb.models import Edge, Node


class GenesisClientTests(unittest.TestCase):
    @patch("genesisdb.client.requests.post")
    def test_query_posts_hql(self, post):
        response = Mock(status_code=200)
        response.json.return_value = {"nodes": [], "edges": []}
        post.return_value = response

        client = GenesisClient("http://localhost:3000/")
        result = client.query("MATCH (n) RETURN n")

        self.assertEqual(result, {"nodes": [], "edges": []})
        post.assert_called_once_with(
            "http://localhost:3000/v1/query/hql",
            headers={"Content-Type": "application/json"},
            timeout=10.0,
            json={"query": "MATCH (n) RETURN n"},
        )

    @patch("genesisdb.client.requests.post")
    def test_add_node_maps_response(self, post):
        response = Mock(status_code=200)
        response.json.return_value = {
            "id": "py-node",
            "labels": ["Doc"],
            "props": {"title": "Python"},
            "impact": 1.0,
            "lang": "en",
            "expires_at": None,
        }
        post.return_value = response

        client = GenesisClient()
        node = client.add_node(labels=["Doc"], id="py-node", props={"title": "Python"})

        self.assertEqual(node.id, "py-node")
        self.assertEqual(node.labels, ["Doc"])
        self.assertEqual(node.props["title"], "Python")
        self.assertNotIn("impact", node.__dataclass_fields__)

    def test_models_preserve_legacy_metadata_without_importance_policy(self):
        props = {"legacy.genesisdb.impact": 0.9}
        node = Node(id="historic", labels=["Doc"], props=props)
        edge = Edge(id="historic-edge", from_id="historic", to_id="other", rel="LINK", props=props)
        self.assertEqual(node.props["legacy.genesisdb.impact"], 0.9)
        self.assertEqual(edge.props["legacy.genesisdb.impact"], 0.9)
        self.assertNotIn("impact", Node.__dataclass_fields__)
        self.assertNotIn("impact", Edge.__dataclass_fields__)


if __name__ == "__main__":
    unittest.main()
