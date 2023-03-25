SELECT uuid
FROM nodes
WHERE due IS NULL
  AND trashed = 0
