use entity::pastes;
use sea_orm::{ActiveModelTrait, ActiveValue, DbConn, DbErr};

use crate::utils::{self, is_url};

pub struct Mutation;

impl Mutation {
    pub async fn create_paste(
        db: &DbConn,
        form_data: &pastes::Model,
    ) -> Result<pastes::Model, DbErr> {
        // if the user defined a custom url, use it
        let id = match &form_data.custom_url {
            Some(custom_url) => custom_url.clone(),
            None => utils::generate_key(10),
        };

        let paste = pastes::ActiveModel {
            id: ActiveValue::Set(id),
            content: ActiveValue::Set(form_data.content.to_owned()),
            is_url: ActiveValue::Set(is_url(&form_data.content)),
            belongs_to: ActiveValue::NotSet, // TODO: add after auth is implemented
        };

        paste.insert(db).await
    }
}
