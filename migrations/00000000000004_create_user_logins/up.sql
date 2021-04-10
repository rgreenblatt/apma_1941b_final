CREATE TABLE user_logins (
  id SERIAL PRIMARY KEY,
  user_id SERIAL,
  login VARCHAR NOT NULL
);
