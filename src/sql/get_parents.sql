SELECT nodes.uuid
FROM edges
INNER JOIN nodes
  ON edges.from_uuid = nodes.uuid
WHERE edges.to_uuid = ?
