CREATE TABLE contributions (
  id SERIAL PRIMARY KEY,
  repo_id SERIAL,
  user_id SERIAL,
  num INT NOT NULL

  /* NOTE: this constraint has been removed because it makes things 
   * too slow. However, it should always (at least nearly) hold true.
   */
  /* CONSTRAINT contributions_repo_user_unique UNIQUE (repo_id,user_id) */
);

CREATE INDEX contribution_foreign_id_index
on contributions (repo_id, user_id);
