CREATE TABLE users (
  id SERIAL PRIMARY KEY,
  github_id INT NOT NULL,

  CONSTRAINT users_github_id_unique UNIQUE (github_id)
);

CREATE INDEX user_github_id_index
on users (github_id);
