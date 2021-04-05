CREATE TABLE contributions (
  id SERIAL PRIMARY KEY,
  repo_id SERIAL,
  user_id SERIAL,
  num INT NOT NULL,
  CONSTRAINT contributions_repo_user_unique UNIQUE (repo_id,user_id)
)
