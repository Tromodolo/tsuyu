use sqlx::{MySql, Pool};
use std::{env, error};
use sqlx::mysql::{MySqlPoolOptions};
use bcrypt::{DEFAULT_COST, hash, verify};
use crate::account::{UserLoginData, User, File};
use futures::TryStreamExt;

// Should only be used once
pub async fn initialize_db_pool() -> Result<Pool<MySql>, Box<dyn error::Error>> {
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&env::var("DATABASE_URL")?).await?;
    Ok(pool)
}

pub async fn create_tables (pool: &Pool<MySql>){
    sqlx::query("\
        create table if not exists `users` (
            `id` int PRIMARY KEY AUTO_INCREMENT,
            `username` varchar(64) not null,
            `hashed_password` varchar(255) not null,
            `email` varchar(255),
            `is_admin` bool not null,
            `api_key` varchar(128),
            `last_update` datetime not null,
            `created_at` datetime not null
        );"
    ).execute(pool).await.unwrap();

    sqlx::query("\
        create table if not exists `files` (
            `id` int PRIMARY KEY AUTO_INCREMENT,
            `name` varchar(255) not null,
            `original_name` varchar(255) not null,
            `filetype` varchar(64) not null,
            `file_hash` varchar(255) not null,
            `uploaded_by` int not null,
            `uploaded_by_ip` varchar(50) not null,
            `created_at` datetime not null
        );"
    ).execute(pool).await.unwrap();

    sqlx::query("\
        create table if not exists `banned_ips` (
          `id` int PRIMARY KEY AUTO_INCREMENT,
          `ip` varchar(50)
        );"
    ).execute(pool).await.unwrap();

    // Drop and recreate foreign key to make sure that it always exists only once
    sqlx::query("alter table `files` drop foreign key if exists `files_user_id`;")
         .execute(pool).await.unwrap();
    sqlx::query("alter table `files` add foreign key `files_user_id` (`uploaded_by`) references `users` (`id`);")
        .execute(pool).await.unwrap();
}

pub async fn check_user_login (data: &UserLoginData, pool: &Pool<MySql>) -> Option<User> {
    let row = sqlx::query_as::<_, User>("select * from `users` where `username` = ?")
        .bind(&data.username)
        .fetch_one(pool)
        .await;

    if let Ok(user) = row {
        match verify(&data.password, &user.hashed_password) {
            Ok(_) => return Some(user),
            Err(e) => eprint!("Error when trying to verify user password: {}", e),
        }
    }
    None
}

pub async fn get_user_by_token(token: String, pool: &Pool<MySql>) -> Option<User> {
    let row = sqlx::query_as::<_, User>("select * from `users` where `api_key` = ?")
        .bind(token)
        .fetch_one(pool)
        .await;

    if let Ok(user) = row {
        return Some(user);
    }
    None
}

pub async fn write_file(file: &File, pool: &Pool<MySql>) -> anyhow::Result<()> {
    sqlx::query("insert into `files` (name, original_name, filetype, file_hash, uploaded_by, uploaded_by_ip, created_at) values (?, ?, ?, ?, ?, ?, ?)")
        .bind(&file.name)
        .bind(&file.original_name)
        .bind(&file.filetype)
        .bind(&file.file_hash)
        .bind(&file.uploaded_by)
        .bind(&file.uploaded_by_ip)
        .bind(&file.created_at)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn is_ip_banned(ip: &str, pool: &Pool<MySql>) -> anyhow::Result<bool> {
    let mut rows = sqlx::query("select * from `banned_ips` where `ip` = ?")
        .bind(ip)
        .fetch(pool);

    if let Some(_) = rows.try_next().await? {
        return Ok(true);
    }
    return Ok(false);
}

pub async fn get_existing_file_by_hash(hash: &str, user_id: &i64, pool: &Pool<MySql>) -> Option<File> {
    let row = sqlx::query_as::<_, File>("select * from `files` where `uploaded_by` = ? and `file_hash` = ?")
        .bind(user_id)
        .bind(hash)
        .fetch_one(pool)
        .await;

    if let Ok(file) = row {
        return Some(file);
    }
    None
}