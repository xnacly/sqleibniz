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

-- https://www.sqlite.org/lang_createtable.html
CREATE TABLE users (id INTEGER, metadata ANY) STRICT;
CREATE TEMP TABLE IF NOT EXISTS main.users (id INTEGER, name TEXT) STRICT;
CREATE TABLE strict_users_without_rowid (id INTEGER PRIMARY KEY) STRICT, WITHOUT ROWID;
CREATE TABLE memberships (
    user_id INTEGER,
    team_id INTEGER,
    PRIMARY KEY (user_id, team_id) ON CONFLICT REPLACE
) STRICT;
CREATE TABLE emails (
    email TEXT NOT NULL,
    CONSTRAINT unique_email UNIQUE (email COLLATE nocase) ON CONFLICT IGNORE
) STRICT;
CREATE TABLE checked_users (
    name TEXT,
    CHECK ('literal')
) STRICT;
CREATE TABLE user_teams (
    team_id INTEGER,
    FOREIGN KEY (team_id) REFERENCES teams (id) ON DELETE CASCADE
) STRICT;

-- https://www.sqlite.org/lang_createindex.html
CREATE INDEX idx_users_id ON users (id);
CREATE UNIQUE INDEX IF NOT EXISTS main.idx_users_email ON users (email);
CREATE INDEX idx_users_name ON users (name COLLATE nocase DESC, id ASC);

-- https://www.sqlite.org/lang_createtrigger.html
CREATE TRIGGER user_ai AFTER INSERT ON users BEGIN SELECT 1; END;
CREATE TRIGGER user_i INSERT ON users BEGIN SELECT 1; END;
CREATE TRIGGER user_au AFTER UPDATE ON users BEGIN UPDATE users SET name = new.name; END;
CREATE TEMP TRIGGER IF NOT EXISTS temp.user_update INSTEAD OF UPDATE OF name, email ON users FOR EACH ROW WHEN old_name BEGIN UPDATE users SET name = new.name; END;
CREATE TRIGGER user_ad BEFORE DELETE ON users BEGIN INSERT INTO audit VALUES (old.id); DELETE FROM sessions WHERE user_id = old.id; END;

-- https://www.sqlite.org/lang_delete.html
DELETE FROM users;
DELETE FROM main.users WHERE id = 1;
DELETE FROM logs WHERE created_at < 10 ORDER BY created_at DESC NULLS LAST LIMIT 5 OFFSET 2;
DELETE FROM users WHERE id = 1 RETURNING *, users.*, id AS deleted_id;

-- https://www.sqlite.org/lang_insert.html
INSERT INTO users DEFAULT VALUES;
INSERT INTO main.users (id, name) VALUES (1, 'Ada');
INSERT INTO users (id, name) VALUES (1, 'Ada'), (2, 'Grace');
INSERT OR IGNORE INTO users (id) VALUES (1) RETURNING *, id AS inserted_id;

-- https://www.sqlite.org/pragma.html
PRAGMA database_list;
PRAGMA schema.cache_size = 5;
PRAGMA schema.locking_mode = EXCLUSIVE;
PRAGMA foreign_keys = true;
PRAGMA schema.optimize(0xfffe);
PRAGMA application_id;
