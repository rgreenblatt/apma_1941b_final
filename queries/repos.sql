SELECT
  repo_github_id,
FROM (
  SELECT
    repo_github_id,
    user_github_id,
  FROM (
    SELECT
      repo.id AS repo_github_id,
      actor.id AS user_github_id,
    FROM
      `githubarchive.year.20*`
    WHERE
      repo.id IS NOT NULL
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
    user_github_id )
GROUP BY
  repo_github_id
HAVING
  COUNT(*) > 1
ORDER BY
  repo_github_id
