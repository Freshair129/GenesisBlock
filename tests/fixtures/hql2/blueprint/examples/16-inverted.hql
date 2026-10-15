CREATE INDEX docs_text ON NODE Document (text) USING INVERTED WITH $analyzer_config;
