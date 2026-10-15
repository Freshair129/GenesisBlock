CREATE INDEX reviewed_annotations ON ANNOTATION (author, kind) USING BTREE WHERE status = "approved";
