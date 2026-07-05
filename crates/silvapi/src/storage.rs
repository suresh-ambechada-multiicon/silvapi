use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result};
use rusqlite::{Connection, params};

use crate::models::{HttpResponse, Workspace};

const DB_FILE: &str = "silvapi.sqlite";
const MAX_RESPONSE_HISTORY_PER_REQUEST: i64 = 20;

pub fn load_workspace() -> Result<Option<Workspace>> {
    let path = db_path()?;
    if !path.exists() {
        return Ok(None);
    }

    let conn = open_db(&path)?;
    let mut stmt = conn.prepare("select json from workspace where id = 1")?;
    let mut rows = stmt.query([])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };

    let json: String = row.get(0)?;
    let mut workspace: Workspace =
        serde_json::from_str(&json).context("failed to parse saved workspace")?;
    let migrated_cache = std::mem::take(&mut workspace.response_cache);
    if !migrated_cache.is_empty() {
        save_response_cache(&conn, &migrated_cache)?;
    }
    workspace.response_cache = load_response_cache(&conn)?;
    Ok(Some(workspace))
}

pub fn save_workspace(workspace: &Workspace) -> Result<()> {
    let path = db_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let conn = open_db(&path)?;
    let json = serde_json::to_string(workspace)?;
    conn.execute(
        "insert into workspace (id, json, updated_at) values (1, ?1, strftime('%s','now'))
         on conflict(id) do update set json = excluded.json, updated_at = excluded.updated_at",
        params![json],
    )?;
    Ok(())
}

pub fn append_response(request_id: &str, response: &HttpResponse) -> Result<()> {
    let path = db_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let conn = open_db(&path)?;
    let json = serde_json::to_string(response)?;
    conn.execute(
        "insert into response_history (request_id, json, created_at)
         values (?1, ?2, strftime('%s','now'))",
        params![request_id, json],
    )?;
    conn.execute(
        "delete from response_history
         where request_id = ?1
           and id not in (
             select id from response_history
             where request_id = ?1
             order by id desc
             limit ?2
           )",
        params![request_id, MAX_RESPONSE_HISTORY_PER_REQUEST],
    )?;
    Ok(())
}

pub fn delete_response_history(request_id: &str) -> Result<()> {
    let path = db_path()?;
    if !path.exists() {
        return Ok(());
    }

    let conn = open_db(&path)?;
    conn.execute(
        "delete from response_history where request_id = ?1",
        params![request_id],
    )?;
    Ok(())
}

pub fn load_theme_name() -> Result<Option<String>> {
    let path = db_path()?;
    if !path.exists() {
        return Ok(None);
    }

    let conn = open_db(&path)?;
    let mut stmt = conn.prepare("select value from settings where key = 'theme'")?;
    let mut rows = stmt.query([])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };

    Ok(Some(row.get(0)?))
}

pub fn save_theme_name(theme_name: &str) -> Result<()> {
    let path = db_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let conn = open_db(&path)?;
    conn.execute(
        "insert into settings (key, value, updated_at) values ('theme', ?1, strftime('%s','now'))
         on conflict(key) do update set value = excluded.value, updated_at = excluded.updated_at",
        params![theme_name],
    )?;
    Ok(())
}

pub fn load_setting(key: &str) -> Result<Option<String>> {
    let path = db_path()?;
    if !path.exists() {
        return Ok(None);
    }

    let conn = open_db(&path)?;
    let mut stmt = conn.prepare("select value from settings where key = ?1")?;
    let mut rows = stmt.query(params![key])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };

    Ok(Some(row.get(0)?))
}

pub fn save_setting(key: &str, value: &str) -> Result<()> {
    let path = db_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let conn = open_db(&path)?;
    conn.execute(
        "insert into settings (key, value, updated_at) values (?1, ?2, strftime('%s','now'))
         on conflict(key) do update set value = excluded.value, updated_at = excluded.updated_at",
        params![key, value],
    )?;
    Ok(())
}

fn open_db(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "pragma journal_mode = WAL;
        pragma synchronous = NORMAL;
        pragma busy_timeout = 5000;

        create table if not exists workspace (
            id integer primary key,
            json text not null,
            updated_at integer not null
        );
        create table if not exists settings (
            key text primary key,
            value text not null,
            updated_at integer not null
        );
        create table if not exists response_history (
            id integer primary key autoincrement,
            request_id text not null,
            json text not null,
            created_at integer not null
        );
        create index if not exists idx_response_history_request_id
            on response_history(request_id, id);",
    )?;
    Ok(conn)
}

fn load_response_cache(
    conn: &Connection,
) -> Result<std::collections::HashMap<String, Vec<HttpResponse>>> {
    let mut stmt = conn.prepare(
        "select request_id, json
         from response_history h
         where h.id in (
           select id from response_history
           where request_id = h.request_id
           order by id desc
           limit ?1
         )
         order by request_id asc, id asc",
    )?;
    let mut rows = stmt.query(params![MAX_RESPONSE_HISTORY_PER_REQUEST])?;
    let mut cache: std::collections::HashMap<String, Vec<HttpResponse>> = Default::default();

    while let Some(row) = rows.next()? {
        let request_id: String = row.get(0)?;
        let json: String = row.get(1)?;
        if let Ok(response) = serde_json::from_str::<HttpResponse>(&json) {
            cache.entry(request_id).or_default().push(response);
        }
    }

    Ok(cache)
}

fn save_response_cache(
    conn: &Connection,
    cache: &std::collections::HashMap<String, Vec<HttpResponse>>,
) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    for (request_id, responses) in cache {
        let existing_count: i64 = tx.query_row(
            "select count(*) from response_history where request_id = ?1",
            params![request_id],
            |row| row.get(0),
        )?;
        if existing_count > 0 {
            continue;
        }
        for response in responses {
            let json = serde_json::to_string(response)?;
            tx.execute(
                "insert into response_history (request_id, json, created_at)
                 values (?1, ?2, strftime('%s','now'))",
                params![request_id, json],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

fn db_path() -> Result<PathBuf> {
    let base = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .context("HOME is not set")?;
    Ok(base.join("silvapi").join(DB_FILE))
}
