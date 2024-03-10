use entity::{pastes, users};
use sea_orm::{sea_query::Query, ActiveModelTrait, ActiveValue, DbConn, DbErr};

use crate::utils::{self, is_url};

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

        let mut paste: pastes::ActiveModel =
            crate::Query::get_paste_by_id(db, paste_id).await?.into();
        paste.content = ActiveValue::Set(form_data.content.clone());
        paste.is_url = ActiveValue::Set(is_url);

        paste.update(db).await
    }
}
