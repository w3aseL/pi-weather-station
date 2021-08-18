pub mod process;
pub mod types;

use postgres::{ Client };

pub trait DatabaseType {
    fn create_table(client: &mut Client);
    fn insert(&self, client: &mut Client);
    fn insert_many<T: DatabaseType>(data: Vec<T>, client: &mut Client);
    fn find_all<T: DatabaseType>() -> Vec<T>;
}