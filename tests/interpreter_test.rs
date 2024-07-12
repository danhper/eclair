use eclair::interpreter::{self, Config, Env, Type, Value};

#[tokio::test]
async fn test_binops() {
    let mut env = _create_env();

    _check_result(&mut env, "1 + 8", Value::from(9u64)).await;
    _check_result(&mut env, "int256(1) - 8", Value::from(-7)).await;
    _check_result(&mut env, "3 + 8 * 4", Value::from(35u64)).await;
    _check_result(&mut env, "(10 + 4) % 3", Value::from(2u64)).await;
}

#[tokio::test]
async fn test_string() {
    let mut env = _create_env();

    _check_result(&mut env, "\"foo\"", Value::from("foo")).await;
    _check_result(&mut env, "\"foo\".length", Value::from(3u64)).await;
    _check_result(&mut env, "\"foo\".concat(\"bar\")", Value::from("foobar")).await;
}

#[tokio::test]
async fn test_builtin_type() {
    let mut env = _create_env();

    _check_result(&mut env, "type(1)", Value::TypeObject(Type::Uint(256))).await;
    _check_result(
        &mut env,
        "type(uint256)",
        Value::TypeObject(Type::Type(Box::new(Type::Uint(256)))),
    )
    .await;
}

#[tokio::test]
async fn test_builtin_format() {
    let mut env = _create_env();

    _check_result(&mut env, "2e18.format()", Value::from("2.00")).await;
    _check_result(&mut env, "3.5678e7.format(6)", Value::from("35.68")).await;
    _check_result(&mut env, "\"foo\".format()", Value::from("foo")).await;
}

#[tokio::test]
async fn test_defined_functions() {
    let mut env = _create_env();

    _execute(&mut env, "function add(a, b) { return a + b; }").await;
    _check_result(&mut env, "add(1, 2)", Value::from(3u64)).await;

    _execute(
        &mut env,
        "function cond(pred, yes, no) { if (pred) return yes; else return no; }",
    )
    .await;
    _check_result(&mut env, "cond(true, 1, 2)", Value::from(1u64)).await;
    _check_result(&mut env, "cond(false, 1, 2)", Value::from(2u64)).await;
}

#[tokio::test]
async fn test_for_loop() {
    let mut env = _create_env();

    let res = _execute(
        &mut env,
        r#"
        a = 1;
        for (i = 1; i <= 5; i++) {
            a *= i;
        }
        a;
    "#,
    )
    .await;
    assert_eq!(res, Some(Value::from(120u64)));

    let res = _execute(
        &mut env,
        r#"
        a = 1;
        for (i = 1; i <= 5; i++) {
            if (a > 10) break;
            a *= i;
        }
        a
    "#,
    )
    .await;
    assert_eq!(res, Some(Value::from(24u64)));

    let res = _execute(
        &mut env,
        r#"
        a = 1;
        for (i = 1; i <= 5; i++) {
            if (i % 2 == 0) continue;
            a *= i;
        }
        a
    "#,
    )
    .await;
    assert_eq!(res, Some(Value::from(15u64)));
}

async fn _execute(env: &mut Env, code: &str) -> Option<Value> {
    interpreter::evaluate_code(env, code).await.unwrap()
}

async fn _check_result(env: &mut Env, code: &str, expected: Value) {
    let res = _execute(env, code).await;
    assert_eq!(res, Some(expected));
}

fn _create_env() -> Env {
    let foundry_conf = foundry_config::load_config();
    let config = Config::new(None, false, foundry_conf);
    let mut env = Env::new(config);
    interpreter::load_builtins(&mut env);
    env
}
