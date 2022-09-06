CARGO_LAMBDA_FLAGS = ''

.PHONY: build deploy

build:
	cargo lambda build --release

deploy: build
	sam deploy
	aws cloudformation describe-stacks --stack-name sam-rust-app --query "Stacks[0].Outputs" > outputs.json


