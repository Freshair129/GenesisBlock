USE kb FROM TABLE orders AS o
|> FILTER o.total > decimal("1000.00")
|> AGG count(*) AS n, sum(o.total) AS total
|> RETURN n, total;
