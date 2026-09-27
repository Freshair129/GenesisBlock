USE kb FROM NODES Customer AS c
|> INNER JOIN TABLE orders AS o ON c.id = o.customer_id
|> FILTER o.total > decimal("1000.00")
|> GROUP BY c.id AS customer AGG sum(o.total) AS revenue
|> ORDER BY revenue DESC, customer ASC
|> RETURN customer, revenue;
