USE kb MATCH SHORTEST (a {id:"A"})-[:DEPENDS_ON*1..6]->(b {id:"B"}) AS p SIMPLE
|> RETURN p;
