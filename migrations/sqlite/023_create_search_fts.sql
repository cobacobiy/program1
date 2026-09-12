-- Migration: 023_create_search_fts.sql
CREATE VIRTUAL TABLE IF NOT EXISTS catalog_fts USING fts5(
    name,
    description,
    category,
    content='catalog_items',
    content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS catalog_items_ai AFTER INSERT ON catalog_items BEGIN
  INSERT INTO catalog_fts(rowid, name, description, category) 
  VALUES (new.rowid, new.name, coalesce(new.description, ''), new.category);
END;

CREATE TRIGGER IF NOT EXISTS catalog_items_ad AFTER DELETE ON catalog_items BEGIN
  INSERT INTO catalog_fts(catalog_fts, rowid, name, description, category) 
  VALUES('delete', old.rowid, old.name, coalesce(old.description, ''), old.category);
END;

CREATE TRIGGER IF NOT EXISTS catalog_items_au AFTER UPDATE ON catalog_items BEGIN
  INSERT INTO catalog_fts(catalog_fts, rowid, name, description, category) 
  VALUES('delete', old.rowid, old.name, coalesce(old.description, ''), old.category);
  INSERT INTO catalog_fts(rowid, name, description, category) 
  VALUES (new.rowid, new.name, coalesce(new.description, ''), new.category);
END;

INSERT OR IGNORE INTO catalog_fts(rowid, name, description, category)
SELECT rowid, name, coalesce(description, ''), category FROM catalog_items;

CREATE TABLE IF NOT EXISTS search_analytics (
    id TEXT PRIMARY KEY NOT NULL,
    keyword TEXT NOT NULL UNIQUE,
    search_count INTEGER NOT NULL DEFAULT 1,
    last_searched_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_search_analytics_count ON search_analytics(search_count DESC);
