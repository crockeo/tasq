SELECT nodes.uuid
FROM edges
INNER JOIN nodes
  ON edges.to_uuid = nodes.uuid
WHERE edges.from_uuid = ?
