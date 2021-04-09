CREATE TABLE contributions (
  id SERIAL PRIMARY KEY,
  repo_id SERIAL,
  user_id SERIAL,
  num INT NOT NULL
);
