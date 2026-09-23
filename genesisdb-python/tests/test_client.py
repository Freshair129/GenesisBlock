import unittest
from unittest.mock import Mock, patch

from genesisdb import GenesisClient


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


if __name__ == "__main__":
    unittest.main()
