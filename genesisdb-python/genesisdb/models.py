from dataclasses import dataclass
from typing import List, Dict, Any, Optional

@dataclass
class Node:
    id: str
    labels: List[str]
    props: Dict[str, Any]
    impact: Optional[float] = None
    embedding: Optional[List[float]] = None
    lang: Optional[str] = None
    valid_from: Optional[str] = None
    expires_at: Optional[str] = None

@dataclass
class Edge:
    id: str
    from_id: str
    to_id: str
    rel: str
    props: Dict[str, Any]
    impact: Optional[float] = None

@dataclass
class ContextPackage:
    nodes: List[Node]
    edges: List[Edge]
    super_nodes: List[Dict[str, Any]]
    token_estimate: int
    reasoning_path: str
