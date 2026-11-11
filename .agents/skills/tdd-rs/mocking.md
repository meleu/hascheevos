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

At system boundaries, design interfaces that are easy to mock:

**1. Use dependency injection**

Put the boundary behind a `trait` and pass it in rather than constructing it internally:

```rust
// Easy to mock: the caller chooses the PaymentClient
fn process_payment(order: &Order, payments: &impl PaymentClient) -> Result<Receipt, PaymentError> {
    payments.charge(order.total)
}

// Hard to mock: the function builds its own client
fn process_payment(order: &Order) -> Result<Receipt, PaymentError> {
    let client = StripeClient::from_env();
    client.charge(order.total)
}
```

The test adapter is a hand-written fake implementing the trait; reach for a mocking crate only when the project already uses one:

```rust
struct FakePayments {
    outcome: Result<Receipt, PaymentError>,
}

impl PaymentClient for FakePayments {
    fn charge(&self, _amount: Money) -> Result<Receipt, PaymentError> {
        self.outcome.clone()
    }
}
```

Time and randomness are boundaries too: take the current time or an RNG as a parameter instead of calling `SystemTime::now()` or a thread-local RNG inside the logic.

**2. Prefer SDK-style interfaces over generic fetchers**

Give each external operation its own trait method instead of one generic method with conditional logic:

```rust
// GOOD: Each method is independently mockable
trait Api {
    fn get_user(&self, id: UserId) -> Result<User, ApiError>;
    fn get_orders(&self, user_id: UserId) -> Result<Vec<Order>, ApiError>;
    fn create_order(&self, order: &NewOrder) -> Result<Order, ApiError>;
}

// BAD: Mocking requires conditional logic inside the mock
trait Api {
    fn request(&self, method: Method, endpoint: &str, body: Option<&[u8]>) -> Result<Response, ApiError>;
}
```

The SDK approach means:
- Each mock returns one specific type
- No conditional logic in test setup
- Easier to see which endpoints a test exercises
- Type safety per endpoint
