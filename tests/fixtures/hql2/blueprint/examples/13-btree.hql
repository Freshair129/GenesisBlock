CREATE INDEX docs_language_date ON NODE Document (language, published_at) USING BTREE INCLUDE (title);
