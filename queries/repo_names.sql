SELECT
  repo.id AS github_id,
  repo.name AS name
FROM
  `githubarchive.year.20*` t_outer
INNER JOIN (
  SELECT
    repo.id AS github_id,
    MAX(created_at) AS max_created_at
  FROM
    `githubarchive.year.20*` t1
  LEFT JOIN
    `gh-archive-data.test_dataset.repos` t2
  ON
    t2.repo_github_id = t1.repo.id
  WHERE
    t2.repo_github_id IS NOT NULL
    AND repo.id IS NOT NULL
    AND actor.id IS NOT NULL
    AND type != "ForkEvent"
    AND type != "DeleteEvent"
    AND type != "MemberEvent"
    AND type != "SponsorshipEvent"
    and type != "WatchEvent"
    AND _TABLE_SUFFIX BETWEEN '15'
    AND '20'
  GROUP BY
    github_id ) t_inner
ON
  t_outer.repo.id = t_inner.github_id
  AND t_outer.created_at = t_inner.max_created_at
GROUP BY
  github_id,
  name
ORDER BY
  github_id,
  name;
