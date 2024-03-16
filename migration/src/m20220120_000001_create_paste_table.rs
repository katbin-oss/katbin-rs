use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            "CREATE EXTENSION citext;

            create table public.users
            (
                id              bigserial
                    primary key,
                email           citext       not null,
                hashed_password varchar(255) not null unique,
                confirmed_at    timestamp(0),
                inserted_at     timestamp(0) not null,
                updated_at      timestamp(0) not null
            );

            alter table public.users
                owner to postgres;

            create table public.pastes
            (
                id         varchar(255)          not null
                    primary key,
                is_url     boolean default false not null,
                content    text                  not null,
                belongs_to bigint
                    references public.users
                        on delete cascade
            );

            alter table public.pastes
                owner to postgres;

            create index pastes_belongs_to_index
                on public.pastes (belongs_to);

            create unique index users_email_index
                on public.users (email);

            create table public.users_tokens
            (
                id          bigserial
                    primary key,
                user_id     bigint       not null
                    references public.users
                        on delete cascade,
                token       bytea        not null,
                context     varchar(255) not null,
                sent_to     varchar(255),
                inserted_at timestamp(0) not null
            );

            alter table public.users_tokens
                owner to postgres;

            create unique index users_tokens_context_token_index
                on public.users_tokens (context, token);

            create index users_tokens_user_id_index
                on public.users_tokens (user_id);
        ",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DROP table public.users_tokens;
            DROP table public.pastes;
            DROP table public.users;",
            )
            .await?;
        Ok(())
    }
}
