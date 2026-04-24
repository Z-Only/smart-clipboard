//! Scenario 6: Template commands honour the security guard (both locked rejection
//! and unlocked access).

use serde_json::json;

use super::harness::{
    invoke, invoke_raw, lock_serial, unlock_with_password, TestDir, TestHarness, TestKeyringGuard,
};

#[test]
fn template_crud_rejected_while_locked() {
    let _serial = lock_serial();
    let _keyring = TestKeyringGuard::new();
    let dir = TestDir::new();
    let harness = TestHarness::new(&dir.path);

    harness.lock_now();

    let template_cmds: Vec<(&str, serde_json::Value)> = vec![
        (
            "create_template",
            json!({"name": "T", "content": "Hi {{n}}", "category": "general"}),
        ),
        ("get_templates", json!({"category": null})),
        ("get_template", json!({"id": 999_999})),
        ("get_template_categories", json!({})),
    ];

    let mut failures = Vec::new();
    for (cmd, body) in template_cmds {
        if invoke_raw(&harness, cmd, body).is_ok() {
            failures.push(cmd.to_string());
        }
    }

    assert!(
        failures.is_empty(),
        "Template commands that should reject while locked but succeeded: {:?}",
        failures
    );
}

#[test]
fn template_create_and_list_succeed_after_unlock() {
    let _serial = lock_serial();
    let _keyring = TestKeyringGuard::new();
    let dir = TestDir::new();
    let harness = TestHarness::new(&dir.path);

    harness.lock_now();
    unlock_with_password(&harness);

    // Create a template.
    let created: serde_json::Value = invoke(
        &harness,
        "create_template",
        json!({"name": "Greeting", "content": "Hello {{name}}", "category": "general"}),
    )
    .expect("create_template must succeed after unlock");

    assert!(
        created.get("id").is_some(),
        "created template must have an id"
    );

    // List templates.
    let list: Vec<serde_json::Value> = invoke(&harness, "get_templates", json!({"category": null}))
        .expect("get_templates must succeed after unlock");

    assert!(
        !list.is_empty(),
        "template list must contain at least one template"
    );
}
