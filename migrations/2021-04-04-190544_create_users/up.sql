CREATE TABLE users (
  id SERIAL PRIMARY KEY,
  login VARCHAR NOT NULL,
  CONSTRAINT users_login_unique UNIQUE (login)
)
