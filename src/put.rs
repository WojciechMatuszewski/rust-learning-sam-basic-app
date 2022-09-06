use std::{collections::HashMap, env};

use anyhow::Result;
use aws_lambda_events::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};

use http::{HeaderMap, HeaderValue};
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

    #[tokio::test]
    #[cfg_attr(not(feature = "unit_tests"), ignore)]
    async fn test_invalid_path_parameters() -> Result<()> {
        let store = store::MockItemSaver::new();

        let payload = create_event(HashMap::new())?;
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

        let payload = create_event(HashMap::from([(String::from("id"), String::from("123"))]))?;
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

        let payload = create_event(HashMap::from([(String::from("id"), String::from("123"))]))?;
        let context = create_context()?;

        let result = lambda_handler(LambdaEvent { payload, context }).await?;
        assert_eq!(result.status_code, 200);
        assert_eq!(
            result.body.unwrap(),
            aws_lambda_events::encodings::Body::Text(String::from("All good!"))
        );

        return Ok(());
    }
}

static REQUEST: &str = r#"{
    "body": "{}",
    "headers":null,
    "httpMethod":"PUT",
    "isBase64Encoded":false,
    "multiValueHeaders":null,
    "multiValueQueryStringParameters":null,
    "path":"/myPath",
    "pathParameters": null,
    "queryStringParameters":null,
    "requestContext":{
       "accountId":"xxxxx",
       "apiId":"xxxxx",
       "domainName":"testPrefix.testDomainName",
       "domainPrefix":"testPrefix",
       "extendedRequestId":"NvWWKEZbliAFliA=",
       "httpMethod":"POST",
       "identity":{},
       "path":"/myPath",
       "protocol":"HTTP/1.1",
       "requestId":"e5488776-afe4-4e5e-92b1-37bd23f234d6",
       "requestTime":"18/Feb/2022:13:23:12 +0000",
       "requestTimeEpoch":1645190592806,
       "resourceId":"ddw8yd",
       "resourcePath":"/myPath",
       "stage":"test-invoke-stage"
    },
    "resource":"/myPath",
    "stageVariables":null
}"#;

fn create_event(path_parameters: HashMap<String, String>) -> Result<ApiGatewayProxyRequest> {
    let mut base: ApiGatewayProxyRequest = serde_json::from_str(REQUEST)?;
    base.path_parameters = path_parameters;

    return Ok(base);
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
