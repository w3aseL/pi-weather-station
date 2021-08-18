use postgres::{ Client, NoTls };

pub fn get_client(addr: String, username: String, password: String, database_name: String) -> Client {
    Client::connect(format!("host={} user={} password={} dbname={}", addr, username, password, database_name).as_str(), NoTls).expect("Failed to connect to database!")
}