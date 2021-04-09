CREATE TABLE repos (
  id SERIAL PRIMARY KEY,
  github_id INT NOT NULL,

  CONSTRAINT repos_github_id_unique UNIQUE (github_id)
);
