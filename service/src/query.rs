use entity::{pastes, schema, users};
use sea_orm::{ColumnTrait, DbConn, DbErr, EntityTrait, QueryFilter};

pub struct Query;

impl Query {
    pub async fn get_paste_by_id(db: &DbConn, id: &str) -> Result<pastes::Model, DbErr> {
        match pastes::Entity::find_by_id(id).one(db).await? {
            Some(paste) => Ok(paste),
            None => Err(DbErr::RecordNotFound(String::from("paste not found"))),
        }
    }

    pub async fn login(db: &DbConn, form: &schema::LoginPost) -> Result<users::Model, DbErr> {
        let user = Query::get_user_by_email(db, &form.email).await?;

        let verified = bcrypt::verify(&form.password, &user.hashed_password)
            .map_err(|_| DbErr::RecordNotFound(String::from("Passwords do not match")))?;
        if !verified {
            return Err(DbErr::RecordNotFound(String::from(
                "Passwords do not match",
            )));
        }

        Ok(user)
    }

    pub async fn get_user_by_email(db: &DbConn, email: &str) -> Result<users::Model, DbErr> {
        let user = users::Entity::find()
            .filter(users::Column::Email.eq(email))
            .one(db)
            .await?;

        match user {
            Some(u) => Ok(u),
            None => Err(DbErr::RecordNotFound(String::from("User not found")).into()),
        }
    }
}
