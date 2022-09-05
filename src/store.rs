use async_trait::async_trait;
use mockall::predicate::*;
use mockall::*;
use serde::{Deserialize, Serialize};
use serde_dynamo::{from_item, to_item};

use anyhow::{anyhow, Ok, Result};
use aws_sdk_dynamodb::model::AttributeValue;

#[automock]
#[async_trait]
pub trait ItemSaver {
    async fn save_item(&self, id: String) -> Result<()>;
}

#[automock]
#[async_trait]
pub trait ItemGetter {
    async fn get_item(&self, id: String) -> Result<Entry>;
}

#[derive(Clone)]
pub struct Store {
    db: aws_sdk_dynamodb::Client,
    table_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct Entry {
    id: String,
}

impl Store {
    pub fn new(db: aws_sdk_dynamodb::Client, table_name: String) -> Self {
        return Self { db, table_name };
    }
}

#[async_trait]
impl ItemSaver for Store {
    async fn save_item(&self, id: String) -> Result<()> {
        let entry = Entry { id };
        let item = to_item(entry)?;

        self.db
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await?;

        return Ok(());
    }
}

#[async_trait]
impl ItemGetter for Store {
    async fn get_item(&self, id: String) -> Result<Entry> {
        let result = self
            .db
            .get_item()
            .table_name(&self.table_name)
            .key("id", AttributeValue::S(id))
            .send()
            .await?;

        match result.item {
            Some(item) => return Ok(from_item(item)?),
            None => return Err(anyhow!("Fail")),
        }
    }
}
