UPDATE nodes
SET title = ?,
    description = ?,
    scheduled = ?,
    due = ?,
    completed = ?,
    trashed = ?
WHERE uuid = ?
