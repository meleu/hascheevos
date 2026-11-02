# When to Mock

Mock at **system boundaries** only:

- External APIs (payment, email, etc.)
- Databases (sometimes - prefer test DB)
- Time/randomness
- File system (sometimes)

Don't mock:

- Your own types/modules
- Internal collaborators
- Anything you control

## Designing for Mockability

At system boundaries, design traits that are easy to mock:

**1. Use dependency injection**

Pass external dependencies in behind a trait rather than creating them internally:

```rust
// Easy to mock
fn process_payment(order: &Order, payments: &impl PaymentClient) -> Result<Receipt, PaymentError> {
    payments.charge(order.total)
}

// Hard to mock
fn process_payment(order: &Order) -> Result<Receipt, PaymentError> {
    let client = StripeClient::new(env::var("STRIPE_KEY")?);
    client.charge(order.total)
}
```

**2. Prefer SDK-style traits over generic fetchers**

Create a specific method for each external operation instead of one generic method with conditional logic:

```rust
// GOOD: Each method is independently mockable
trait Api {
    fn get_user(&self, id: UserId) -> Result<User, ApiError>;
    fn get_orders(&self, user_id: UserId) -> Result<Vec<Order>, ApiError>;
    fn create_order(&self, data: &NewOrder) -> Result<Order, ApiError>;
}

// BAD: Mocking requires conditional logic inside the mock
trait Api {
    fn fetch(&self, endpoint: &str, options: RequestOptions) -> Result<Response, ApiError>;
}
```

The SDK approach means:
- Each mock returns one specific type
- No conditional logic in test setup
- Easier to see which endpoints a test exercises
- Type safety per endpoint
