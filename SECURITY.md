# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in AgroVest Backend, please report it responsibly.

### How to Report

1. **Do NOT** open a public GitHub issue for security vulnerabilities
2. Email security concerns to: **[INSERT SECURITY EMAIL]**
3. Include the following information:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

### What to Expect

- Acknowledgment within 48 hours
- Status update within 1 week
- Fix timeline depends on severity

### Severity Levels

| Severity | Description | Response Time |
|----------|-------------|---------------|
| Critical | Remote code execution, SQL injection, auth bypass | Immediate |
| High | Data exposure, privilege escalation | 48 hours |
| Medium | Denial of service, information disclosure | 1 week |
| Low | Minor issues, best practices | 2 weeks |

## Security Best Practices

When deploying AgroVest Backend:

1. **Use strong secrets** — Generate a cryptographically secure `JWT_SECRET`
2. **Enable TLS** — Always use HTTPS in production
3. **Restrict CORS** — Set `CORS_ORIGINS` to your frontend domain only
4. **Secure database** — Use strong PostgreSQL credentials and network isolation
5. **Keep dependencies updated** — Run `cargo update` regularly
6. **Monitor logs** — Watch for suspicious authentication attempts

## Known Fixed Vulnerabilities

| ID | Endpoint | Class | Fixed in |
|----|----------|-------|----------|
| [#6](https://github.com/AgroVestOfficial/AgroVest-Backend/issues/6) | `GET /api/v1/escrows/{id}` | BOLA — unauthenticated access to financial records | `fix/escrow-bola-auth` |


## Dependencies

We regularly audit our dependencies for known vulnerabilities. To check locally:

```bash
cargo install cargo-audit
cargo audit
```
