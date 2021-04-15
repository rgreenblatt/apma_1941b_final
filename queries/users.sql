SELECT
  actor.id AS github_id,
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
  AND type != "WatchEvent"
  AND _TABLE_SUFFIX BETWEEN '15'
  AND '20'
GROUP BY
  github_id
ORDER BY
  github_id;
