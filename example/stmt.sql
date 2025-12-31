-- vim: filetype=sql
/* stmt.sql displays the current progress of sqleibniz by highlighting all currently available statements. */

-- https://www.sqlite.org/lang_explain.html
EXPLAIN VACUUM;
EXPLAIN QUERY PLAN VACUUM;

-- https://www.sqlite.org/lang_vacuum.html
VACUUM;
VACUUM schema_name;
VACUUM INTO 'filename';
VACUUM schema_name INTO 'filename';

/* ---- https://www.sqlite.org/lang_transaction.html ---- */
-- https://www.sqlite.org/syntax/begin-stmt.html
BEGIN;
BEGIN TRANSACTION;
BEGIN DEFERRED;
BEGIN IMMEDIATE;
BEGIN EXCLUSIVE;
BEGIN DEFERRED TRANSACTION;
BEGIN IMMEDIATE TRANSACTION;
BEGIN EXCLUSIVE TRANSACTION;

-- https://www.sqlite.org/syntax/commit-stmt.html
COMMIT;
END;
COMMIT TRANSACTION;
END TRANSACTION;

-- https://www.sqlite.org/syntax/rollback-stmt.html
ROLLBACK;
ROLLBACK TO save_point;
ROLLBACK TO SAVEPOINT save_point;
ROLLBACK TRANSACTION;
ROLLBACK TRANSACTION TO save_point;
ROLLBACK TRANSACTION TO SAVEPOINT save_point;
/* ------------------------------------------------------ */

-- https://www.sqlite.org/lang_detach.html
DETACH schema_name;
DETACH DATABASE schema_name;

-- https://www.sqlite.org/lang_analyze.html
ANALYZE;
ANALYZE schema_name;
ANALYZE index_or_table_name.index_or_table_name;
ANALYZE schema_name.index_or_table_name;

-- https://www.sqlite.org/lang_dropindex.html
DROP INDEX index_name;
DROP INDEX IF EXISTS schema_name.index_name;

-- https://www.sqlite.org/lang_droptable.html
DROP TABLE table_name;
DROP TABLE IF EXISTS schema_name.table_name;

-- https://www.sqlite.org/lang_droptrigger.html
DROP TRIGGER trigger_name;
DROP TRIGGER IF EXISTS schema_name.trigger_name;

-- https://www.sqlite.org/lang_dropview.html
DROP VIEW view_name;
DROP VIEW IF EXISTS schema_name.view_name;

-- https://www.sqlite.org/lang_savepoint.html
SAVEPOINT savepoint_name;

-- https://www.sqlite.org/syntax/release-stmt.html
RELEASE savepoint_name;
RELEASE SAVEPOINT savepoint_name;


-- https://www.sqlite.org/lang_attach.html
ATTACH DATABASE 'users.db' AS users;
ATTACH 'users.db' AS users;

-- https://www.sqlite.org/syntax/reindex-stmt.html
REINDEX;
REINDEX collation_name;
REINDEX schema_name.table_name;

-- https://www.sqlite.org/lang_altertable.html
ALTER TABLE schema.table_name RENAME TO new_table;
ALTER TABLE schema.table_name RENAME old_column_name TO new_column_name;
ALTER TABLE schema.table_name RENAME COLUMN old_column_name TO new_column_name;

ALTER TABLE schema.table_name ADD new_column_name TEXT;
ALTER TABLE schema.table_name ADD COLUMN new_column_name TEXT;

ALTER TABLE schema.table_name DROP column_name;
ALTER TABLE schema.table_name DROP COLUMN column_name;
