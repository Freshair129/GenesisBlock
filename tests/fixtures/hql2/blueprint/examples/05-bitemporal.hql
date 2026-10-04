USE kb AT TX 120 AT VALID "2026-09-01T00:00:00Z"
MATCH (a)-[r:DEPENDS_ON]->(b)
|> RETURN a.id, r.id, b.id;
