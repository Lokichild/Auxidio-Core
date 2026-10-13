use auxidio_harness::verifier::{PythonVerifier, VerifierClient};
use harness_core::VerifyStatus;

fn service_path() -> String {
    format!(
        "{}/../../python/math_verifier/math_verifier.py",
        env!("CARGO_MANIFEST_DIR")
    )
}

#[test]
fn service_verifies_correct_arithmetic() {
    let verifier = PythonVerifier::spawn("python3", &service_path()).unwrap();
    let result = verifier
        .verify("The total is 3 * 14.99 + 6.50 = 51.47", "print(round(3 * 14.99 + 6.50, 2))")
        .unwrap();
    assert_eq!(result.status, VerifyStatus::Verified);
    assert_eq!(result.value, Some(51.47));
}

#[test]
fn service_flags_mismatched_claim() {
    let verifier = PythonVerifier::spawn("python3", &service_path()).unwrap();
    let result = verifier
        .verify("The total is 49.99", "print(round(3 * 14.99 + 6.50, 2))")
        .unwrap();
    assert_eq!(result.status, VerifyStatus::Mismatch);
    assert_eq!(result.value, Some(51.47));
}

#[test]
fn service_reports_sandbox_failures() {
    let verifier = PythonVerifier::spawn("python3", &service_path()).unwrap();

    let syntax = verifier.verify("claim", "print(1 +").unwrap();
    assert_eq!(syntax.status, VerifyStatus::SyntaxError);

    let runtime = verifier.verify("claim", "print(1 / 0)").unwrap();
    assert_eq!(runtime.status, VerifyStatus::RuntimeError);

    let non_numeric = verifier.verify("claim", "print('hello')").unwrap();
    assert_eq!(non_numeric.status, VerifyStatus::NonNumeric);

    let blocked = verifier.verify("claim", "import os\nprint(1)").unwrap();
    assert_eq!(blocked.status, VerifyStatus::RuntimeError);

    let blocked_open = verifier.verify("claim", "open('/etc/passwd').read()").unwrap();
    assert_eq!(blocked_open.status, VerifyStatus::RuntimeError);
}

#[test]
fn service_allows_allowlisted_imports() {
    let verifier = PythonVerifier::spawn("python3", &service_path()).unwrap();
    let result = verifier
        .verify("sqrt of 2 is about 1.41421356", "import math\nprint(math.sqrt(2))")
        .unwrap();
    assert_eq!(result.status, VerifyStatus::Verified);
}
