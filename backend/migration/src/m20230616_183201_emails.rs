use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts

        manager
            .create_table(
                Table::create()
                    .table(Emails::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Emails::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Emails::Owner).big_integer().not_null())
                    .col(ColumnDef::new(Emails::Subject).string().not_null())
                    .col(ColumnDef::new(Emails::IsHtml).boolean().not_null())
                    .col(ColumnDef::new(Emails::Body).string().not_null())
                    .col(ColumnDef::new(Emails::SendDate).date().not_null())
                    .col(
                        ColumnDef::new(Emails::IsSent)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Emails::IsHidden)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts

        manager
            .drop_table(Table::drop().table(Emails::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Emails {
    Table,
    Id,
    Owner,
    Subject,
    IsHtml,
    Body,
    SendDate,
    IsSent,
    IsHidden,
}
