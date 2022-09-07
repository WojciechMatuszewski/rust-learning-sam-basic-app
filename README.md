# Rust AWS Lambda

- There are two libraries for the runtime

  - The `aws_lambda_http`. This one seem to be tailored for HTTP-based Lambdas
  - The `aws_lambda_runtime`. This one seem to be tailored for general purpose Lambdas

- Writing lambda functions seem to follow a similar convention to the one present in Go

  - But the composition model is different. **Creating in-scope closures seem to be hard/impossible**.

    ```rust
    fn newHandler() -> Handler {
      return fn handler() {} // Not a valid syntax
    }
    ```

  - It seems like for now, one has to provide all the arguments and not use higher order functions

  - Like in Go case, you probably want to build some kind of abstraction for responses. It gets tedious having to populate all struct fields all the time

  - Implementing the middleware pattern must be pretty hard in Rust due to the suboptimal (in my opinion) support for closures.

  - There is a package to use to serialize / deserialize DynamoDB structures, but I find the experience lacking, mainly due to "features" in cargo.

- Once you get going, it's relatively easy to continue working. You do not have to know much of the language to write HTTP handlers.

- **`cargo fix`** is very useful and something I miss dearly from TypeScript ecosystem – the "clean imports" task in VSCode.

- I like the pattern of having the tests in the same file as the code. It makes it easy to read the tests, but it makes it hard to separate unit tests from the integration and end-to-end ones.

  - One could add compilation attributes for ignoring the tests.

    ```rust
    #[cfg(test)]
    mod tests {
        #[test]
        #[cfg_attr(not(feature = "unit_tests"), ignore)]
        fn something() {
            println!("it works")
        }
    }
    ```

    To run the `something` test, you would use `cargo test --features unit_tests` command.

- The **notion of an `interface` works a bit differently** in Rust than in Go. The biggest change is that **you have to explicitly define an interface implementation in `Rust`**. That is not the case in Go.

  - In Go, if a function takes an interface as a parameter, all your struct has to do is to fullfil that interface, no need to couple the interface declaration and the implementation together.

- Mocking is done through macros and not code generation like in the case of Go.

- The Rust built-in test framework does not expose any hooks like `setup` or `init`. **This makes it hard to setup environment variables before running the test** (think DynamoDB table names and such).

  - Others suggested writing a macro, using the `sync::Once` or using some kind of testing framework, for example the [stainless](https://github.com/reem/stainless) one.

  - I think I like the approach of `sync::Once` the most as it is the most straightforward one.

    ```rust
    static INIT: Once = Once::new();
    fn initialize() {
        INIT.call_once(|| env::set_var("TABLE_NAME", "sam-rust-app-Table-H3J30UUTALAN"))
    }

    #[test]
    fn do_something() -> Result<()> {
      initialize();

      // ...

      return Ok(());
    }
    ```

- It is very frustrating that the AWS SAM does not have the `--stack-outputs` option like the CDK.

  - You have to get the stack outputs manually, via the AWS CLI.

    ```bash
    aws cloudformation describe-stacks --stack-name YOUR_STACK_NAME --query "Stacks[0].Outputs"
    ```

- Creating the test events for the handlers is not that bad. Using `serde_json` and parsing the "raw strings" seem to get the job done.
