CREATE TABLE edges (
       from_uuid TEXT,
       to_uuid TEXT,

       PRIMARY KEY(from_uuid, to_uuid)
);
