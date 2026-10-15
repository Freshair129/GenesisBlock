USE kb FROM NODES Customer AS c
|> SEMI JOIN TABLE orders AS o ON c.id = o.customer_id
|> RETURN c.id;
