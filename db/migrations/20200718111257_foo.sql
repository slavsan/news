CREATE TABLE sources (
  id INT,
  name VARCHAR,
  url VARCHAR
);

CREATE TABLE articles (
  id INT,
  title VARCHAR,
  url VARCHAR
);

CREATE TABLE source_articles (
  source_id INT,
  article_id INT
);

CREATE UNIQUE INDEX unique_article_url_idx ON articles(url);
CREATE UNIQUE INDEX unique_source_url_idx ON sources(url);
CREATE UNIQUE INDEX unique_source_name_idx ON sources(name);
CREATE UNIQUE INDEX unique_source_article_idx ON source_articles(source_id, article_id);
