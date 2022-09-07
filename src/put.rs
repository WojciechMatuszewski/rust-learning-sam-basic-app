use std::env;

use anyhow::Result;
use aws_lambda_events::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};

use lambda_runtime::{service_fn, LambdaEvent};

mod apigw;
mod environment;
mod store;

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    let func = service_fn(lambda_handler);
    lambda_runtime::run(func).await?;

    Ok(())
}

async fn lambda_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse> {
    let config = aws_config::load_from_env().await;

    let table_name = env::var("TABLE_NAME").expect("TABLE_NAME must be set");
    let dynamodb_client = aws_sdk_dynamodb::Client::new(&config);

    let store = store::Store::new(dynamodb_client, table_name);

    return handler(&store, event).await;
}

async fn handler(
    store: &impl store::ItemSaver,
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> anyhow::Result<ApiGatewayProxyResponse> {
    let id = match event.payload.path_parameters.get("id") {
        Some(id) => id,
        None => {
            return Ok(apigw::respond(400, String::from("Bad request")));
        }
    };

    println!("Putting stuff into DynamoDB");

    let res = store.save_item(id.to_owned()).await;
    match res {
        Ok(_) => {
            return Ok(apigw::respond(200, String::from("All good!")));
        }
        Err(error) => {
            return Ok(apigw::respond(500, error.to_string()));
        }
    };
}

#[cfg(test)]
mod tests {
    use std::sync::Once;

    use super::*;
    use anyhow::anyhow;
    use http::{HeaderMap, HeaderValue};

    #[tokio::test]
    #[cfg_attr(not(feature = "unit_tests"), ignore)]
    async fn test_invalid_path_parameters() -> Result<()> {
        let store = store::MockItemSaver::new();

        let raw_payload = r#"{"httpMethod": "PUT"}"#;
        let payload: ApiGatewayProxyRequest = serde_json::from_str(&raw_payload)?;

        let context = create_context()?;

        let payload = LambdaEvent { payload, context };
        let result = handler(&store, payload).await?;

        assert_eq!(result.status_code, 400);
        return Ok(());
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "unit_tests"), ignore)]
    async fn test_fail_save() -> Result<()> {
        let mut store = store::MockItemSaver::new();

        store
            .expect_save_item()
            .times(1)
            .return_once(|_| return Err(anyhow!("Error!")));

        let raw_payload = r#"{"pathParameters": {"id": "123"}, "httpMethod": "PUT"}"#;
        let payload: ApiGatewayProxyRequest = serde_json::from_str(&raw_payload)?;
        let context = create_context()?;

        let payload = LambdaEvent { payload, context };
        let result = handler(&store, payload).await?;

        assert_eq!(result.status_code, 500);

        return Ok(());
    }

    static INIT: Once = Once::new();
    fn initialize() -> Result<()> {
        let outputs = environment::get_stack_outputs()?;

        INIT.call_once(|| env::set_var("TABLE_NAME", outputs.table_name));
        return Ok(());
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration_tests"), ignore)]
    async fn test_success_save() -> Result<()> {
        match initialize() {
            Ok(_) => {}
            Err(error) => {
                println!("error: {:?}", error)
            }
        }
        let raw_payload = r#"{"pathParameters": {"id": "123"}, "httpMethod": "PUT"}"#;
        let payload: ApiGatewayProxyRequest = serde_json::from_str(&raw_payload)?;

        let context = create_context()?;

        let result = lambda_handler(LambdaEvent { payload, context }).await?;
        assert_eq!(result.status_code, 200);
        assert_eq!(
            result.body.unwrap(),
            aws_lambda_events::encodings::Body::Text(String::from("All good!"))
        );

        return Ok(());
    }

    fn create_context() -> Result<lambda_runtime::Context> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "lambda-runtime-aws-request-id",
            HeaderValue::from_static("my-id"),
        );
        headers.insert(
            "lambda-runtime-deadline-ms",
            HeaderValue::from_static("123"),
        );
        let context = lambda_runtime::Context::try_from(headers).unwrap();
        return Ok(context);
    }
}
