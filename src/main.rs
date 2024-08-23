use rocket::http::Status;
use rocket::response::status::Custom;
use rocket::serde::json::Json;
use rocket::{get, launch, post, routes};
use rusqlite::{params, Connection, Result};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct User {
    id: Option<i64>,
    name: String,
    password: String,
}

fn setup_db() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute(
        "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT, password TEXT)",
        (),
    )?;

    // Insert 10 rows of dummy data
    let dummy_users = [
        ("Alice", "password123"),
        ("Bob", "password456"),
        ("Charlie", "password789"),
        ("David", "password101"),
        ("Eve", "password102"),
        ("Frank", "password103"),
        ("Grace", "password104"),
        ("Hank", "password105"),
        ("Ivy", "password106"),
        ("Jack", "password107"),
    ];

    for (i, (name, password)) in dummy_users.iter().enumerate() {
        conn.execute(
            "INSERT INTO users ( name, password) VALUES ( ?1, ?2)",
            rusqlite::params![name, password],
        )?;
    }

    Ok(conn)
}
#[get("/user")]
fn get_users() -> Json<Vec<User>> {
    let conn = setup_db().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, name, password FROM users")
        .unwrap();
    let user_iter = stmt
        .query_map([], |row| {
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                password: row.get(2)?,
            })
        })
        .unwrap();

    let users: Vec<User> = user_iter.map(|user| user.unwrap()).collect();
    Json(users)
}
#[post("/user", format = "json", data = "<user>")]
async fn create_user(user: Json<User>) -> Result<Json<User>, Custom<String>> {
    // Set up the database connection
    let conn = match setup_db() {
        Ok(conn) => conn,
        Err(err) => {
            return Err(Custom(
                Status::InternalServerError,
                format!("Database error: {:?}", err),
            ))
        }
    };

    // Execute the insert statement
    match conn.execute(
        "INSERT INTO users (name, password) VALUES (?1, ?2)",
        params![user.name, user.password],
    ) {
        Ok(_) => (),
        Err(err) => {
            return Err(Custom(
                Status::InternalServerError,
                format!("Insert error: {:?}", err),
            ))
        }
    }

    // Retrieve the last inserted row ID
    let last_id = conn.last_insert_rowid();

    // Query the newly inserted user
    let mut stmt = match conn.prepare("SELECT id, name, password FROM users WHERE id = ?1") {
        Ok(stmt) => stmt,
        Err(err) => {
            return Err(Custom(
                Status::InternalServerError,
                format!("Query preparation error: {:?}", err),
            ))
        }
    };

    let user = match stmt.query_row(params![last_id], |row| {
        Ok(User {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            password: row.get(2)?,
        })
    }) {
        Ok(user) => user,
        Err(err) => {
            return Err(Custom(
                Status::InternalServerError,
                format!("Query execution error: {:?}", err),
            ))
        }
    };

    Ok(Json(user))
}

fn is_authenticated(conn: Connection, user: Json<User>) -> bool {
    conn.execute(
        "SELECT * FROM users WHERE name = ? AND PASSWORD = ?",
        [&user.name, &user.password],
    )
    .is_ok()
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![get_users, create_user])
}
