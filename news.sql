CREATE USER <Account name> superuser password '<My Password>';

CREATE TABLE news_items (
        url text NOT NULL,
	prompt text NOT NULL,
        source text NOT NULL,
        title text NOT NULL,
        queried bool NOT NULL,
	seq oid NOT NULL,
        dt timestamp WITH TIME ZONE NOT NULL,
        summary text;
);

ALTER TABLE news_items OWNER TO <Account name>;

ALTER TABLE news_items ADD CONSTRAINT news_pk PRIMARY KEY (url, prompt);
CREATE INDEX new_items_prompt ON news_items (prompt);
CREATE INDEX new_items_dt ON news_items (dt);
CREATE INDEX news_items_embedding ON news_items USING hnsw (embedding public.vector_l2_ops);
