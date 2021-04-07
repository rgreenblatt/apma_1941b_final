CREATE TABLE repos (
  id SERIAL PRIMARY KEY,
  github_id INT NOT NULL

  /* NOTE: this constraint has been removed because it makes things 
   * too slow. However, it should always (at least nearly) hold true.
   */
  /* CONSTRAINT repos_github_id_unique UNIQUE (github_id) */
);

CREATE INDEX repo_github_id_index
on repos (github_id);
