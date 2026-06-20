# Request Body Validation Implementation

## Architecture

All request body validation is centralized through the `ValidJson<T>` extractor in [src/utils/validators.rs](src/utils/validators.rs):

1. **Request deserialization** - `axum` deserializes JSON into type `T`
2. **Validation** - `T::validate()` is called automatically (via `validator` crate's `#[derive(Validate)]` macro)
3. **Error response** - If validation fails, returns `400 Bad Request` with field-level error details
4. **Handler entry** - If validation passes, the validated struct is passed to the handler

## Custom Validators

All defined in `src/utils/validators.rs`:

- **`validate_stellar_address()`** - Ensures 56-char G-prefixed format for blockchain addresses (Scenario E prevention)
- **`validate_future_timestamp()`** - Ensures timestamps > now with 300-sec grace period (Scenario C prevention)

## Validation Rules by Model

| Model | Field | Constraint | Scenario |
|-------|-------|-----------|----------|
| **CreateProduct** | `product_name` | 1-255 chars | Scenario D |
| | `product_price` | >= 1 | **Scenario A** ✓ |
| | `product_description` | max 5000 chars | Scenario D |
| **CreateInvestment** | `name` | 1-255 chars | Scenario D |
| | `min_amount` | >= 1 | Positive amounts |
| | `end_date` | `validate_future_timestamp()` | **Scenario C** ✓ |
| **CreateProposal** | `title` | 1-500 chars | Scenario D |
| | `required_votes` | >= 1 | **Scenario B** ✓ |
| | `ends_at` | `validate_future_timestamp()` | **Scenario C** ✓ |
| **CreateDispute** | `arbitrator` | Stellar address | **Scenario E** ✓ |
| **CreateEscrow** | `farmer` | Stellar address | **Scenario E** ✓ |

## Unit Tests

All critical validations have unit tests in the model files:

- [src/models/product.rs](src/models/product.rs) - Tests Scenario A (negative prices)
- [src/models/proposal.rs](src/models/proposal.rs) - Tests Scenario B (zero votes) & C (past dates)
- [src/models/investment.rs](src/models/investment.rs) - Tests Scenario C (expired rounds)

Run tests:
```bash
cargo test --lib models::
```

## Error Response Format

When validation fails, the API returns:

```json
{
  "error": {
    "code": "validation_error",
    "message": "Request body validation failed",
    "details": {
      "product_price": ["must be at least 1"],
      "product_name": ["length validation failed"]
    }
  }
}
```

## Integration

All route handlers use `ValidJson<T>` instead of `Json<T>`:

```rust
// Before
pub async fn create_product(Json(data): Json<CreateProduct>) -> Result<...> { }

// After
pub async fn create_product(ValidJson(data): ValidJson<CreateProduct>) -> Result<...> { }
```

This ensures validation happens automatically before any business logic executes.

## Related Issues

- [#5](https://github.com/AgroVestOfficial/AgroVest-Backend/issues/5) - Prevent data corruption via invalid inputs
- [#9](https://github.com/AgroVestOfficial/AgroVest-Backend/issues/9) - Cross-field temporal validation for investments/proposals
