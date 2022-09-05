use std::env;

use anyhow::Result;
use aws_lambda_events::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use lambda_runtime::{service_fn, LambdaEvent};

mod apigw;
mod store;

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    let config = aws_config::load_from_env().await;
    let table_name = env::var("TABLE_NAME").expect("TABLE_NAME must be set");
    let dynamodb_client = aws_sdk_dynamodb::Client::new(&config);

    let store = store::Store::new(dynamodb_client, table_name);
    let func = service_fn(|event: LambdaEvent<ApiGatewayProxyRequest>| {
        return handler(&store, event);
    });
    lambda_runtime::run(func).await?;

    Ok(())
}

async fn handler(
    store: &impl store::ItemGetter,
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse> {
    let id = match event.payload.path_parameters.get("id") {
        Some(id) => id,
        None => return Ok(apigw::respond(400, String::from("Bad request"))),
    };

    let res = match store.get_item(id.to_string()).await {
        Ok(entry) => entry,
        Err(_) => return Ok(apigw::respond(404, String::from("Not found"))),
    };

    let body = serde_json::to_string(&res)?;
    return Ok(apigw::respond(200, body));
}
