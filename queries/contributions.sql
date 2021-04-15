SELECT
  repo_github_id,
  user_github_id,
  COUNT(*) AS num
FROM (
  SELECT
    repo.id AS repo_github_id,
    actor.id AS user_github_id,
  FROM
    `githubarchive.year.20*` t1
  LEFT JOIN
    `gh-archive-data.dataset.repos` t2
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
    AND type != "WatchEvent"
    AND _TABLE_SUFFIX BETWEEN '15'
    AND '20' )
GROUP BY
  repo_github_id,
  user_github_id
ORDER BY
  repo_github_id,
  user_github_id;
