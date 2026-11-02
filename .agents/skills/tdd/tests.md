# Good and Bad Tests

## Good Tests

**Integration-style**: Test through real interfaces, not mocks of internal parts.

```rust
// GOOD: Tests observable behavior
#[test]
fn user_can_checkout_with_valid_cart() {
    let mut cart = Cart::new();
    cart.add(product);
    let result = checkout(&cart, &payment_method).unwrap();
    assert_eq!(result.status, Status::Confirmed);
}
```

Characteristics:

- Tests behavior users/callers care about
- Uses public API only
- Survives internal refactors
- Describes WHAT, not HOW
- One logical assertion per test

## Bad Tests

**Implementation-detail tests**: Coupled to internal structure.

```rust
// BAD: Tests implementation details
#[test]
fn checkout_calls_payment_service_process() {
    let mut mock_payment = MockPaymentService::new();
    mock_payment
        .expect_process()
        .with(eq(cart.total))
        .times(1)
        .returning(|_| Ok(()));
    checkout(&cart, &mock_payment).unwrap();
}
```

Red flags:

- Mocking internal collaborators
- Testing private functions
- Asserting on call counts/order
- Test breaks when refactoring without behavior change
- Test name describes HOW not WHAT
- Verifying through external means instead of interface

```rust
// BAD: Bypasses interface to verify
#[test]
fn create_user_saves_to_database() {
    create_user(NewUser { name: "Alice" }).unwrap();
    let row = db.query_opt("SELECT * FROM users WHERE name = ?", ["Alice"]).unwrap();
    assert!(row.is_some());
}

// GOOD: Verifies through interface
#[test]
fn create_user_makes_user_retrievable() {
    let user = create_user(NewUser { name: "Alice" }).unwrap();
    let retrieved = get_user(user.id).unwrap();
    assert_eq!(retrieved.name, "Alice");
}
```

**Tautological tests**: Expected value restates the implementation, so the test passes by construction.

```rust
// BAD: Expected value is recomputed the way the code computes it
#[test]
fn calculate_total_sums_line_items() {
    let items = [Item { price: 10 }, Item { price: 5 }];
    let expected: u32 = items.iter().map(|i| i.price).sum();
    assert_eq!(calculate_total(&items), expected);
}

// GOOD: Expected value is an independent, known literal
#[test]
fn calculate_total_sums_line_items() {
    assert_eq!(calculate_total(&[Item { price: 10 }, Item { price: 5 }]), 15);
}
```
