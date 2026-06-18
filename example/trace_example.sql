-- vim: filetype=sql
-- Stable input for the README parser trace example.

EXPLAIN VACUUM;
EXPLAIN QUERY PLAN VACUUM my_big_schema INTO 'repacked.db';
