use client_side::domain::{IdempotencyKey, PageSize};
use client_side::infra::network::{HttpWorker, Identity, RealHttpWorker};
use nanoid::nanoid;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("client_side=trace,http_worker_demo=trace"))
        .init();

    let worker = RealHttpWorker::new();

    let alphabet: [char; 16] = [
        '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', 'a', 'b', 'c', 'd', 'e', 'f',
    ];
    let run_id = nanoid!(10, &alphabet);

    const USERNAME_PREFIX: &str = "testuser";
    const PASSWORD: &str = "testpass";
    let mut users: Vec<(String, String, Identity)> = Vec::new();

    for i in 0..2 {
        let username = format!("{}{}_{}", USERNAME_PREFIX, i, run_id);
        let password = PASSWORD.to_owned();

        match worker
            .signup(
                username.clone(),
                password.clone(),
                Uuid::nil(),
                "123456".to_string(),
            )
            .await
        {
            Ok(_) => println!("Success"),
            Err(_) => println!("Failed"),
        }

        match worker
            .login(
                username.clone(),
                password.clone(),
                Uuid::nil(),
                "123456".to_string(),
            )
            .await
        {
            Ok(identity) => {
                users.push((username, password, identity));
            }
            Err(e) => println!("{}", e),
        }
    }

    let conv = worker.add_friend(
        users[0].2.auth_tokens.access_token.clone(),
        &users[1].0,
        IdempotencyKey(uuid::Uuid::nil()),
    ).await;
    tracing::trace!("{:?}", conv);

    let friends = worker.fetch_friend_list(
        users[0].2.auth_tokens.access_token.clone(),
        PageSize(20),
        None,
    ).await;
    tracing::trace!("{:?}", friends);
}
