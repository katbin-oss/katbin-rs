use entity::pastes;
use sea_orm::{DbConn, DbErr, EntityTrait};

pub struct Query;

impl Query {
    pub async fn get_paste_by_id(db: &DbConn, id: &str) -> Result<pastes::Model, DbErr> {
        match pastes::Entity::find_by_id(id).one(db).await? {
            Some(paste) => Ok(paste),
            None => Err(DbErr::RecordNotFound(String::from("paste not found"))),
        }
    }
}
