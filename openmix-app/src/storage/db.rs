pub fn init(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute_batch(include_str!("schema.sql"))
}

pub fn migrate(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    init(conn)?;
    conn.pragma_update(None, "user_version", 2)
}
