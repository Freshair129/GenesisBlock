import requests
import json
from typing import List, Dict, Any, Optional, Union
from .models import Node, Edge, ContextPackage
from .exceptions import ConnectionError, QueryError

class GenesisClient:
    def __init__(self, base_url: str = "http://localhost:3000"):
        self.base_url = base_url.rstrip("/")
        self._check_connection()

    def _check_connection(self):
        try:
            # We don't have a specific health endpoint yet, 
            # but we can try listing status or just assume alive
            pass
        except Exception as e:
            raise ConnectionError(f"Could not connect to GenesisDB at {self.base_url}: {e}")

    def query(self, hql: str) -> Any:
        """Executes a raw HQL command."""
        url = f"{self.base_url}/v1/query/hql"
        response = requests.post(url, json={"query": hql})
        if response.status_code != 200:
            raise QueryError(f"HQL Error: {response.text}")
        return response.json()

    def add_node(
        self, 
        labels: List[str], 
        id: Optional[str] = None,
        props: Optional[Dict[str, Any]] = None,
        embedding: Optional[List[float]] = None,
        ttl: Optional[int] = None,
        caused_by: Optional[str] = "python-sdk"
    ) -> Node:
        """Adds a new knowledge atom to the graph."""
        url = f"{self.base_url}/v1/node/add"
        payload = {
            "id": id,
            "labels": labels,
            "props": props,
            "embedding": embedding,
            "ttl": ttl,
            "caused_by": caused_by
        }
        response = requests.post(url, json=payload)
        if response.status_code != 200:
            raise QueryError(f"Add Node Error: {response.text}")
        
        data = response.json()
        return Node(
            id=data["id"],
            labels=data["labels"],
            props=data["props"],
            impact=data.get("impact"),
            lang=data.get("lang"),
            expires_at=data.get("expires_at")
        )

    def get_context(
        self, 
        target: str, 
        tier: str = "H1", 
        budget: Optional[int] = None,
        fuzzy: bool = False
    ) -> ContextPackage:
        """Retrieves a tiered context fragment using the GRL protocol."""
        # Using the retrieve_context endpoint if exposed via Axum, 
        # or we use execute_hql with CONTEXT syntax.
        hql = f"CONTEXT FOR {target} TIER {tier}"
        if budget:
            hql += f" BUDGET {budget}"
        if fuzzy:
            hql = hql.replace("FOR ", "FOR ~")
        
        res = self.query(hql)
        
        nodes = [Node(id=n["id"], labels=n["labels"], props=n["props"], impact=n.get("impact")) for n in res["nodes"]]
        edges = [Edge(id=e["id"], from_id=e["from"], to_id=e["to"], rel=e["rel"], props=e["props"]) for e in res["edges"]]
        
        return ContextPackage(
            nodes=nodes,
            edges=edges,
            super_nodes=res.get("super_nodes", []),
            token_estimate=res.get("token_estimate", 0),
            reasoning_path=res.get("reasoning_path", "")
        )
