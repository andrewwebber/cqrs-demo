use async_nats::jetstream::stream::Config;
use async_nats::ConnectOptions;
use nats_es::types::NatsCqrs;
use serde::{Deserialize, Serialize};

use crate::config::cqrs_framework;
use crate::domain::aggregate::BankAccount;
use crate::queries::BankAccountView;
use nats_es::views_repository::NatsViewRepository;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct ApplicationConfig {
    nats_connection: String,
    nats_user: String,
    nats_password: String,
}

#[derive(Clone)]
pub struct ApplicationState {
    pub cqrs: Arc<NatsCqrs<BankAccount>>,
    pub account_query: Arc<NatsViewRepository<BankAccountView, BankAccount>>,
}

pub async fn new_application_state() -> ApplicationState {
    let ApplicationConfig {
        nats_connection,
        nats_user,
        nats_password,
    }: ApplicationConfig = envy::from_env().unwrap();
    println!("connect to nats - {nats_connection}");

    let client = ConnectOptions::new()
        .user_and_password(nats_user, nats_password)
        .connect(nats_connection)
        .await
        .expect("unable to start nats client");
    let jetstream = async_nats::jetstream::new(client.clone());

    jetstream
        .get_or_create_stream(Config {
            name: "events_cqrs_demo".to_string(),
            subjects: vec!["events.account.>".to_string()],
            num_replicas: 1,
            ..Default::default()
        })
        .await
        .unwrap();
    jetstream
        .create_key_value(async_nats::jetstream::kv::Config {
            bucket: "views_cqrs_demo".to_string(),
            ..Default::default()
        })
        .await
        .unwrap();

    println!("creating cqrs framework");
    let (cqrs, account_query) = cqrs_framework(client).await;
    ApplicationState {
        cqrs,
        account_query,
    }
}

#[cfg(test)]
mod tests {
    use crate::{domain::commands::BankAccountCommand, state::new_application_state};
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    #[tokio::test]
    async fn test_cqrs() {
        dotenvy::dotenv().ok();
        tracing_subscriber::registry()
            .with(fmt::layer())
            .with(EnvFilter::from_default_env())
            .init();
        let id = format!("{}", std::process::id());
        let state = new_application_state().await;
        println!("executing command");
        state
            .cqrs
            .execute(
                &id,
                BankAccountCommand::OpenAccount {
                    account_id: id.to_owned(),
                },
            )
            .await
            .expect("unable to execute open account command");

        for _ in 0..4 {
            state
                .cqrs
                .execute(&id, BankAccountCommand::DepositMoney { amount: 20f64 })
                .await
                .expect("unable to execute open account command");
        }
    }
}
