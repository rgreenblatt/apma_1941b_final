CREATE TABLE dependencies (
  id SERIAL PRIMARY KEY,
  repo_from_id SERIAL,
  repo_to_id SERIAL

  /* NOTE: this constraint has been removed because it makes things 
   * too slow. However, it should always (at least nearly) hold true.
   */
  /* CONSTRAINT dependencies_from_to_unique UNIQUE (repo_from_id,repo_to_id) */
);

CREATE INDEX dependencies_foreign_id_index
on dependencies (repo_from_id, repo_to_id);
