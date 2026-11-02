//! Tiny local HTTP server: serves the page and a small JSON API.
//! Bound to 127.0.0.1 only. POSTs must be application/json and the Host
//! header must be ours, which blocks other websites (CSRF / DNS rebinding)
//! from reading or changing your activity data.

use crate::{db, PORT};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::io::{Cursor, Read};
use tiny_http::{Header, Method, Request, Response, Server};

type Resp = Response<Cursor<Vec<u8>>>;

const INDEX: &str = include_str!("index.html");

#[derive(Deserialize, Serialize)]
struct Config {
    settings: Settings,
}

#[derive(Deserialize, Serialize)]
struct Settings {
    min_secs: u64,
}

fn load_config() -> Config {
    let content =
        fs::read_to_string("config.toml").unwrap_or_else(|_| "settings.min_secs = 15".to_string());
    toml::from_str(&content).unwrap_or(Config {
        settings: Settings { min_secs: 15 },
    })
}

fn index_html() -> String {
    let config = load_config();
    INDEX
        .replace("__VERSION__", env!("CARGO_PKG_VERSION"))
        .replace("__BUILD_NUM__", env!("BUILD_NUMBER"))
        .replace("__BUILD_UNIX__", env!("BUILD_UNIX"))
        .replace("__MIN_SECS__", &config.settings.min_secs.to_string())
}

fn reply(status: u16, ctype: &str, body: String) -> Resp {
    Response::from_string(body)
        .with_status_code(status)
        .with_header(Header::from_bytes("Content-Type", ctype).unwrap())
        .with_header(Header::from_bytes("Cache-Control", "no-store").unwrap())
}

fn json_reply(status: u16, v: Value) -> Resp {
    reply(status, "application/json", v.to_string())
}

fn header<'a>(req: &'a Request, name: &str) -> Option<&'a str> {
    req.headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}

fn query_num(query: &str, key: &str) -> Option<i64> {
    query
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .find(|(k, _)| *k == key)
        .and_then(|(_, v)| v.parse().ok())
}

fn ids(v: &Value) -> Vec<i64> {
    v.as_array()
        .map(|a| a.iter().filter_map(Value::as_i64).collect())
        .unwrap_or_default()
}

