use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;

pub struct OfflineStore { db: Mutex<Connection> }
#[derive(Serialize)] pub struct OfflineStatus { pending: i64, conflicts: i64 }
#[derive(Serialize)] #[serde(rename_all = "camelCase")]
pub struct OfflineCommand { pub idempotency_key: String, pub action: String, pub payload: serde_json::Value, pub created_at: String }

impl OfflineStore {
    pub fn open(path: &Path) -> Result<Self, String> {
        let db = Connection::open(path).map_err(|e| e.to_string())?;
        db.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS offline_outbox (id INTEGER PRIMARY KEY, idempotency_key TEXT UNIQUE NOT NULL, action TEXT NOT NULL, payload TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'pending', retry_count INTEGER NOT NULL DEFAULT 0, conflict TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, next_attempt_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP); CREATE TABLE IF NOT EXISTS offline_catalog (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);").map_err(|e| e.to_string())?;
        Ok(Self { db: Mutex::new(db) })
    }
    pub fn enqueue(&self, key: &str, action: &str, payload: &str) -> Result<(), String> { self.db.lock().map_err(|_| "offline database lock poisoned".to_string())?.execute("INSERT INTO offline_outbox(idempotency_key,action,payload) VALUES(?1,?2,?3) ON CONFLICT(idempotency_key) DO NOTHING", params![key,action,payload]).map_err(|e| e.to_string())?; Ok(()) }
    pub fn status(&self) -> Result<OfflineStatus, String> { let db=self.db.lock().map_err(|_| "offline database lock poisoned".to_string())?; Ok(OfflineStatus { pending: db.query_row("SELECT COUNT(*) FROM offline_outbox WHERE status='pending'",[],|r|r.get(0)).map_err(|e|e.to_string())?, conflicts: db.query_row("SELECT COUNT(*) FROM offline_outbox WHERE status='conflict'",[],|r|r.get(0)).map_err(|e|e.to_string())? }) }
    pub fn pending(&self)->Result<Vec<OfflineCommand>,String>{let db=self.db.lock().map_err(|_|"offline database lock poisoned".to_string())?;let mut stmt=db.prepare("SELECT idempotency_key,action,payload,created_at FROM offline_outbox WHERE status='pending' AND next_attempt_at<=CURRENT_TIMESTAMP ORDER BY id LIMIT 50").map_err(|e|e.to_string())?;stmt.query_map([],|r|{let p:String=r.get(2)?;Ok(OfflineCommand{idempotency_key:r.get(0)?,action:r.get(1)?,payload:serde_json::from_str(&p).unwrap_or(serde_json::Value::Null),created_at:r.get(3)?})}).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())}
    pub fn mark(&self,key:&str,status:&str,conflict:Option<&str>)->Result<(),String>{let db=self.db.lock().map_err(|_|"offline database lock poisoned".to_string())?;match status{"completed"=>db.execute("UPDATE offline_outbox SET status='completed' WHERE idempotency_key=?1",params![key]),"conflict"=>db.execute("UPDATE offline_outbox SET status='conflict',conflict=?2 WHERE idempotency_key=?1",params![key,conflict]),"retry"=>db.execute("UPDATE offline_outbox SET retry_count=retry_count+1,next_attempt_at=datetime('now','+' || MIN(retry_count+1,8) || ' minutes') WHERE idempotency_key=?1",params![key]),_=>return Err("unsupported sync status".into())}.map_err(|e|e.to_string())?;Ok(())}
    pub fn replace_catalog(&self,s:&str)->Result<(),String>{self.db.lock().map_err(|_|"offline database lock poisoned".to_string())?.execute("INSERT INTO offline_catalog(key,value,updated_at) VALUES('bootstrap',?1,CURRENT_TIMESTAMP) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=CURRENT_TIMESTAMP",params![s]).map_err(|e|e.to_string())?;Ok(())}
    pub fn catalog(&self)->Result<Option<serde_json::Value>,String>{let db=self.db.lock().map_err(|_|"offline database lock poisoned".to_string())?;let s:Option<String>=db.query_row("SELECT value FROM offline_catalog WHERE key='bootstrap'",[],|r|r.get(0)).optional().map_err(|e|e.to_string())?;s.map(|v|serde_json::from_str(&v).map_err(|e|e.to_string())).transpose()}
}
