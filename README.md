# go-again

This project is a Rust CLI meant as a go development helper.
When working with a big go codebase with a lot of tests one often runs all tests to understand where things are broken.
The developer then takes the failing test cases (or the test cases that had a panic) and runs them individually or just all failing ones.
When the developer isolated and fixed the failing test case, he first wants to run the previously failing tests to see if they now pass.
Only then one would want to run all tests again.

`go-again` helps with this workflow.
It hooks into the process of running tests and storing the last failed tests in a file per project & branch with the `go-again remember` subcommand. The developer just needs to pipe the go test command output into `go-again remember`. For example running all tests and remembering the failed ones would look like this:

```sh
go test ./... | go-again remember
```

When the developer wants to re-run the previously failed tests, he can use go-again again with the `go-again run` command. For example:

```sh
# Assuming the following tests failed previously:
# ./internal/logic TestCalculateSum
# ./api/handler TestHandleRequest
# This command will re-run only those tests
go-again run
```

The developer can also use `go-again select` to select which of the previously failed tests to run again.
This will run an fzf style selector in the terminal to pick the tests to re-run.

```sh
go-again select
```

If the developer just wants to see which tests failed previously, he can use `go-again list`:

```sh
go-again list

# This will output something like:
#$> ./internal/logic TestCalculateSum
#$> ./api/handler TestHandleRequest
```
