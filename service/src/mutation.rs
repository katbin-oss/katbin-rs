use chrono::Utc;
use entity::{pastes, schema, users};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DbConn, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter,
};

use crate::{
    utils::{self, is_url},
    Query,
};

pub struct Mutation;

impl Mutation {
    #[tracing::instrument]
    pub async fn create_paste(
        db: &DbConn,
        form_data: &pastes::Model,
        current_user: Option<users::Model>,
    ) -> Result<pastes::Model, DbErr> {
        // if the user defined a custom url, use it
        let id = match (current_user.as_ref(), &form_data.custom_url) {
            (Some(_user), Some(custom_url)) => {
                if !custom_url.is_empty() {
                    tracing::debug!("Custom URL is not empty");
                    custom_url.clone()
                } else {
                    tracing::debug!("Custom URL is empty, generating random key");
                    utils::generate_key(10)
                }
            }
            _ => {
                tracing::debug!("Current user is empty, generating random key");
                utils::generate_key(10)
            }
        };
        let is_url = is_url(&form_data.content);

        let paste = pastes::ActiveModel {
            id: ActiveValue::Set(id),
            content: ActiveValue::Set(form_data.content.to_owned()),
            is_url: ActiveValue::Set(is_url),
            belongs_to: match current_user {
                Some(user) => ActiveValue::Set(Some(user.id)),
                None => ActiveValue::NotSet,
            },
        };

        paste.insert(db).await
    }

    #[tracing::instrument]
    pub async fn update_paste_content(
        db: &DbConn,
        form_data: &pastes::Model,
        current_user: Option<users::Model>,
        paste_id: &str,
    ) -> Result<pastes::Model, DbErr> {
        let is_url = is_url(&form_data.content);

        let mut paste: pastes::ActiveModel = Query::get_paste_by_id(db, paste_id).await?.into();
        paste.content = ActiveValue::Set(form_data.content.clone());
        paste.is_url = ActiveValue::Set(is_url);

        paste.update(db).await
    }

    #[tracing::instrument]
    pub async fn register(
        db: &DbConn,
        form_data: &schema::LoginPost,
    ) -> Result<users::Model, DbErr> {
        let existing_user = users::Entity::find()
            .filter(users::Column::Email.eq(form_data.email.clone()))
            .count(db)
            .await?;
        if existing_user != 0 {
            return Err(DbErr::Custom("User already exists!".to_string()));
        }

        let hashed_password = bcrypt::hash(form_data.password.clone(), 10)
            .map_err(|_| DbErr::Custom(String::from("Passwords do not match")))?;

        let user = users::ActiveModel {
            email: ActiveValue::Set(form_data.email.clone()),
            hashed_password: ActiveValue::Set(hashed_password),
            inserted_at: ActiveValue::Set(Utc::now().naive_utc()),
            updated_at: ActiveValue::Set(Utc::now().naive_utc()),
            ..Default::default()
        };
        user.insert(db).await
    }
}
