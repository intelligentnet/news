alter table prompt_embed alter column embedding type vector(384);
alter table news_items alter column embedding type vector(384);
alter table prompt_embed alter column embedding type vector(1536);
alter table news_items alter column embedding type vector(1536);
alter table news_items add created timestamp WITH TIME ZONE NOT NULL default 'now';

select a.prompt, b.title, a.embedding <=> b.embedding score from prompt_embed a, news_items b where a.prompt = b.prompt and a.prompt = 'ukraine' order by score desc;

select a.prompt, b.title from prompt_embed a, news_items b where to_tsvector(b.title) @@ to_tsquery('ukraine') and a.prompt = b.prompt and a.prompt = 'ukraine';
UPDATE news_items a SET seq = 0, dt = now() - interval '4 hours' WHERE EXISTS (SELECT 1 FROM prompt_embed b WHERE b.prompt = 'japan' AND (a.prompt = b.prompt OR a.embedding <=> b.embedding < 0.25));