fn handle(conn: &Connection, req: &mut Request) -> Result<Resp, String> {
    let host_ok = matches!(header(req, "Host"),
        Some(h) if h == format!("127.0.0.1:{PORT}") || h == format!("localhost:{PORT}"));
    if !host_ok {
        return Ok(json_reply(403, json!({"error": "bad host"})));
    }

    let url = req.url().to_string();
    let (path, query) = url.split_once('?').unwrap_or((&url, ""));
    let method = req.method().clone();

    let mut body = Value::Null;
    if method == Method::Post {
        let ct = header(req, "Content-Type").unwrap_or("");
        if !ct.starts_with("application/json") {
            return Ok(json_reply(415, json!({"error": "json only"})));
        }
        let mut s = String::new();
        req.as_reader()
            .take(1 << 20)
            .read_to_string(&mut s)
            .map_err(|e| e.to_string())?;
        body = serde_json::from_str(&s).map_err(|e| e.to_string())?;
    }

    let from = query_num(query, "from").unwrap_or(0);
    let to = query_num(query, "to").unwrap_or(i64::MAX);

    match (&method, path) {
        (Method::Get, "/") => Ok(reply(200, "text/html; charset=utf-8", index_html())),

        (Method::Get, "/api/debug") => {
            let count: i64 = conn
                .query_row("SELECT count(*) FROM segments", [], |r| r.get(0))
                .map_err(|e| e.to_string())?;
            Ok(json_reply(
                200,
                json!({
                    "segments_count": count,
                    "db_connected": true
                }),
            ))
        }

        (Method::Get, "/api/debug/counts") => {
            let count: i64 = conn
                .query_row(
                    "SELECT count(*) FROM segments WHERE start < ?1 AND end >= ?2",
                    params![to, from],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            Ok(json_reply(
                200,
                json!({
                    "query_count": count,
                    "params": {
                        "to": to,
                        "from": from
                    }
                }),
            ))
        }

        (Method::Get, "/api/segments") => {
            let mut st = conn
                .prepare(
                    "SELECT id, process, title, start, end, active_secs, total_secs, entry_id
                     FROM segments WHERE start < ?1 AND end >= ?2 ORDER BY start",
                )
                .map_err(|e| e.to_string())?;
            let rows = st
                .query_map(params![to, from], |r| {
                    Ok(json!({
                        "id": r.get::<_, i64>(0)?, "process": r.get::<_, String>(1)?,
                        "title": r.get::<_, String>(2)?, "start": r.get::<_, i64>(3)?,
                        "end": r.get::<_, i64>(4)?, "active": r.get::<_, f64>(5)?,
                        "total": r.get::<_, f64>(6)?, "entry_id": r.get::<_, Option<i64>>(7)?,
                    }))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            Ok(json_reply(200, Value::Array(rows)))
        }

        (Method::Get, "/api/entries") => {
            let mut st = conn
                .prepare(
                    "SELECT id, start, end, description, project
                     FROM entries WHERE start < ?1 AND end >= ?2 ORDER BY start",
                )
                .map_err(|e| e.to_string())?;
            let rows = st
                .query_map(params![to, from], |r| {
                    Ok(json!({
                        "id": r.get::<_, i64>(0)?, "start": r.get::<_, i64>(1)?,
                        "end": r.get::<_, i64>(2)?, "description": r.get::<_, String>(3)?,
                        "project": r.get::<_, String>(4)?,
                    }))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            Ok(json_reply(200, Value::Array(rows)))
        }

        (Method::Get, "/api/projects") => {
            let mut st = conn
                .prepare("SELECT DISTINCT project FROM entries ORDER BY project")
                .map_err(|e| e.to_string())?;
            let rows = st
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            Ok(json_reply(200, json!(rows)))
        }

        (Method::Post, "/api/entries") => {
            let start = body["start"].as_i64().ok_or("start")?;
            let end = body["end"].as_i64().ok_or("end")?;
            let desc = body["description"].as_str().unwrap_or("");
            let project = body["project"].as_str().unwrap_or("");
            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO entries (start, end, description, project) VALUES (?1, ?2, ?3, ?4)",
                params![start, end, desc, project],
            )
            .map_err(|e| e.to_string())?;
            let id = tx.last_insert_rowid();
            for sid in ids(&body["segment_ids"]) {
                tx.execute(
                    "UPDATE segments SET entry_id = ?1 WHERE id = ?2",
                    params![id, sid],
                )
                .map_err(|e| e.to_string())?;
            }
            tx.commit().map_err(|e| e.to_string())?;
            Ok(json_reply(200, json!({"id": id})))
        }

        (Method::Post, "/api/entries/delete") => {
            let id = body["id"].as_i64().ok_or("id")?;
            conn.execute(
                "UPDATE segments SET entry_id = NULL WHERE entry_id = ?1",
                [id],
            )
            .map_err(|e| e.to_string())?;
            conn.execute("DELETE FROM entries WHERE id = ?1", [id])
                .map_err(|e| e.to_string())?;
            Ok(json_reply(200, json!({"ok": true})))
        }

        (Method::Post, "/api/segments/delete") => {
            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
            for sid in ids(&body["ids"]) {
                tx.execute(
                    "DELETE FROM segments WHERE id = ?1 AND entry_id IS NULL",
                    [sid],
                )
                .map_err(|e| e.to_string())?;
            }
            tx.commit().map_err(|e| e.to_string())?;
            Ok(json_reply(200, json!({"ok": true})))
        }

        _ => Ok(json_reply(404, json!({"error": "not found"}))),
    }
}

pub fn serve(server: Server) {
    let conn = match db::open() {
        Ok(c) => c,
        Err(e) => {
            db::log(&format!("server: db open failed: {e}"));
            return;
        }
    };
    for mut req in server.incoming_requests() {
        let resp = handle(&conn, &mut req).unwrap_or_else(|e| json_reply(400, json!({"error": e})));
        let _ = req.respond(resp);
    }
}
