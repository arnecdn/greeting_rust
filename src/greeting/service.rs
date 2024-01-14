use std::error::Error;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::greeting::repository::GreetingEntity;

enum ServiceError{
    UnrecognizedGreetingError,
}

trait GreetingService {
    fn receive_greeting(&self, greeting: Greeting) -> Result<Greeting, ServiceError>;
    fn all_greetings(&self) -> Result<Vec<Greeting>, ServiceError>;

}



#[derive( Clone)]
pub struct Greeting{
    pub(crate) to: String,
    pub(crate) from: String,
    pub(crate) heading: String,
    pub(crate) message: String,
    pub(crate) created: DateTime<Utc>,

}

impl Greeting {

    pub fn from(greeting: GreetingEntity) -> Greeting {Greeting {
        to: greeting.to,
        from: greeting.from,
        heading: greeting.heading,
        message: greeting.message,
        created: greeting.created,
    }
}
}

impl GreetingService for Greeting {
    fn receive_greeting(&self, greeting: Greeting) -> Result<Greeting, ServiceError> {
        Ok(greeting)
    }

    fn all_greetings(&self) -> Result<Vec<Greeting>, ServiceError> {
        Ok(vec![self.clone()])
    }
}