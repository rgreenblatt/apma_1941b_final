CREATE TABLE repos (
  id SERIAL PRIMARY KEY,
  owner_name VARCHAR NOT NULL,
  CONSTRAINT repos_owner_name_unique UNIQUE (owner_name)
)

