CREATE TABLE repo_names (
  id SERIAL PRIMARY KEY,
  repo_id SERIAL,
  name VARCHAR NOT NULL,

  CONSTRAINT repo_names_unique UNIQUE (repo_id,name)
);

CREATE INDEX repo_names_id_name_index
on repo_names (repo_id,name);
