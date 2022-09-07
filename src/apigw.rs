use aws_lambda_events::{apigw::ApiGatewayProxyResponse, encodings::Body};

use http::HeaderMap;

pub fn respond(status_code: i64, body: String) -> ApiGatewayProxyResponse {
    let response = ApiGatewayProxyResponse {
        status_code: status_code,
        headers: HeaderMap::new(),
        multi_value_headers: HeaderMap::new(),
        body: Some(Body::Text(body)),
        is_base64_encoded: Some(false),
    };

    return response;
}
