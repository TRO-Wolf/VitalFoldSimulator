# Mistake Analysis & Improvement Plan

## What I Did Wrong

When fixing Fix #1 (user.rs Handler), I made an **incorrect assumption about the data types and how Actix Web extractors work**.

### The Mistake

I used `web::Json<Claims>` as an extractor, which is fundamentally wrong because:
- `web::Json<T>` is an extractor that deserializes JSON from the **request body**
- The Claims are **not** in the request body
- The Claims are stored in **request extensions** by the jwt_validator middleware
- You cannot deserialize JWT claims from a JSON request body

### What I Should Have Understood

**Actix Web Extractors work in order**:
1. `HttpRequest` — Raw request, can access extensions manually
2. `web::Data<T>` — Shared application state from app_data()
3. `web::Path<T>` — URL path parameters
4. `web::Query<T>` — URL query string parameters
5. `web::Json<T>` — JSON request body

**For Claims from extensions**, you MUST:
- Use `HttpRequest` parameter
- Call `req.extensions().get::<Claims>()`
- Handle the Option result

### Why I Made This Mistake

I was thinking about the **signature** of the handler without considering **how extractors actually work in Actix Web**. The fix I attempted would try to parse Claims from the HTTP request body as JSON, which is incorrect.

### The Correct Fix

```rust
pub async fn me(
    req: HttpRequest,           // Raw request
    pool: web::Data<DbPool>,    // App data
) -> Result<HttpResponse, AppError> {
    // Extract Claims from request extensions
    let claims = req
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| AppError::Unauthorized("Auth required".to_string()))?
        .clone();

    // Rest of handler...
}
```

## How to Improve

### 1. Understand Actix Web Extractors Better
- Study the official Actix Web documentation on extractors
- Understand the order in which extractors are applied
- Know that `web::Json<T>` is specifically for deserializing JSON bodies
- Know that `req.extensions()` is the correct way to access data stored by middleware

### 2. Think About Data Flow
Before writing code, trace the data:
- **Where does the data originate?** (request body, headers, URL, middleware state)
- **How does it get there?** (deserialized, parsed, inserted)
- **How do I access it?** (which extractor or method)

### 3. Verify With Tests First
- Don't assume; write a test that demonstrates the behavior
- Test the actual mechanism: "Can I extract Claims from extensions?"
- Test failure cases: "What happens if Claims aren't in extensions?"

### 4. Review Middleware Documentation
- Understand exactly what the middleware does
- Understand what data it inserts and where
- Don't assume; read the middleware code

### 5. Use Compiler Feedback
- The type system should have warned me
- `web::Json<Claims>` is for request body deserialization
- If middleware inserts Claims into extensions, that's where they must come from

## Prevention Checklist

For the next fix, I will:
- [ ] Read the actual code path (middleware, handler, request flow)
- [ ] Verify data types and where data originates
- [ ] Check Actix Web documentation for correct extractor usage
- [ ] Consider the full request lifecycle
- [ ] Write down the data flow before coding
- [ ] Test the mechanism, not assume it works

## Applied to All Future Fixes

This same discipline applies to:
- **Fix #2**: Understand sqlx::PgPool vs deadpool::Pool type differences
- **Fix #3**: Validate input properly with correct error types
- **Fix #4**: Understand database constraint behavior
- **Fix #5**: Handle empty/whitespace validation correctly

**Key Principle**: When unsure about framework behavior, read the documentation and source code first, never guess.
