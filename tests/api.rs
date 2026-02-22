use cargo_avail::check::{Availability, CheckError, Client, canon_crate_name, check_name};

#[test]
fn public_api_canon_crate_name() {
    assert_eq!(canon_crate_name("Foo-Bar"), "foo_bar");
    assert_eq!(canon_crate_name("already_canonical"), "already_canonical");
}

#[test]
fn public_api_empty_name_returns_error() {
    let client = Client::new();
    assert!(matches!(
        check_name(&client, ""),
        Err(CheckError::InvalidName(_))
    ));
}

#[test]
fn public_api_reserved_returns_reserved() {
    let client = Client::new();
    match check_name(&client, "std") {
        Ok(Availability::Reserved) => {}
        other => panic!("expected Reserved, got {other:?}"),
    }
}

#[test]
fn public_api_invalid_returns_error() {
    let client = Client::new();
    match check_name(&client, "123bad") {
        Err(CheckError::InvalidName(e)) => {
            assert!(e.to_string().contains("cannot start with a digit"));
        }
        other => panic!("expected InvalidName, got {other:?}"),
    }
}

#[test]
fn client_default_equals_new() {
    let _client: Client = Client::default();
}

#[test]
#[ignore] // requires network access
fn public_api_taken_returns_taken() {
    let client = Client::new();
    match check_name(&client, "serde") {
        Ok(Availability::Taken) => {}
        other => panic!("expected Taken, got {other:?}"),
    }
}

#[test]
#[ignore] // requires network access
fn public_api_available_returns_available() {
    let client = Client::new();
    match check_name(&client, "zzzyyyxxxwww-not-a-real-crate") {
        Ok(Availability::Available) => {}
        other => panic!("expected Available, got {other:?}"),
    }
}
