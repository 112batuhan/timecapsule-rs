use sea_orm::ActiveValue::{self, Set};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};

use super::{Db, DbError};
use crate::entities::prelude::*;
use crate::entities::*;

impl Db {
    pub async fn create_email(
        &self,
        owner: i64,
        subject: String,
        is_html: bool,
        body: String,
        send_date: chrono::NaiveDate,
    ) -> Result<(), DbError> {
        let email_to_insert = emails::ActiveModel {
            owner: Set(owner),
            subject: Set(subject),
            is_html: Set(is_html),
            body: Set(body),
            send_date: Set(send_date),
            ..Default::default()
        };
        // No explicit error handling needed for this operation as there can't be any unique key
        // violations. Maybe in the future we can set a limit per person.
        Emails::insert(email_to_insert).exec(&self.pg_con).await?;
        Ok(())
    }

    pub async fn get_emails_by_user(&self, user_id: i64) -> Result<Vec<emails::Model>, DbError> {
        let email_vec = emails::Entity::find()
            .filter(emails::Column::Owner.eq(user_id))
            .all(&self.pg_con)
            .await?;

        if email_vec.is_empty() {
            Err(DbError::EmptyQuery)
        } else {
            Ok(email_vec)
        }
    }

    pub async fn get_email_by_id(&self, email_id: i64) -> Result<emails::Model, DbError> {
        let email = emails::Entity::find_by_id(email_id)
            .one(&self.pg_con)
            .await?;
        email.ok_or(DbError::EmptyQuery)
    }

    pub async fn delete_email(&self, email: emails::Model) -> Result<(), DbError> {
        let email: emails::ActiveModel = email.into();
        let delete_result = email.delete(&self.pg_con).await?;
        if delete_result.rows_affected == 0 {
            Err(DbError::EmptyQuery)
        } else {
            Ok(())
        }
    }

    pub async fn update_email(
        &self,
        email: emails::Model,
        subject: String,
        is_html: bool,
        body: String,
        send_date: chrono::NaiveDate,
    ) -> Result<emails::Model, DbError> {
        let mut email: emails::ActiveModel = email.into();
        email.subject = Set(subject);
        email.is_html = Set(is_html);
        email.body = Set(body);
        email.send_date = Set(send_date);

        Ok(email.update(&self.pg_con).await?)
    }

    pub async fn hide_email(&self, email: emails::Model) -> Result<emails::Model, DbError> {
        let mut email: emails::ActiveModel = email.into();
        email.is_hidden = Set(true);

        Ok(email.update(&self.pg_con).await?)
    }

    pub async fn duplicate_email(&self, email: emails::Model) -> Result<emails::Model, DbError> {
        let new_subject = format!("copy of {}", email.subject);
        let mut email: emails::ActiveModel = email.into();
        email.id = ActiveValue::default();
        email.subject = Set(new_subject);
        Ok(email.insert(&self.pg_con).await?)
    }
}
