CREATE USER news superuser password 'news_pass';

CREATE EXTENSION IF NOT EXISTS vector WITH SCHEMA public;

CREATE TABLE news_items (
        url text NOT NULL,
	prompt text NOT NULL,
        source text NOT NULL,
        title text NOT NULL,
        queried bool NOT NULL,
	seq oid NOT NULL,
        dt timestamp WITH TIME ZONE NOT NULL,
        sentiment real NOT NULL default 0.0,
        summary text,
        embedding vector(384),
);

ALTER TABLE news_items OWNER TO news;

ALTER TABLE news_items ADD CONSTRAINT news_pk PRIMARY KEY (url, prompt);
CREATE INDEX new_items_prompt ON news_items (prompt);
CREATE INDEX new_items_dt ON news_items (dt);
CREATE INDEX news_items_embedding ON news_items USING hnsw (embedding public.vector_l2_ops);

CREATE TABLE prompt_embed (
        prompt text NOT NULL PRIMARY KEY,
	format text NOT NULL DEFAULT 'headlines',
        embedding vector(384)
);

CREATE INDEX prompt_embed_idx ON prompt_embed USING hnsw (embedding public.vector_l2_ops);

postgres://u_jqptyf5afj9smjl:nesjj1icuakcxcr@02f7e6f1-1adb-4347-835a-02c74fcccb0e.db.cloud.postgresml.org:6432/pgml_qadtbqpvaiaztvq

