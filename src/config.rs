use std::sync::Arc;

use async_nats::Client;
use cqrs_es::persist::PersistedEventStore;
use cqrs_es::{Aggregate, CqrsFramework, Query};

use crate::domain::aggregate::BankAccount;
use crate::queries::{AccountQuery, BankAccountView, SimpleLoggingQuery};
use crate::services::{BankAccountServices, HappyPathBankAccountServices};
use nats_es::event_repository::NatsEventStore;
use nats_es::types::NatsCqrs;
use nats_es::views_repository::NatsViewRepository;

pub async fn cqrs_framework(
    client: Client,
) -> (
    Arc<NatsCqrs<BankAccount>>,
    Arc<NatsViewRepository<BankAccountView, BankAccount>>,
) {
    println!("creating cqrs_framework");
    // A very simple query that writes each event to stdout.
    let simple_query = SimpleLoggingQuery {};

    // A query that stores the current state of an individual account.
    let account_view_repo = Arc::new(NatsViewRepository::new(client.clone(), "views_cqrs_demo"));
    let mut account_query = AccountQuery::new(account_view_repo.clone());

    // Without a query error handler there will be no indication if an
    // error occurs (e.g., database connection failure, missing columns or table).
    // Consider logging an error or panicking in your own application.
    account_query.use_error_handler(Box::new(|e| println!("{}", e)));

    // Create and return an event-sourced `CqrsFramework`.
    let queries: Vec<Box<dyn Query<BankAccount>>> =
        vec![Box::new(simple_query), Box::new(account_query)];
    let repo = NatsEventStore::new(
        client.clone(),
        nats_es::event_repository::NatsEventStoreOptions {
            stream_name: "events_cqrs_demo".to_string(),
            aggregate_types: vec![BankAccount::aggregate_type()],
            replicas: 3,
        },
    )
    .await
    .expect("unable to create nats event store");

    let store = PersistedEventStore::new_event_store(repo);
    println!("created event store");
    let services = BankAccountServices::new(Box::new(HappyPathBankAccountServices));
    let cqrs = CqrsFramework::new(store, queries, services);
    println!("created cqrs");
    (Arc::new(cqrs), account_view_repo)
}
