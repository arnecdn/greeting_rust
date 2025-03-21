use std::time::Duration;

use async_trait::async_trait;
use chrono::NaiveDateTime;
use log::info;
use opentelemetry::{ global};
use opentelemetry::propagation::{Injector};
use opentelemetry::trace::{Status, TraceContextExt};
use rdkafka::error::KafkaError;
use rdkafka::producer::{FutureProducer, FutureRecord, Producer};
use rdkafka::ClientConfig;
use rdkafka::message::{ Header, Headers, OwnedHeaders};
use serde::{Deserialize, Serialize};
use serde_json;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use crate::greeting::service::{Greeting, GreetingRepository, ServiceError};
use crate::settings::Kafka;

pub struct KafkaGreetingRepository {
    producer: FutureProducer,
    topic: String,
}
impl KafkaGreetingRepository {
    pub fn new(
        config: Kafka,
        transactional_id: &str
    ) -> Result<Self, ServiceError> {
        let p: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", config.broker.clone())
            .set("message.timeout.ms", config.message_timeout_ms.to_string())
            .set("debug", "all")
            .set("enable.idempotence", config.enable_idempotence.to_string())
            .set("transactional.id", transactional_id)
            .set("message.send.max.retries", "10")
            .create()
            .expect("Producer creation error with invalid configuration");

        p.init_transactions(Duration::from_secs(5))
            .expect("Expected to init transactions");

        Ok(KafkaGreetingRepository {
            producer: p,
            topic: String::from(config.topic.clone()),

        })
    }
}
#[async_trait]
impl GreetingRepository for KafkaGreetingRepository {
    async fn all(&self) -> Result<Vec<Greeting>, ServiceError> {
        panic!("Not implemented")
    }

    async fn store(&mut self, greeting: Greeting) -> Result<(), ServiceError> {
        let msg = GreetingMessage::from(&greeting.clone());
        let x = serde_json::to_string(&msg).unwrap();
        let parent_context = Span::current().context();

        let mut headers = OwnedHeaders::new().insert(Header { key: "greeting_id", value: Some(&msg.id) });

        global::get_text_map_propagator(|propagator| {
            propagator.inject_context(&parent_context, &mut HeaderInjector(&mut headers))
        });

        self.producer
            .begin_transaction()
            .expect("Failed beginning transaction");
        info!("Sending msg id {}", msg.id);

        self.producer
            .send(
                FutureRecord::to(&self.topic).headers(headers).payload(&x).key(&msg.id).partition(-1),
                Duration::from_secs(5),
            )
            .await
            .expect("Failed");

        self.producer
            .commit_transaction(Duration::from_secs(5))
            .expect("Error comiting transaction");
        parent_context.span().set_status(Status::Ok);
        parent_context.span().end();
        Ok(())
    }
}
impl From<KafkaError> for ServiceError {
    fn from(_error: KafkaError) -> Self {
        ServiceError::RepoError(_error.to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GreetingMessage {
    id: String,
    to: String,
    from: String,
    heading: String,
    message: String,
    created: NaiveDateTime,
}

impl From<&Greeting> for GreetingMessage {
    fn from(greeting: &Greeting) -> Self {
        GreetingMessage {
            id: greeting.id.to_string(),
            to: greeting.to.to_string(),
            from: greeting.from.to_string(),
            heading: greeting.heading.to_string(),
            message: greeting.message.to_string(),
            created: *&greeting.created,
        }
    }
}

pub struct HeaderInjector<'a>(pub &'a mut OwnedHeaders);

impl <'a>Injector for HeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        let mut new = OwnedHeaders::new().insert(rdkafka::message::Header {
            key,
            value: Some(&value),
        });

        for header in self.0.iter() {
            let s = String::from_utf8(header.value.unwrap().to_vec()).unwrap();
            new = new.insert(rdkafka::message::Header { key: header.key, value: Some(&s) });
        }

        self.0.clone_from(&new);
    }
}

