use std::{collections::HashMap, env};

use anyhow::Result;
use aws_lambda_events::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};

use http::{HeaderMap, HeaderValue};
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

// /// Unit tests
// ///
// /// These tests are run using the `cargo test` command.
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use aws_sdk_dynamodb::{Client, Config, Credentials, Region};
//     use aws_smithy_client::{erase::DynConnector, test_connection::TestConnection};
//     use aws_smithy_http::body::SdkBody;
//     use std::collections::HashMap;

//     // Helper function to create a mock AWS configuration
//     async fn get_mock_config() -> Config {
//         let cfg = aws_config::from_env()
//             .region(Region::new("eu-west-1"))
//             .credentials_provider(Credentials::new(
//                 "access_key",
//                 "privatekey",
//                 None,
//                 None,
//                 "dummy",
//             ))
//             .load()
//             .await;

//         Config::new(&cfg)
//     }

//     /// Helper function to generate a sample DynamoDB request
//     fn get_request_builder() -> http::request::Builder {
//         http::Request::builder()
//             .header("content-type", "application/x-amz-json-1.0")
//             .uri(http::uri::Uri::from_static(
//                 "https://dynamodb.eu-west-1.amazonaws.com/",
//             ))
//     }

//     #[tokio::test]
//     async fn test_put_item() {
//         // Mock DynamoDB client
//         //
//         // `TestConnection` takes a vector of requests and responses, allowing us to
//         // simulate the behaviour of the DynamoDB API endpoint. Since we are only
//         // making a single request in this test, we only need to provide a single
//         // entry in the vector.
//         let conn = TestConnection::new(vec![(
//             get_request_builder()
//                 .header("x-amz-target", "DynamoDB_20120810.PutItem")
//                 .body(SdkBody::from(
//                     r#"{"TableName":"test","Item":{"id":{"S":"1"},"payload":{"S":"test1"}}}"#,
//                 ))
//                 .unwrap(),
//             http::Response::builder()
//                 .status(200)
//                 .body(SdkBody::from(
//                     r#"{"Attributes": {"id": {"S": "1"}, "payload": {"S": "test1"}}}"#,
//                 ))
//                 .unwrap(),
//         )]);
//         let client =
//             Client::from_conf_conn(get_mock_config().await, DynConnector::new(conn.clone()));

//         let table_name = "test_table";

//         // Mock API Gateway request
//         let mut path_parameters = HashMap::new();
//         path_parameters.insert("id".to_string(), vec!["1".to_string()]);

//         let request = http::Request::builder()
//             .method("PUT")
//             .uri("/1")
//             .body(Body::Text("test1".to_string()))
//             .unwrap()
//             .with_path_parameters(path_parameters);

//         // Send mock request to Lambda handler function
//         let response = put_item(&client, table_name, request)
//             .await
//             .unwrap()
//             .into_response();

//         // Assert that the response is correct
//         assert_eq!(response.status(), 200);
//         assert_eq!(response.body(), &Body::Text("item saved".to_string()));
//     }
// }
