SELECT nodes.uuid
FROM nodes
LEFT JOIN edges
  ON nodes.uuid = edges.to_uuid
WHERE nodes.uuid IS NOT NULL
  AND edges.to_uuid IS NULL
