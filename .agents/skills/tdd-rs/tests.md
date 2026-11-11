# Good and Bad Tests

## Good Tests

**Integration-style**: Test through real interfaces, not mocks of internal parts.

```rust
// GOOD: Tests observable behavior
#[test]
fn user_can_checkout_with_valid_cart() {
    let mut cart = Cart::new();
    cart.add(product());

    let receipt = checkout(&cart, &payment_method()).unwrap();

    assert_eq!(receipt.status, Status::Confirmed);
}
```

Characteristics:

- Tests behavior users/callers care about
- Uses the `pub` API only
- Survives internal refactors
- Describes WHAT, not HOW
- One logical assertion per test

A test in the crate's `tests/` directory compiles as a separate crate and sees only `pub` items, so the compiler holds it to the interface. A `#[cfg(test)] mod tests` inside a source file reaches private items through `use super::*`; hold it to the `pub` API by discipline.

## Bad Tests

**Implementation-detail tests**: Coupled to internal structure.

```rust
// BAD: Tests implementation details
#[test]
fn checkout_calls_payment_service_process() {
    let mut payments = MockPaymentService::new();
    payments
        .expect_process()
        .with(eq(cart().total()))
        .times(1)
        .returning(|_| Ok(()));

    checkout(&cart(), &payments).unwrap();
}
```

Red flags:

- Mocking internal collaborators
- Testing private functions
- Widening visibility (`pub(crate)`, `#[cfg(test)] pub`) so a test can reach inside
- Asserting on call counts/order
- Test breaks when refactoring without behavior change
- Test name describes HOW not WHAT
- Verifying through external means instead of interface

```rust
// BAD: Bypasses interface to verify
#[test]
fn create_user_saves_to_database() {
    let app = test_app();
    app.create_user("Alice").unwrap();

    let count: i64 = app
        .db
        .query_row("SELECT COUNT(*) FROM users WHERE name = ?1", ["Alice"], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

// GOOD: Verifies through interface
#[test]
fn create_user_makes_user_retrievable() {
    let app = test_app();
    let id = app.create_user("Alice").unwrap();

    let retrieved = app.get_user(id).unwrap();

    assert_eq!(retrieved.name, "Alice");
}
```

**Tautological tests**: Expected value restates the implementation, so the test passes by construction.

```rust
// BAD: Expected value is recomputed the way the code computes it
#[test]
fn calculate_total_sums_line_items() {
    let items = [LineItem { price: 10 }, LineItem { price: 5 }];
    let expected: u32 = items.iter().map(|item| item.price).sum();
    assert_eq!(calculate_total(&items), expected);
}

// GOOD: Expected value is an independent, known literal
#[test]
fn calculate_total_sums_line_items() {
    let items = [LineItem { price: 10 }, LineItem { price: 5 }];
    assert_eq!(calculate_total(&items), 15);
}
```
