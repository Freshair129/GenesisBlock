import requests
from typing import List, Dict, Any, Optional, Union
from .models import CoverageReport, Node, Edge, ContextPackage
from .exceptions import ConnectionError, QueryError

class GenesisClient:
    def __init__(
        self,
        base_url: str = "http://localhost:3000",
        timeout: float = 10.0,
        api_key: Optional[str] = None,
    ):
        self.base_url = base_url.rstrip("/")
        if timeout <= 0:
            raise ValueError("timeout must be positive")
        self.timeout = timeout
        self.api_key = api_key
        self._check_connection()

    def _check_connection(self):
        try:
            # We don't have a specific health endpoint yet, 
            # but we can try listing status or just assume alive
            pass
        except Exception as e:
            raise ConnectionError(f"Could not connect to GenesisBlockDB at {self.base_url}: {e}")

    def _headers(self) -> Dict[str, str]:
        headers = {"Content-Type": "application/json"}
        if self.api_key:
            headers["Authorization"] = f"Bearer {self.api_key}"
        return headers

    def _request(self, method: str, path: str, payload: Optional[Any] = None) -> Any:
        url = f"{self.base_url}{path}"
        request = requests.post if method == "POST" else requests.get
        kwargs = {"headers": self._headers(), "timeout": self.timeout}
        if payload is not None:
            kwargs["json"] = payload
        try:
            response = request(url, **kwargs)
        except requests.RequestException as exc:
            raise ConnectionError(f"Request to GenesisBlockDB failed: {exc}") from exc
        if response.status_code < 200 or response.status_code >= 300:
            try:
                error_body = response.json()
            except ValueError:
                error_body = {}
            code = error_body.get("code", "QUERY_EXECUTION_FAILED")
            message = error_body.get("message", response.text)
            raise QueryError(message, status_code=response.status_code, code=code)
        try:
            return response.json()
        except ValueError as exc:
            raise QueryError(
                "Server returned invalid JSON",
                status_code=response.status_code,
                code="QUERY_EXECUTION_FAILED",
            ) from exc

    def query(self, hql: str) -> Any:
        """Executes a raw HQL command."""
        return self._request("POST", "/v1/query/hql", {"query": hql})

    def execute_query_ir(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """Executes a closed, versioned Query IR request."""
        return self._request("POST", "/v1/query/ir", request)

    def query_ir_capabilities(self) -> Dict[str, Any]:
        """Returns the server's operation and boundary capability manifest."""
        return self._request("GET", "/v1/query/ir/capabilities")

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
        payload = {
            "id": id,
            "labels": labels,
            "props": props,
            "embedding": embedding,
            "ttl": ttl,
            "caused_by": caused_by
        }
        data = self._request("POST", "/v1/node/add", payload)
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
            reasoning_path=res.get("reasoning_path", ""),
            coverage=CoverageReport(**res["coverage"]) if res.get("coverage") else None,
        )
